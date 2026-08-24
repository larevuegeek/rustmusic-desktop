//! ID3v2 tag parser (versions 2.3 and 2.4).
//!
//! Used by MP3, DSF, AIFF and WAV containers. Operates on a raw byte
//! slice so it doesn't care about the surrounding container.
//!
//! References:
//!   - https://id3.org/id3v2.3.0
//!   - https://id3.org/id3v2.4.0-structure
//!   - https://id3.org/id3v2.4.0-frames

use base64::Engine;

use crate::core::audio_metadata::extractor::error::ExtractError;
use crate::entity::audio::audio_tags::{AttachedImage, AudioTags, ImageType};
use crate::helper::string::string::normalize_year;

// ─── Public types ────────────────────────────────────────────────────

/// A parsed ID3v2 tag (header + frames).
pub struct Id3v2Tag {
    /// 3 for ID3v2.3, 4 for ID3v2.4.
    pub version: u8,
    pub frames: Vec<Id3v2Frame>,
}

/// A single ID3v2 frame: a 4-byte ID + a typed payload.
pub struct Id3v2Frame {
    /// Raw 4-byte frame ID, e.g. `*b"TIT2"`, `*b"APIC"`, `*b"COMM"`.
    pub id: [u8; 4],
    pub kind: Id3v2FrameKind,
}

/// Decoded payload of a frame.
pub enum Id3v2FrameKind {
    /// Standard text frame (TIT2, TPE1, TALB, TYER, TDRC, TCON, ...).
    Text(String),

    /// Comment frame (COMM).
    Comment {
        language: [u8; 3],
        description: String,
        text: String,
    },

    /// Attached picture (APIC) - cover art etc.
    Picture {
        mime_type: String,
        picture_type: u8,
        description: String,
        data: Vec<u8>,
    },

    /// User-defined text (TXXX) - typically for non-standard tags.
    UserText { description: String, value: String },

    /// Popularimeter (POPM): rating 0-255 + play counter.
    Popularimeter {
        email: String,
        rating: u8,
        counter: Vec<u8>,
    },

    /// URL frames (WXXX, WOAR, ...).
    Url(String),

    /// Frame we did not decode - kept as raw bytes for future extension.
    Unknown(Vec<u8>),
}

// ─── Public API ──────────────────────────────────────────────────────

/// Parse an ID3v2 blob.
///
/// `blob` must start with the 10-byte ID3v2 header (`b"ID3"` + version + flags + size).
/// Bytes past the declared tag size are ignored.
pub fn parse(blob: &[u8]) -> Result<Id3v2Tag, ExtractError> {
    // 1. Header
    if blob.len() < 10 {
        return Err(ExtractError::Truncated);
    }
    if &blob[0..3] != b"ID3" {
        return Err(ExtractError::InvalidTags("Missing ID3 magic".into()));
    }
    let version = blob[3];
    if version != 3 && version != 4 {
        return Err(ExtractError::InvalidTags(format!(
            "Unsupported ID3v2 version: 2.{}",
            version
        )));
    }
    let flags = blob[5];
    let unsync_global = flags & 0b1000_0000 != 0;
    let extended_header = flags & 0b0100_0000 != 0;

    let declared_size = read_synchsafe(&blob[6..10]) as usize;
    let tag_end = 10usize
        .checked_add(declared_size)
        .ok_or(ExtractError::Truncated)?;
    if tag_end > blob.len() {
        return Err(ExtractError::Truncated);
    }

    // 2. Tag body, undoing global unsync if requested (v2.3-style)
    let owned_body: Vec<u8>;
    let body: &[u8] = if unsync_global {
        owned_body = unsynchronise(&blob[10..tag_end]);
        &owned_body
    } else {
        &blob[10..tag_end]
    };

    let mut pos = 0usize;

    // 3. Skip extended header if present (we don't use it for read-only)
    if extended_header {
        if body.len() < pos + 4 {
            return Err(ExtractError::InvalidTags(
                "Truncated extended header".into(),
            ));
        }
        let ext_size = if version == 4 {
            // v2.4: synchsafe size INCLUDES the 4 size bytes
            read_synchsafe(&body[pos..pos + 4]) as usize
        } else {
            // v2.3: regular u32 BE, size EXCLUDES the 4 size bytes (per spec)
            u32::from_be_bytes([body[pos], body[pos + 1], body[pos + 2], body[pos + 3]]) as usize
                + 4
        };
        if pos + ext_size > body.len() {
            return Err(ExtractError::InvalidTags("Extended header overflow".into()));
        }
        pos += ext_size;
    }

    // 4. Frame loop
    let mut frames = Vec::new();
    while pos + 10 <= body.len() {
        // First byte 0 means we hit padding -> done
        if body[pos] == 0 {
            break;
        }

        let mut id = [0u8; 4];
        id.copy_from_slice(&body[pos..pos + 4]);
        pos += 4;

        let frame_size = if version == 4 {
            read_synchsafe(&body[pos..pos + 4]) as usize
        } else {
            u32::from_be_bytes([body[pos], body[pos + 1], body[pos + 2], body[pos + 3]]) as usize
        };
        pos += 4;

        let _frame_flags = u16::from_be_bytes([body[pos], body[pos + 1]]);
        pos += 2;

        // Frame body: graceful break if overflow (some real files have garbage trailing)
        if pos + frame_size > body.len() {
            break;
        }
        let frame_body = &body[pos..pos + frame_size];
        pos += frame_size;

        let kind = decode_frame(&id, frame_body);
        frames.push(Id3v2Frame { id, kind });
    }

    Ok(Id3v2Tag { version, frames })
}

/// Map an `Id3v2Tag` into the universal `AudioTags` structure.
///
/// Frames not mapped to a typed field are pushed into `custom_tags` so
/// they remain visible to the UI / database.
pub fn to_audio_tags(tag: &Id3v2Tag) -> AudioTags {
    let mut tags = AudioTags::new();
    tags.id3_version = Some(format!("ID3v2.{}", tag.version));

    for frame in &tag.frames {
        match (&frame.id, &frame.kind) {
            // ─── Identité du morceau ────────────────────────────────
            (b"TIT2", Id3v2FrameKind::Text(v)) => tags.title = Some(v.clone()),
            (b"TIT3", Id3v2FrameKind::Text(v)) => tags.subtitle = Some(v.clone()),

            // ─── Artistes ───────────────────────────────────────────
            (b"TPE1", Id3v2FrameKind::Text(v)) => {
                tags.artist = Some(append_multi(tags.artist.take(), v));
            }
            (b"TPE2", Id3v2FrameKind::Text(v)) => {
                tags.album_artist = Some(append_multi(tags.album_artist.take(), v));
            }
            (b"TPE3", Id3v2FrameKind::Text(v)) => tags.conductor = Some(v.clone()),
            (b"TPE4", Id3v2FrameKind::Text(v)) => tags.remix_artist = Some(v.clone()),
            (b"TOPE", Id3v2FrameKind::Text(v)) => tags.original_artist = Some(v.clone()),
            (b"TCOM", Id3v2FrameKind::Text(v)) => tags.composer = Some(v.clone()),
            (b"TEXT", Id3v2FrameKind::Text(v)) => tags.lyricist = Some(v.clone()),

            // ─── Album / classement ─────────────────────────────────
            (b"TALB", Id3v2FrameKind::Text(v)) => tags.album = Some(v.clone()),
            (b"TCON", Id3v2FrameKind::Text(v)) => tags.genre = Some(clean_genre(v)),
            (b"TDRC", Id3v2FrameKind::Text(v)) | (b"TYER", Id3v2FrameKind::Text(v)) => {
                tags.year = normalize_year(Some(v.clone()));
            }
            (b"TRCK", Id3v2FrameKind::Text(v)) => {
                let (n, total) = parse_slash_pair(v);
                tags.track_number = n;
                tags.total_tracks = total;
            }
            (b"TPOS", Id3v2FrameKind::Text(v)) => {
                let (n, total) = parse_slash_pair(v);
                tags.disc_number = n;
                tags.total_discs = total;
            }
            (b"TBPM", Id3v2FrameKind::Text(v)) => tags.bpm = Some(v.clone()),
            (b"TKEY", Id3v2FrameKind::Text(v)) => tags.key = Some(v.clone()),
            (b"TMOO", Id3v2FrameKind::Text(v)) => tags.mood = Some(v.clone()),
            (b"TSRC", Id3v2FrameKind::Text(v)) => tags.isrc = Some(v.clone()),
            (b"TLAN", Id3v2FrameKind::Text(v)) => tags.language = Some(v.clone()),
            (b"TMED", Id3v2FrameKind::Text(v)) => tags.media_type = Some(v.clone()),
            (b"TFLT", Id3v2FrameKind::Text(v)) => tags.file_type = Some(v.clone()),
            (b"TCMP", Id3v2FrameKind::Text(v)) => {
                tags.compilation = Some(v.trim() == "1");
            }

            // ─── Production / publication ───────────────────────────
            (b"TPUB", Id3v2FrameKind::Text(v)) => tags.publisher = Some(v.clone()),
            (b"TCOP", Id3v2FrameKind::Text(v)) => tags.copyright = Some(v.clone()),
            (b"TENC", Id3v2FrameKind::Text(v)) => tags.encoded_by = Some(v.clone()),
            (b"TSSE", Id3v2FrameKind::Text(v)) => tags.encoding_settings = Some(v.clone()),
            (b"TRSN", Id3v2FrameKind::Text(v)) => {
                tags.internet_radio_station_name = Some(v.clone())
            }
            (b"TRSO", Id3v2FrameKind::Text(v)) => {
                tags.internet_radio_station_owner = Some(v.clone())
            }

            // ─── Commentaire / paroles ──────────────────────────────
            (b"COMM", Id3v2FrameKind::Comment { description, text, .. }) => {
                // Filter out iTunes-specific technical comments
                if !is_itunes_internal_comment(description) && tags.comment.is_none() {
                    tags.comment = Some(text.clone());
                }
            }
            (b"USLT", Id3v2FrameKind::Comment { text, .. }) => {
                tags.unsynchronised_lyrics = Some(text.clone());
            }

            // ─── Notation ───────────────────────────────────────────
            (b"POPM", Id3v2FrameKind::Popularimeter { rating, .. }) => {
                tags.rating = Some(popm_to_stars(*rating));
            }

            // ─── URLs typées ────────────────────────────────────────
            (b"WOAF", Id3v2FrameKind::Url(v)) => tags.official_audio_file_url = Some(v.clone()),
            (b"WOAR", Id3v2FrameKind::Url(v)) => tags.official_artist_url = Some(v.clone()),
            (b"WOAS", Id3v2FrameKind::Url(v)) => tags.official_audio_source_url = Some(v.clone()),
            (b"WPAY", Id3v2FrameKind::Url(v)) => tags.payment_url = Some(v.clone()),
            (b"WPUB", Id3v2FrameKind::Url(v)) => tags.publisher_url = Some(v.clone()),

            // ─── Cover art ──────────────────────────────────────────
            (b"APIC", Id3v2FrameKind::Picture { mime_type, picture_type, description, data }) => {
                tags.attached_images.push(AttachedImage {
                    image_type: Some(apic_type_to_image_type(*picture_type)),
                    mime_type: mime_type.clone(),
                    description: if description.is_empty() { None } else { Some(description.clone()) },
                    image_src: format!(
                        "data:{};base64,{}",
                        mime_type,
                        base64::engine::general_purpose::STANDARD.encode(data)
                    ),
                    image_data: data.clone(),
                });
            }

            // ─── Fallback : custom_tags ─────────────────────────────
            (b"TXXX", Id3v2FrameKind::UserText { description, value }) => {
                tags.custom_tags.push((description.clone(), value.clone()));
            }
            (b"WXXX", Id3v2FrameKind::Url(url)) => {
                tags.custom_tags.push(("WXXX".into(), url.clone()));
            }
            (id, Id3v2FrameKind::Text(v)) => {
                tags.custom_tags.push((id_to_string(id), v.clone()));
            }
            (id, Id3v2FrameKind::Url(v)) => {
                tags.custom_tags.push((id_to_string(id), v.clone()));
            }
            _ => {} // Unknown frames silently ignored
        }
    }

    tags
}

// ─── Frame decoders ──────────────────────────────────────────────────

fn decode_frame(id: &[u8; 4], body: &[u8]) -> Id3v2FrameKind {
    match id {
        b"TXXX" => decode_txxx(body),
        b"APIC" => decode_apic(body),
        b"COMM" | b"USLT" => decode_comm_or_uslt(body),
        b"POPM" => decode_popm(body),
        b"WXXX" => decode_wxxx(body),
        _ if id[0] == b'T' => decode_text(body),
        _ if id[0] == b'W' => decode_url_simple(body),
        _ => Id3v2FrameKind::Unknown(body.to_vec()),
    }
}

/// Generic text frame (T*** except TXXX).
fn decode_text(body: &[u8]) -> Id3v2FrameKind {
    if body.is_empty() {
        return Id3v2FrameKind::Text(String::new());
    }
    let encoding = body[0];
    let text = decode_string(encoding, &body[1..]);
    Id3v2FrameKind::Text(strip_trailing_nulls(text))
}

/// User-defined text frame (TXXX).
fn decode_txxx(body: &[u8]) -> Id3v2FrameKind {
    if body.is_empty() {
        return Id3v2FrameKind::Unknown(body.to_vec());
    }
    let encoding = body[0];
    let rest = &body[1..];
    match split_at_null(encoding, rest) {
        Some((desc, value)) => Id3v2FrameKind::UserText {
            description: decode_string(encoding, desc),
            value: strip_trailing_nulls(decode_string(encoding, value)),
        },
        None => Id3v2FrameKind::UserText {
            description: String::new(),
            value: strip_trailing_nulls(decode_string(encoding, rest)),
        },
    }
}

/// Attached picture (APIC).
fn decode_apic(body: &[u8]) -> Id3v2FrameKind {
    if body.is_empty() {
        return Id3v2FrameKind::Unknown(body.to_vec());
    }
    let encoding = body[0];
    let rest = &body[1..];

    // MIME type: always ISO-8859-1, null-terminated
    let (mime_bytes, rest) = match split_at_null(0, rest) {
        Some(p) => p,
        None => return Id3v2FrameKind::Unknown(body.to_vec()),
    };
    let mime_type: String = mime_bytes.iter().map(|&b| b as char).collect();

    // Picture type: 1 byte
    if rest.is_empty() {
        return Id3v2FrameKind::Unknown(body.to_vec());
    }
    let picture_type = rest[0];
    let rest = &rest[1..];

    // Description: in given encoding, null-terminated
    let (desc_bytes, data) = match split_at_null(encoding, rest) {
        Some(p) => p,
        None => return Id3v2FrameKind::Unknown(body.to_vec()),
    };
    let description = decode_string(encoding, desc_bytes);

    Id3v2FrameKind::Picture {
        mime_type,
        picture_type,
        description,
        data: data.to_vec(),
    }
}

/// Comment (COMM) or unsynchronised lyrics (USLT) - same structure.
fn decode_comm_or_uslt(body: &[u8]) -> Id3v2FrameKind {
    if body.len() < 4 {
        return Id3v2FrameKind::Unknown(body.to_vec());
    }
    let encoding = body[0];
    let mut language = [0u8; 3];
    language.copy_from_slice(&body[1..4]);
    let rest = &body[4..];

    let (desc_bytes, text_bytes) = match split_at_null(encoding, rest) {
        Some(p) => p,
        None => (rest, &[][..]),
    };

    Id3v2FrameKind::Comment {
        language,
        description: decode_string(encoding, desc_bytes),
        text: strip_trailing_nulls(decode_string(encoding, text_bytes)),
    }
}

/// Popularimeter (POPM): email + rating(1B) + counter(N B).
fn decode_popm(body: &[u8]) -> Id3v2FrameKind {
    let (email_bytes, rest) = match split_at_null(0, body) {
        Some(p) => p,
        None => return Id3v2FrameKind::Unknown(body.to_vec()),
    };
    if rest.is_empty() {
        return Id3v2FrameKind::Unknown(body.to_vec());
    }
    let email: String = email_bytes.iter().map(|&b| b as char).collect();
    Id3v2FrameKind::Popularimeter {
        email,
        rating: rest[0],
        counter: rest[1..].to_vec(),
    }
}

/// User-defined URL (WXXX).
fn decode_wxxx(body: &[u8]) -> Id3v2FrameKind {
    if body.is_empty() {
        return Id3v2FrameKind::Url(String::new());
    }
    let encoding = body[0];
    let rest = &body[1..];
    let url_bytes = match split_at_null(encoding, rest) {
        Some((_desc, url)) => url,
        None => rest,
    };
    let url: String = url_bytes
        .iter()
        .take_while(|&&b| b != 0)
        .map(|&b| b as char)
        .collect();
    Id3v2FrameKind::Url(url)
}

/// Simple URL frame (W*** except WXXX) - just a URL in ISO-8859-1.
fn decode_url_simple(body: &[u8]) -> Id3v2FrameKind {
    let url: String = body
        .iter()
        .take_while(|&&b| b != 0)
        .map(|&b| b as char)
        .collect();
    Id3v2FrameKind::Url(url)
}

// ─── Low-level helpers ───────────────────────────────────────────────

/// Decode a 28-bit synchsafe integer from 4 bytes.
fn read_synchsafe(bytes: &[u8]) -> u32 {
    ((bytes[0] as u32 & 0x7F) << 21)
        | ((bytes[1] as u32 & 0x7F) << 14)
        | ((bytes[2] as u32 & 0x7F) << 7)
        | (bytes[3] as u32 & 0x7F)
}

/// Undo ID3v2 unsynchronisation: drop the 0x00 inserted after every 0xFF.
fn unsynchronise(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        out.push(bytes[i]);
        if bytes[i] == 0xFF && i + 1 < bytes.len() && bytes[i + 1] == 0x00 {
            i += 2;
        } else {
            i += 1;
        }
    }
    out
}

/// Decode a string given its ID3v2 encoding byte.
///
/// Encodings:
///   - 0: ISO-8859-1 (Latin-1)
///   - 1: UTF-16 with BOM
///   - 2: UTF-16BE without BOM (v2.4 only)
///   - 3: UTF-8 (v2.4 only)
fn decode_string(encoding: u8, bytes: &[u8]) -> String {
    match encoding {
        0 => bytes.iter().map(|&b| b as char).collect(),
        1 => decode_utf16_with_bom(bytes),
        2 => decode_utf16_be(bytes),
        3 => String::from_utf8_lossy(bytes).into_owned(),
        // Unknown encoding: treat as Latin-1 to preserve bytes
        _ => bytes.iter().map(|&b| b as char).collect(),
    }
}

fn decode_utf16_with_bom(bytes: &[u8]) -> String {
    if bytes.len() < 2 {
        return String::new();
    }
    match (bytes[0], bytes[1]) {
        (0xFE, 0xFF) => decode_utf16_be(&bytes[2..]),
        (0xFF, 0xFE) => decode_utf16_le(&bytes[2..]),
        // No BOM: spec says BE by default
        _ => decode_utf16_be(bytes),
    }
}

fn decode_utf16_be(bytes: &[u8]) -> String {
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|c| u16::from_be_bytes([c[0], c[1]]))
        .collect();
    String::from_utf16_lossy(&units)
}

fn decode_utf16_le(bytes: &[u8]) -> String {
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    String::from_utf16_lossy(&units)
}

/// Split `bytes` on the first null terminator appropriate for the given encoding.
/// UTF-16 needs an aligned double-null terminator.
fn split_at_null(encoding: u8, bytes: &[u8]) -> Option<(&[u8], &[u8])> {
    match encoding {
        1 | 2 => {
            // UTF-16: aligned double 0x00
            let mut i = 0;
            while i + 1 < bytes.len() {
                if bytes[i] == 0 && bytes[i + 1] == 0 {
                    return Some((&bytes[..i], &bytes[i + 2..]));
                }
                i += 2;
            }
            None
        }
        _ => bytes
            .iter()
            .position(|&b| b == 0)
            .map(|p| (&bytes[..p], &bytes[p + 1..])),
    }
}

/// Trim trailing null characters that some encoders leave at end of strings.
fn strip_trailing_nulls(s: String) -> String {
    s.trim_end_matches('\0').to_string()
}

// ─── AudioTags mapping helpers ───────────────────────────────────────

/// Append a new value to a multi-value field (artist, album_artist) using
/// `; ` as separator. Mirrors what audio_analyser does for Vorbis tags.
fn append_multi(existing: Option<String>, new: &str) -> String {
    match existing {
        Some(prev) if !prev.is_empty() => format!("{prev}; {new}"),
        _ => new.to_string(),
    }
}

/// Parse a `"3/12"` style pair → `(Some(3), Some(12))`.
/// `"3"` → `(Some(3), None)`. Anything unparseable → `(None, None)`.
fn parse_slash_pair(s: &str) -> (Option<u16>, Option<u16>) {
    let mut parts = s.split('/');
    let first = parts.next().and_then(|p| p.trim().parse::<u16>().ok());
    let second = parts.next().and_then(|p| p.trim().parse::<u16>().ok());
    (first, second)
}

/// Strip ID3v1 genre numeric prefixes like `(17)Rock` → `Rock`,
/// `(17)` (alone) → `Rock` via lookup, `(RX)` → `Remix`, `(CR)` → `Cover`.
fn clean_genre(raw: &str) -> String {
    let trimmed = raw.trim();
    if !trimmed.starts_with('(') {
        return trimmed.to_string();
    }
    // Strip leading parenthesised tokens
    let mut rest = trimmed;
    let mut last_resolved: Option<&str> = None;
    while let Some(stripped) = rest.strip_prefix('(') {
        if let Some(close) = stripped.find(')') {
            let token = &stripped[..close];
            // Escaped "((Foo)" - the genre actually starts with '('
            if token.is_empty() {
                rest = &stripped[close + 1..];
                break;
            }
            last_resolved = match token {
                "RX" => Some("Remix"),
                "CR" => Some("Cover"),
                num => num.parse::<u8>().ok().and_then(id3v1_genre_name),
            };
            rest = &stripped[close + 1..];
        } else {
            break;
        }
    }
    let rest = rest.trim();
    if rest.is_empty() {
        last_resolved.unwrap_or(trimmed).to_string()
    } else {
        rest.to_string()
    }
}

/// ID3v1 genre table (a subset is enough — covers the genres still seen in real files).
fn id3v1_genre_name(n: u8) -> Option<&'static str> {
    const GENRES: &[(u8, &str)] = &[
        (0, "Blues"), (1, "Classic Rock"), (2, "Country"), (3, "Dance"),
        (4, "Disco"), (5, "Funk"), (6, "Grunge"), (7, "Hip-Hop"),
        (8, "Jazz"), (9, "Metal"), (10, "New Age"), (11, "Oldies"),
        (12, "Other"), (13, "Pop"), (14, "R&B"), (15, "Rap"),
        (16, "Reggae"), (17, "Rock"), (18, "Techno"), (19, "Industrial"),
        (20, "Alternative"), (21, "Ska"), (22, "Death Metal"), (23, "Pranks"),
        (24, "Soundtrack"), (25, "Euro-Techno"), (26, "Ambient"), (27, "Trip-Hop"),
        (28, "Vocal"), (29, "Jazz+Funk"), (30, "Fusion"), (31, "Trance"),
        (32, "Classical"), (33, "Instrumental"), (34, "Acid"), (35, "House"),
        (36, "Game"), (37, "Sound Clip"), (38, "Gospel"), (39, "Noise"),
        (40, "AlternRock"), (41, "Bass"), (42, "Soul"), (43, "Punk"),
        (44, "Space"), (45, "Meditative"), (46, "Instrumental Pop"),
        (47, "Instrumental Rock"), (48, "Ethnic"), (49, "Gothic"),
        (50, "Darkwave"), (51, "Techno-Industrial"), (52, "Electronic"),
        (53, "Pop-Folk"), (54, "Eurodance"), (55, "Dream"), (56, "Southern Rock"),
        (57, "Comedy"), (58, "Cult"), (59, "Gangsta"), (60, "Top 40"),
        (61, "Christian Rap"), (62, "Pop/Funk"), (63, "Jungle"),
        (64, "Native American"), (65, "Cabaret"), (66, "New Wave"),
        (67, "Psychedelic"), (68, "Rave"), (69, "Showtunes"), (70, "Trailer"),
        (71, "Lo-Fi"), (72, "Tribal"), (73, "Acid Punk"), (74, "Acid Jazz"),
        (75, "Polka"), (76, "Retro"), (77, "Musical"), (78, "Rock & Roll"),
        (79, "Hard Rock"), (80, "Folk"), (81, "Folk-Rock"), (82, "National Folk"),
    ];
    GENRES.iter().find(|(k, _)| *k == n).map(|(_, v)| *v)
}

/// Traduit l'octet POPM (0-255) en étoiles, par pas d'un demi.
///
/// # Pourquoi ces bornes
/// POPM n'a pas de barème normalisé : chaque logiciel a le sien. Les valeurs
/// pivots retenues — 1, 64, 128, 196, 255 — sont celles qu'écrit Windows Media
/// Player, devenues l'usage de fait ; les demi-crans reprennent ceux de
/// MediaMonkey, seul logiciel répandu à les écrire.
///
/// Les intervalles restent larges de part et d'autre de chaque pivot, pour que
/// la note d'un logiciel au barème légèrement différent tombe quand même sur
/// le bon cran.
fn popm_to_stars(rating: u8) -> f64 {
    match rating {
        0 => 0.0,
        1..=17 => 1.0,
        18..=41 => 1.5,
        42..=79 => 2.0,
        80..=105 => 2.5,
        106..=143 => 3.0,
        144..=173 => 3.5,
        174..=209 => 4.0,
        210..=239 => 4.5,
        _ => 5.0,
    }
}

/// Convert an APIC picture type byte (0-20) to our internal `ImageType`.
fn apic_type_to_image_type(t: u8) -> ImageType {
    match t {
        0x01 => ImageType::Icon32x32PNG,
        0x02 => ImageType::IconOther,
        0x03 => ImageType::CoverFront,
        0x04 => ImageType::CoverBack,
        0x05 => ImageType::LeafletPage,
        0x06 => ImageType::MediaLabel,
        0x07 => ImageType::LeadArtist,
        0x08 => ImageType::Artist,
        0x09 => ImageType::Conductor,
        0x0A => ImageType::BandOrchestra,
        0x0B => ImageType::Composer,
        0x0C => ImageType::LyricistTextWriter,
        0x0D => ImageType::RecordingLocation,
        0x0E => ImageType::DuringRecording,
        0x0F => ImageType::DuringPerformance,
        0x10 => ImageType::MovieVideoScreenCapture,
        0x11 => ImageType::ABrightColouredFish,
        0x12 => ImageType::Illustration,
        0x13 => ImageType::BandArtistLogo,
        0x14 => ImageType::PublisherStudioLogo,
        _ => ImageType::Other,
    }
}

/// COMM frames whose description is one of these are iTunes/Apple
/// internal markers (volume normalisation, CDDB id, etc.) — we skip them.
fn is_itunes_internal_comment(description: &str) -> bool {
    matches!(
        description,
        "iTunNORM"
            | "iTunSMPB"
            | "iTunes_CDDB_1"
            | "iTunes_CDDB_TrackNumber"
            | "iTunPGAP"
            | "iTunMOVI"
            | "iTunEXTC"
            | "iTunEXTI"
    )
}

/// Convert a 4-byte frame ID to a printable string for `custom_tags`.
fn id_to_string(id: &[u8; 4]) -> String {
    id.iter().map(|&b| b as char).collect()
}

#[cfg(test)]
mod tests_popm {
    use super::popm_to_stars;

    #[test]
    fn les_pivots_de_windows_media_player_tombent_juste() {
        // Ces cinq valeurs sont celles qu'écrit WMP : elles doivent rendre
        // exactement 1 à 5 étoiles pleines.
        assert_eq!(popm_to_stars(1), 1.0);
        assert_eq!(popm_to_stars(64), 2.0);
        assert_eq!(popm_to_stars(128), 3.0);
        assert_eq!(popm_to_stars(196), 4.0);
        assert_eq!(popm_to_stars(255), 5.0);
    }

    #[test]
    fn les_demi_crans_de_mediamonkey_tombent_juste() {
        assert_eq!(popm_to_stars(31), 1.5);
        assert_eq!(popm_to_stars(96), 2.5);
        assert_eq!(popm_to_stars(160), 3.5);
        assert_eq!(popm_to_stars(224), 4.5);
    }

    #[test]
    fn zero_reste_zero() {
        // Zéro signifie « pas de note » dans POPM, et ne doit surtout pas
        // devenir une demi-étoile.
        assert_eq!(popm_to_stars(0), 0.0);
    }

    #[test]
    fn chaque_note_tombe_sur_un_demi_cran_exact() {
        // Le doublement doit rendre un entier, sans reste : c'est ce qui
        // garantit que les comparaisons d'égalité restent exactes malgré le
        // flottant. Vrai ici parce que les dénominateurs sont des puissances
        // de deux.
        for octet in 0..=255u8 {
            let note = popm_to_stars(octet);
            assert_eq!(note * 2.0, (note * 2.0).trunc(), "cran bâtard à {octet}");
        }
    }

    #[test]
    fn le_bareme_ne_recule_jamais() {
        // Une note plus haute dans le fichier ne peut pas donner moins
        // d'étoiles : sans ça, un tri par note deviendrait incohérent.
        let mut precedent = 0.0;
        for octet in 0..=255u8 {
            let actuel = popm_to_stars(octet);
            assert!(
                actuel >= precedent,
                "recul à {octet} : {precedent} puis {actuel}"
            );
            assert!((0.0..=5.0).contains(&actuel), "hors bornes à {octet}");
            precedent = actuel;
        }
    }
}
