//! Image de profil : importée, recadrée en carré, réduite et gardée dans le dossier de l'appli.

use std::path::{Path, PathBuf};

use image::imageops::FilterType;

/// Côté de l'image gardée, en pixels.
const SIZE: u32 = 512;

pub fn avatars_dir() -> PathBuf {
    let mut dir = dirs::data_dir().unwrap_or_default();
    dir.push("com.larevuegeek.rustmusic");
    dir.push("avatars");
    dir
}

/// Importe `source` (JPEG, PNG, WebP…) dans `dir` : centre gardé en carré, 512 px, JPEG. Rend le chemin.
pub fn import_into(source: &Path, dir: &Path) -> Result<PathBuf, String> {
    let img = image::open(source).map_err(|e| format!("image illisible : {e}"))?;
    let square = img.resize_to_fill(SIZE, SIZE, FilterType::Lanczos3).to_rgb8();
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("{}.jpg", uuid::Uuid::new_v4()));
    let mut out = std::io::BufWriter::new(std::fs::File::create(&path).map_err(|e| e.to_string())?);
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 90)
        .encode_image(&square)
        .map_err(|e| e.to_string())?;
    Ok(path)
}

/// Efface une ancienne image, seulement si c'est une des nôtres.
pub fn remove_if_ours(path: Option<&str>) {
    if let Some(p) = path.map(Path::new).filter(|p| p.starts_with(avatars_dir())) {
        let _ = std::fs::remove_file(p);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imports_a_square_jpeg() {
        let dir = std::env::temp_dir().join(format!("rm-avatars-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let src = dir.join("photo.png");
        image::RgbImage::from_pixel(800, 300, image::Rgb([200, 30, 30])).save(&src).unwrap();
        let out = import_into(&src, &dir.join("out")).unwrap();
        let back = image::open(&out).unwrap();
        assert_eq!((back.width(), back.height()), (SIZE, SIZE));
        assert!(import_into(&dir.join("absent.png"), &dir).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
