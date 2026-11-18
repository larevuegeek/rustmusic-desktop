//! Error type returned by the audio_metadata extractor.
//! Hand-written `Display` to avoid a `thiserror` dependency.

use std::fmt;
use std::io;

#[derive(Debug)]
pub enum ExtractError {
    /// Underlying I/O error (file open, read, seek).
    Io(io::Error),
    /// File extension or magic bytes don't match anything we handle.
    UnsupportedFormat(String),
    /// Header bytes are present but malformed.
    InvalidHeader(String),
    /// Tag block is present but malformed.
    InvalidTags(String),
    /// File ended before we expected.
    Truncated,
}

impl fmt::Display for ExtractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExtractError::Io(e) => write!(f, "I/O error: {e}"),
            ExtractError::UnsupportedFormat(name) => write!(f, "Unsupported format: {name}"),
            ExtractError::InvalidHeader(msg) => write!(f, "Invalid header: {msg}"),
            ExtractError::InvalidTags(msg) => write!(f, "Invalid tags: {msg}"),
            ExtractError::Truncated => write!(f, "File truncated unexpectedly"),
        }
    }
}

impl std::error::Error for ExtractError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ExtractError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for ExtractError {
    fn from(e: io::Error) -> Self {
        ExtractError::Io(e)
    }
}
