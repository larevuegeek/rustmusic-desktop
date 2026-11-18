//! Error type shared by the dlna_server modules.

use std::fmt;
use std::io;

#[derive(Debug)]
pub enum DlnaError {
    Io(io::Error),
    InvalidConfig(String),
    NotRunning,
    AlreadyRunning,
    Library(String),
    Http(String),
    Internal(String),
}

impl fmt::Display for DlnaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DlnaError::Io(e) => write!(f, "I/O error: {e}"),
            DlnaError::InvalidConfig(msg) => write!(f, "Invalid config: {msg}"),
            DlnaError::NotRunning => write!(f, "DLNA server is not running"),
            DlnaError::AlreadyRunning => write!(f, "DLNA server is already running"),
            DlnaError::Library(msg) => write!(f, "Library access error: {msg}"),
            DlnaError::Http(msg) => write!(f, "HTTP error: {msg}"),
            DlnaError::Internal(msg) => write!(f, "Internal error: {msg}"),
        }
    }
}

impl std::error::Error for DlnaError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DlnaError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for DlnaError {
    fn from(e: io::Error) -> Self {
        DlnaError::Io(e)
    }
}
