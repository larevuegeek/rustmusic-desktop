//! Public API of the audio_metadata module.
//!
//! Detects the file format then delegates to the appropriate
//! `file_format::*` module to extract technical info + tags.
//! After format-specific extraction, applies a generic folder cover
//! fallback (cover.jpg / folder.jpg / front.jpg in the same directory).

use std::path::Path;

use base64::Engine;

use crate::core::audio_metadata::extractor::error::ExtractError;
use crate::core::audio_metadata::extractor::format_sniffer;
use crate::core::audio_metadata::file_format;
use crate::entity::audio::audio_file::{AudioFile, AudioFormat};
use crate::entity::audio::audio_tags::{AttachedImage, ImageType};

/// Extract a complete `AudioFile` (technical info + tags + cover art) from a
/// file on disk. Phase 1 supports DSF only.
pub fn extract(path: &Path) -> Result<AudioFile, ExtractError> {
    let format = format_sniffer::detect(path)?;
    let mut audio_file = match format {
        AudioFormat::DSF => file_format::dsf::extract(path)?,
        AudioFormat::DFF => file_format::dff::extract(path)?,
        other => return Err(ExtractError::UnsupportedFormat(format!("{:?}", other))),
    };

    // If the file has no embedded cover, look for cover.jpg / folder.jpg / front.* in the parent directory.
    if audio_file.tags.attached_images.is_empty() {
        if let Some(image) = try_folder_cover(path) {
            audio_file.tags.attached_images.push(image);
        }
    }

    // Title fallback : DSF/DFF files often lack a DITI chunk and an ID3
    // chunk altogether (typical of CD-to-DSD direct rips). Use the filename
    // (without extension) so the UI never shows "Titre inconnu" by default.
    if audio_file.tags.title.as_deref().map(str::trim).unwrap_or("").is_empty() {
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            audio_file.tags.title = Some(stem.to_string());
        }
    }

    Ok(audio_file)
}

// ─── Folder cover fallback ───────────────────────────────────────────

const COVER_NAMES: [&str; 5] = ["cover", "folder", "Cover", "Folder", "front"];
const COVER_EXTS: [&str; 5] = ["jpg", "jpeg", "png", "webp", "avif"];

/// Look for a cover image in the same directory as `audio_path` and return
/// a populated `AttachedImage` if one is found.
fn try_folder_cover(audio_path: &Path) -> Option<AttachedImage> {
    let folder = audio_path.parent()?;

    let cover_path = COVER_NAMES.iter().flat_map(|name| {
        COVER_EXTS
            .iter()
            .map(move |ext| folder.join(format!("{name}.{ext}")))
    }).find(|p| p.exists())?;

    let data = std::fs::read(&cover_path).ok()?;
    let mime_type = mime_for_extension(&cover_path).to_string();
    let image_src = format!(
        "data:{mime_type};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(&data)
    );

    Some(AttachedImage {
        image_type: Some(ImageType::CoverFront),
        mime_type,
        description: None,
        image_data: data,
        image_src,
    })
}

fn mime_for_extension(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase) {
        Some(ref e) if e == "jpg" || e == "jpeg" => "image/jpeg",
        Some(ref e) if e == "png" => "image/png",
        Some(ref e) if e == "webp" => "image/webp",
        Some(ref e) if e == "avif" => "image/avif",
        _ => "application/octet-stream",
    }
}
