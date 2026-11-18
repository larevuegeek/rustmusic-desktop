//! Detect an audio file's format.
//!
//! Strategy: look at the file extension first (cheap), then read the
//! first magic bytes to confirm.
//!
//! Phase 1: DSF only. Anything else returns `AudioFormat::Unknown` so
//! that the caller can fall back to its existing pipeline.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use crate::core::audio_metadata::extractor::error::ExtractError;
use crate::entity::audio::audio_file::AudioFormat;

/// Try to identify the format of `path`.
pub fn detect(path: &Path) -> Result<AudioFormat, ExtractError> {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase());

    match ext.as_deref() {
        Some("dsf") => confirm_magic(path, b"DSD ", AudioFormat::DSF, "DSF"),
        Some("dff") => confirm_magic(path, b"FRM8", AudioFormat::DFF, "DFF"),
        _ => Ok(AudioFormat::Unknown),
    }
}

/// Verify that `path` actually starts with the expected magic bytes.
fn confirm_magic(
    path: &Path,
    expected: &[u8; 4],
    fmt: AudioFormat,
    label: &str,
) -> Result<AudioFormat, ExtractError> {
    let mut file = File::open(path)?;
    let mut magic = [0u8; 4];
    file.read_exact(&mut magic)?;
    if &magic == expected {
        Ok(fmt)
    } else {
        Err(ExtractError::InvalidHeader(format!(
            "Expected {} magic {:?}, got {:?}",
            label, expected, magic
        )))
    }
}
