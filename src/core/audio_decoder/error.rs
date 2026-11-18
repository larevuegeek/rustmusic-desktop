//! Error type shared by all audio_decoder submodules.
//! Hand-written `Display` to keep the project free of `thiserror`.

use std::fmt;
use std::io;

use crate::core::audio_metadata::extractor::error::ExtractError;

#[derive(Debug)]
pub enum DecodeError {
    /// File I/O failure (open, read, seek).
    Io(io::Error),
    /// Header is present but malformed.
    InvalidFormat(String),
    /// Format / codec not handled by this decoder.
    UnsupportedFormat(String),
    /// File ended earlier than expected.
    Truncated,
    /// DSP-level configuration error (e.g. unsupported rate ratio).
    InvalidConfig(String),
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DecodeError::Io(e) => write!(f, "I/O error: {e}"),
            DecodeError::InvalidFormat(msg) => write!(f, "Invalid format: {msg}"),
            DecodeError::UnsupportedFormat(name) => write!(f, "Unsupported format: {name}"),
            DecodeError::Truncated => write!(f, "File truncated unexpectedly"),
            DecodeError::InvalidConfig(msg) => write!(f, "Invalid decoder config: {msg}"),
        }
    }
}

impl std::error::Error for DecodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DecodeError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for DecodeError {
    fn from(e: io::Error) -> Self {
        DecodeError::Io(e)
    }
}

/// Bridge from the metadata extractor's error so that decoders can
/// freely reuse `audio_metadata::file_format::*::parse_*` helpers with `?`.
impl From<ExtractError> for DecodeError {
    fn from(e: ExtractError) -> Self {
        match e {
            ExtractError::Io(io) => DecodeError::Io(io),
            ExtractError::InvalidHeader(msg) => DecodeError::InvalidFormat(msg),
            ExtractError::InvalidTags(msg) => DecodeError::InvalidFormat(msg),
            ExtractError::Truncated => DecodeError::Truncated,
            ExtractError::UnsupportedFormat(name) => DecodeError::UnsupportedFormat(name),
        }
    }
}
