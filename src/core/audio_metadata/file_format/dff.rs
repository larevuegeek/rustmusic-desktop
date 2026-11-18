//! DFF (Philips DSDIFF) file format parser.
//!
//! Reference: Philips, "DSD-Audio File Format Specification" (DSDIFF v1.5).
//!
//! Layout (all integers big-endian, chunks padded to even byte boundary):
//!
//! ```text
//! +----------------------------------------+
//! | "FRM8" (4) | size (8 BE) | form type   |
//! |   form type = "DSD " (4)               |
//! +----------------------------------------+
//! | sub-chunks within FRM8 body :          |
//! |   FVER  : version                      |
//! |   PROP  : container "SND ":            |
//! |     FS    : sample rate                |
//! |     CHNL  : channel count + IDs        |
//! |     CMPR  : compression type ("DSD ")  |
//! |     ABSS  : absolute start (optional)  |
//! |     LSCO  : loudspeaker config         |
//! |   DSD   : raw DSD audio data (or)      |
//! |   DST   : compressed DSD (unsupported) |
//! |   DIIN  : edited master info           |
//! |     DITI : title                       |
//! |     DIAR : artist                      |
//! |     DICP : copyright                   |
//! |   ID3   : optional ID3v2 (non-spec)    |
//! +----------------------------------------+
//! ```
//!
//! Audio data layout : interleaved per byte (NOT per block like DSF).
//! For stereo : `[ch0_b0, ch1_b0, ch0_b1, ch1_b1, ...]`.
//! Bit ordering : MSB-first (vs LSB-first for DSF).

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use crate::core::audio_metadata::extractor::error::ExtractError;
use crate::core::audio_metadata::tag_format::id3v2;
use crate::entity::audio::audio_file::{AudioFile, AudioFormat};
use crate::entity::audio::audio_tags::AudioTags;

// ─── Chunk identifiers ───────────────────────────────────────────────

const ID_FRM8: &[u8; 4] = b"FRM8";
const FORM_TYPE_DSD: &[u8; 4] = b"DSD ";
const ID_PROP: &[u8; 4] = b"PROP";
const PROP_TYPE_SND: &[u8; 4] = b"SND ";
const ID_FS: &[u8; 4] = b"FS  ";
const ID_CHNL: &[u8; 4] = b"CHNL";
const ID_CMPR: &[u8; 4] = b"CMPR";
const ID_DSD_DATA: &[u8; 4] = b"DSD ";
const ID_DST_DATA: &[u8; 4] = b"DST ";
const ID_DIIN: &[u8; 4] = b"DIIN";
const ID_DITI: &[u8; 4] = b"DITI";
const ID_DIAR: &[u8; 4] = b"DIAR";
const ID_DICP: &[u8; 4] = b"DICP";
const ID_ID3: &[u8; 4] = b"ID3 ";

/// Position and size of a chunk within the file.
#[derive(Debug, Clone, Copy)]
pub struct ChunkLoc {
    pub id: [u8; 4],
    /// Absolute file offset of the chunk's data (i.e. just after the 12-byte header).
    pub data_offset: u64,
    /// Size of the chunk's data in bytes (excludes the 12-byte header).
    pub data_size: u64,
}

/// Properties extracted from the PROP/SND container.
#[derive(Debug, Clone)]
pub struct DffProperties {
    pub sample_rate: u32,
    pub channel_count: u8,
    /// `b"DSD "` for raw DSD, `b"DST "` for DST-compressed.
    pub compression: [u8; 4],
}

// ─── Public API ──────────────────────────────────────────────────────

/// Parse the FRM8 header, return the absolute byte offset where sub-chunks start
/// and where they end.
pub fn parse_frm8_header(file: &mut File) -> Result<(u64, u64), ExtractError> {
    file.seek(SeekFrom::Start(0))?;
    let mut header = [0u8; 16];
    file.read_exact(&mut header)?;

    if &header[0..4] != ID_FRM8 {
        return Err(ExtractError::InvalidHeader(format!(
            "Expected FRM8 magic, got {:?}",
            &header[0..4]
        )));
    }
    let form_size = read_u64_be(&header[4..12]);
    let form_type = &header[12..16];
    if form_type != FORM_TYPE_DSD {
        return Err(ExtractError::InvalidHeader(format!(
            "Expected FORM type 'DSD ', got {:?}",
            form_type
        )));
    }
    // Sub-chunks region: after FRM8 header (12 bytes) + form type (4 bytes) = 16
    // until the FRM8 body ends at offset (12 + form_size).
    let start = 16u64;
    let end = 12u64 + form_size;
    Ok((start, end))
}

/// List every top-level chunk inside the FRM8 form.
pub fn list_top_chunks(file: &mut File) -> Result<Vec<ChunkLoc>, ExtractError> {
    let (start, end) = parse_frm8_header(file)?;
    list_chunks_in_range(file, start, end)
}

/// List every chunk between absolute offsets `start` and `end`.
/// Used for both top-level (FRM8 body) and nested (PROP, DIIN) chunks.
pub fn list_chunks_in_range(
    file: &mut File,
    start: u64,
    end: u64,
) -> Result<Vec<ChunkLoc>, ExtractError> {
    let mut chunks = Vec::new();
    let mut cursor = start;

    while cursor + 12 <= end {
        file.seek(SeekFrom::Start(cursor))?;
        let mut header = [0u8; 12];
        if file.read_exact(&mut header).is_err() {
            break;
        }
        let mut id = [0u8; 4];
        id.copy_from_slice(&header[0..4]);
        let size = read_u64_be(&header[4..12]);
        let data_offset = cursor + 12;

        // Bail out on obviously wrong sizes (e.g. corrupted file)
        if data_offset + size > end {
            log::warn!(
                "DFF: chunk {:?} at {} declares size {} that overflows form end {}",
                String::from_utf8_lossy(&id),
                cursor,
                size,
                end
            );
            break;
        }

        chunks.push(ChunkLoc {
            id,
            data_offset,
            data_size: size,
        });

        // Advance to next chunk, padding to even byte boundary
        let advance = 12 + size + (size % 2);
        cursor += advance;
    }

    Ok(chunks)
}

/// Parse the PROP/SND container at `prop_loc` and extract its FS / CHNL / CMPR sub-chunks.
pub fn parse_prop(file: &mut File, prop_loc: &ChunkLoc) -> Result<DffProperties, ExtractError> {
    // PROP data layout: 4-byte property type + sub-chunks
    if prop_loc.data_size < 4 {
        return Err(ExtractError::InvalidHeader(
            "DFF: PROP chunk too small".into(),
        ));
    }
    file.seek(SeekFrom::Start(prop_loc.data_offset))?;
    let mut prop_type = [0u8; 4];
    file.read_exact(&mut prop_type)?;
    if &prop_type != PROP_TYPE_SND {
        return Err(ExtractError::InvalidHeader(format!(
            "Expected PROP type 'SND ', got {:?}",
            prop_type
        )));
    }

    let sub_start = prop_loc.data_offset + 4;
    let sub_end = prop_loc.data_offset + prop_loc.data_size;
    let sub_chunks = list_chunks_in_range(file, sub_start, sub_end)?;

    let mut props = DffProperties {
        sample_rate: 0,
        channel_count: 0,
        compression: *FORM_TYPE_DSD,
    };

    for chunk in &sub_chunks {
        if &chunk.id == ID_FS {
            file.seek(SeekFrom::Start(chunk.data_offset))?;
            let mut buf = [0u8; 4];
            file.read_exact(&mut buf)?;
            props.sample_rate = u32::from_be_bytes(buf);
        } else if &chunk.id == ID_CHNL {
            file.seek(SeekFrom::Start(chunk.data_offset))?;
            let mut buf = [0u8; 2];
            file.read_exact(&mut buf)?;
            props.channel_count = u16::from_be_bytes(buf) as u8;
        } else if &chunk.id == ID_CMPR {
            file.seek(SeekFrom::Start(chunk.data_offset))?;
            let mut buf = [0u8; 4];
            file.read_exact(&mut buf)?;
            props.compression = buf;
        }
    }

    if props.sample_rate == 0 || props.channel_count == 0 {
        return Err(ExtractError::InvalidHeader(
            "DFF: PROP missing sample_rate or channel_count".into(),
        ));
    }

    Ok(props)
}

/// Public entry point : read a DFF file and return a fully-populated `AudioFile`.
pub fn extract(path: &Path) -> Result<AudioFile, ExtractError> {
    let mut file = File::open(path)?;
    let chunks = list_top_chunks(&mut file)?;

    // 1. PROP chunk → sample_rate, channel_count, compression
    let prop_loc = chunks
        .iter()
        .find(|c| &c.id == ID_PROP)
        .ok_or_else(|| ExtractError::InvalidHeader("DFF: missing PROP chunk".into()))?;
    let props = parse_prop(&mut file, prop_loc)?;

    if &props.compression != FORM_TYPE_DSD {
        return Err(ExtractError::InvalidTags(format!(
            "DFF compression {:?} not supported (only raw DSD)",
            String::from_utf8_lossy(&props.compression)
        )));
    }

    // 2. Audio data chunk
    let dsd_loc = chunks
        .iter()
        .find(|c| &c.id == ID_DSD_DATA)
        .ok_or_else(|| {
            if chunks.iter().any(|c| &c.id == ID_DST_DATA) {
                ExtractError::InvalidTags("DFF: DST-compressed audio not supported".into())
            } else {
                ExtractError::InvalidHeader("DFF: missing DSD audio chunk".into())
            }
        })?;

    let bytes_per_channel = dsd_loc.data_size / props.channel_count as u64;
    let sample_count_per_channel = bytes_per_channel * 8;
    let duration = sample_count_per_channel as f64 / props.sample_rate as f64;
    let bitrate = (props.sample_rate as u64 * props.channel_count as u64) / 1000;
    let file_size = std::fs::metadata(path)?.len();

    // 3. Tags : DIIN (native) + optional ID3 chunk (non-spec but common)
    let mut tags = AudioTags::new();
    if let Some(diin) = chunks.iter().find(|c| &c.id == ID_DIIN) {
        extract_diin_tags(&mut file, diin, &mut tags)?;
    }
    if let Some(id3) = chunks.iter().find(|c| &c.id == ID_ID3) {
        extract_id3_tags(&mut file, id3, &mut tags)?;
    }

    Ok(AudioFile {
        path: path.to_string_lossy().into_owned(),
        audio_format: AudioFormat::DFF,
        file_size,
        duration: duration as f32,
        bits_per_sample: 1,
        bitrate: bitrate as u32,
        sample_rate: props.sample_rate,
        channels: props.channel_count as usize,
        track_id: None,
        tags,
    })
}

// ─── Tag extraction ──────────────────────────────────────────────────

/// Read DITI/DIAR/DICP sub-chunks of a DIIN container.
/// Each text sub-chunk format: 2-byte BE size + UTF-8 text (no null terminator).
fn extract_diin_tags(
    file: &mut File,
    diin: &ChunkLoc,
    tags: &mut AudioTags,
) -> Result<(), ExtractError> {
    let sub_start = diin.data_offset;
    let sub_end = diin.data_offset + diin.data_size;
    let sub_chunks = list_chunks_in_range(file, sub_start, sub_end)?;

    for chunk in &sub_chunks {
        let text = match read_diin_text(file, chunk) {
            Ok(s) => s,
            Err(_) => continue,
        };
        if text.is_empty() {
            continue;
        }
        if &chunk.id == ID_DITI {
            tags.title = Some(text);
        } else if &chunk.id == ID_DIAR {
            tags.artist = Some(text);
        } else if &chunk.id == ID_DICP {
            tags.copyright = Some(text);
        }
    }
    Ok(())
}

/// DIIN text chunk : 2-byte BE length, then UTF-8 bytes.
fn read_diin_text(file: &mut File, chunk: &ChunkLoc) -> Result<String, ExtractError> {
    if chunk.data_size < 2 {
        return Ok(String::new());
    }
    file.seek(SeekFrom::Start(chunk.data_offset))?;
    let mut len_buf = [0u8; 2];
    file.read_exact(&mut len_buf)?;
    let text_len = u16::from_be_bytes(len_buf) as usize;

    if 2 + text_len as u64 > chunk.data_size {
        return Ok(String::new());
    }

    let mut text_buf = vec![0u8; text_len];
    file.read_exact(&mut text_buf)?;

    // DSDIFF spec doesn't mandate an encoding; UTF-8 is the safe default,
    // fallback to lossy conversion.
    Ok(String::from_utf8_lossy(&text_buf).into_owned())
}

/// Some taggers (e.g. dBpoweramp, foobar2000) write an ID3v2 chunk inside DFF
/// as a hors-spec extension. If present, parse it and merge into `tags`.
fn extract_id3_tags(
    file: &mut File,
    id3_chunk: &ChunkLoc,
    tags: &mut AudioTags,
) -> Result<(), ExtractError> {
    file.seek(SeekFrom::Start(id3_chunk.data_offset))?;
    let mut blob = vec![0u8; id3_chunk.data_size as usize];
    file.read_exact(&mut blob)?;

    if blob.len() < 10 || &blob[0..3] != b"ID3" {
        return Ok(());
    }

    let id3_tag = id3v2::parse(&blob)?;
    let id3_tags = id3v2::to_audio_tags(&id3_tag);

    // Merge ID3 fields over DIIN ones (ID3 is generally richer when present)
    if id3_tags.title.is_some() {
        tags.title = id3_tags.title;
    }
    if id3_tags.artist.is_some() {
        tags.artist = id3_tags.artist;
    }
    if id3_tags.album.is_some() {
        tags.album = id3_tags.album;
    }
    if id3_tags.album_artist.is_some() {
        tags.album_artist = id3_tags.album_artist;
    }
    if id3_tags.year.is_some() {
        tags.year = id3_tags.year;
    }
    if id3_tags.genre.is_some() {
        tags.genre = id3_tags.genre;
    }
    if id3_tags.track_number.is_some() {
        tags.track_number = id3_tags.track_number;
    }
    if id3_tags.total_tracks.is_some() {
        tags.total_tracks = id3_tags.total_tracks;
    }
    if id3_tags.disc_number.is_some() {
        tags.disc_number = id3_tags.disc_number;
    }
    if id3_tags.composer.is_some() {
        tags.composer = id3_tags.composer;
    }
    if id3_tags.bpm.is_some() {
        tags.bpm = id3_tags.bpm;
    }
    if id3_tags.rating.is_some() {
        tags.rating = id3_tags.rating;
    }
    if id3_tags.copyright.is_some() {
        tags.copyright = id3_tags.copyright;
    }
    if !id3_tags.attached_images.is_empty() {
        tags.attached_images = id3_tags.attached_images;
    }
    tags.id3_version = id3_tags.id3_version;

    Ok(())
}

// ─── Helpers ─────────────────────────────────────────────────────────

fn read_u64_be(bytes: &[u8]) -> u64 {
    u64::from_be_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ])
}
