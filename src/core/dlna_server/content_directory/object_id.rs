//! Encoding/decoding of DLNA object IDs.
//!
//! Object IDs are opaque strings that the amp echoes back in `Browse` to
//! navigate the content tree. We use a structured grammar so each ID is
//! self-describing :
//!
//! ```text
//! 0                                       (root)
//! 0/lib/<lib_id>                          (a library — auto-collapsed when only one)
//! 0/lib/<lib_id>/artists                  (Artists list)
//! 0/lib/<lib_id>/artists/<artist_id>      (one artist's albums)
//! 0/lib/<lib_id>/albums                   (Albums list)
//! 0/lib/<lib_id>/albums/<album_id>        (one album's tracks)
//! 0/lib/<lib_id>/folders                  (filesystem roots)
//! 0/lib/<lib_id>/folders/<base64-path>    (a filesystem folder's contents)
//! ```
//!
//! Track-leaf IDs (`.../tracks/<tid>`) are not browsable — they're items,
//! not containers — so the parser returns `Unknown` for them.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;

/// Parsed shape of an object ID.
#[derive(Debug)]
pub enum ObjectId {
    Root,
    Library(i64),
    Artists(i64),
    Artist(i64, String),
    Albums(i64),
    Album(i64, String),
    Folders(i64),
    Folder(i64, String),
    /// Catch-all : malformed, unknown subtree, or item-leaf.
    Unknown,
}

/// Parse an object ID string into a structured `ObjectId`. Never fails — a
/// malformed input becomes `ObjectId::Unknown` and produces an empty Browse
/// response upstream.
pub fn parse_object_id(id: &str) -> ObjectId {
    if id == "0" || id.is_empty() {
        return ObjectId::Root;
    }
    let Some(rest) = id.strip_prefix("0/lib/") else {
        return ObjectId::Unknown;
    };

    // rest = "<lib_id>" or "<lib_id>/<sub>..."
    let (lib_str, tail) = match rest.split_once('/') {
        Some((l, t)) => (l, Some(t)),
        None => (rest, None),
    };
    let Ok(lib_id) = lib_str.parse::<i64>() else {
        return ObjectId::Unknown;
    };
    let Some(tail) = tail else {
        return ObjectId::Library(lib_id);
    };

    match tail {
        "artists" => return ObjectId::Artists(lib_id),
        "albums" => return ObjectId::Albums(lib_id),
        "folders" => return ObjectId::Folders(lib_id),
        _ => {}
    }

    if let Some(rest) = tail.strip_prefix("artists/") {
        return ObjectId::Artist(lib_id, rest.to_string());
    }
    if let Some(rest) = tail.strip_prefix("albums/") {
        // ".../tracks/<tid>" is a leaf item, not browsable.
        if rest.contains("/tracks/") {
            return ObjectId::Unknown;
        }
        return ObjectId::Album(lib_id, rest.to_string());
    }
    if let Some(rest) = tail.strip_prefix("folders/") {
        if rest.contains("/tracks/") {
            return ObjectId::Unknown;
        }
        return match decode_path(rest) {
            Some(path) => ObjectId::Folder(lib_id, path),
            None => ObjectId::Unknown,
        };
    }
    ObjectId::Unknown
}

/// Encode an absolute filesystem path into a URL-safe base64 string for use
/// inside a `0/lib/<id>/folders/<here>` object ID.
pub fn encode_path(path: &str) -> String {
    URL_SAFE_NO_PAD.encode(path.as_bytes())
}

/// Decode the base64 path back. Returns `None` if the input isn't valid
/// URL-safe base64 or the bytes aren't valid UTF-8.
pub fn decode_path(encoded: &str) -> Option<String> {
    URL_SAFE_NO_PAD
        .decode(encoded.as_bytes())
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
}
