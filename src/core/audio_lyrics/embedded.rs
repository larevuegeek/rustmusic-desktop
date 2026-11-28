//! Paroles intégrées aux tags du fichier : USLT des MP3, LYRICS et UNSYNCEDLYRICS des FLAC et
//! Ogg, ©lyr des M4A, lyrics des APE et MKV ; USLT de l'ID3 des DSF et DFF.

use std::fs::File;
use std::path::Path;

use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::{MetadataOptions, StandardTag};
use symphonia::default::get_probe;

/// Texte des paroles du fichier, `None` s'il n'en porte pas ou ne se lit pas.
pub fn read_embedded_lyrics(audio_path: &str) -> Option<String> {
    let path = Path::new(audio_path);
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    let text = if ext == "dsf" || ext == "dff" {
        let read = std::panic::catch_unwind(|| crate::core::audio_metadata::extractor::extractor::extract(path));
        read.ok()?.ok()?.tags.unsynchronised_lyrics
    } else {
        from_symphonia(path, &ext)
    };
    text.map(|t| t.trim().to_string()).filter(|t| !t.is_empty())
}

fn from_symphonia(path: &Path, ext: &str) -> Option<String> {
    let file = File::open(path).ok()?;
    let mut hint = Hint::new();
    hint.with_extension(ext);
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut format = get_probe().probe(&hint, mss, FormatOptions::default(), MetadataOptions::default()).ok()?;
    crate::core::audio_metadata::tag_format::revisions::skip_id3v1(format.as_mut());
    let metadata = format.metadata();
    let revision = metadata.current()?;
    revision
        .media
        .tags
        .iter()
        .chain(revision.per_track.iter().flat_map(|t| t.metadata.tags.iter()))
        .find_map(|tag| match &tag.std {
            Some(StandardTag::Lyrics(text)) if !text.trim().is_empty() => Some(text.to_string()),
            _ => None,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// MP3 minimal : ID3v2.4 avec une trame USLT (UTF-8, langue « fra »), puis dix trames muettes.
    fn mp3_with_lyrics(lyrics: &str) -> Vec<u8> {
        let syncsafe = |n: u32| [(n >> 21) as u8 & 0x7F, (n >> 14) as u8 & 0x7F, (n >> 7) as u8 & 0x7F, n as u8 & 0x7F];
        let mut body = vec![3u8];
        body.extend_from_slice(b"fra\0");
        body.extend_from_slice(lyrics.as_bytes());
        let mut tag = b"USLT".to_vec();
        tag.extend_from_slice(&syncsafe(body.len() as u32));
        tag.extend_from_slice(&[0, 0]);
        tag.extend_from_slice(&body);
        let mut file = b"ID3\x04\x00\x00".to_vec();
        file.extend_from_slice(&syncsafe(tag.len() as u32));
        file.extend_from_slice(&tag);
        for _ in 0..10 {
            let mut mpeg = vec![0u8; 417];
            mpeg[..4].copy_from_slice(&[0xFF, 0xFB, 0x90, 0x00]);
            file.extend_from_slice(&mpeg);
        }
        file
    }

    #[test]
    fn the_uslt_frame_of_an_mp3_is_read() {
        let dir = std::env::temp_dir().join(format!("rustmusic-uslt-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("paroles.mp3");
        std::fs::write(&path, mp3_with_lyrics("[00:01.00] Un\nOne\n[00:02.00] Deux")).unwrap();
        let path = path.to_string_lossy().to_string();
        assert_eq!(read_embedded_lyrics(&path).as_deref(), Some("[00:01.00] Un\nOne\n[00:02.00] Deux"));
        assert_eq!(read_embedded_lyrics(&dir.join("absent.mp3").to_string_lossy()), None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
