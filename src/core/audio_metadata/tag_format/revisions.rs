//! Choix des tags lus par symphonia quand un fichier en porte plusieurs blocs.
//!
//! symphonia cherche les tags de fin de fichier (ID3v1, APE) avant ceux du début (ID3v2) :
//! un MP3 qui porte un ID3v1 en plus de son ID3v2 présente donc d'abord l'ID3v1. Ce vieux
//! bloc ne connaît que l'ISO-8859-1 et 30 caractères par champ (« Bonjour Vi?t Nam »), ni
//! pochette ni Replay Gain : il ne sert que s'il est seul.

use symphonia::core::formats::FormatReader;
use symphonia::core::meta::well_known::METADATA_ID_ID3V1;
use symphonia::core::meta::MetadataRevision;

fn is_id3v1(revision: &MetadataRevision) -> bool {
    revision.info.metadata == METADATA_ID_ID3V1
}

/// Écarte les révisions ID3v1 tant qu'une autre suit : `metadata().current()` rend
/// ensuite les tags complets du fichier.
pub fn skip_id3v1(format: &mut dyn FormatReader) {
    let mut metadata = format.metadata();
    while metadata.current().is_some_and(is_id3v1) && !metadata.is_latest() {
        metadata.pop();
    }
}

#[cfg(test)]
mod tests {
    use crate::core::audio_analyser::audio_analyser::AudioAnalyser;

    /// MP3 minimal : ID3v2.4 (titre UTF-8), dix trames muettes, puis un ID3v1 dont le titre
    /// a perdu ses lettres hors ISO-8859-1.
    fn mp3_with_both_tags(title: &str, latin1_title: &str) -> Vec<u8> {
        let mut frame = vec![3u8];
        frame.extend_from_slice(title.as_bytes());
        let mut tag = b"TIT2".to_vec();
        let size = frame.len() as u32;
        tag.extend_from_slice(&[(size >> 21) as u8 & 0x7F, (size >> 14) as u8 & 0x7F, (size >> 7) as u8 & 0x7F, size as u8 & 0x7F, 0, 0]);
        tag.extend_from_slice(&frame);
        let mut file = b"ID3\x04\x00\x00".to_vec();
        let size = tag.len() as u32;
        file.extend_from_slice(&[(size >> 21) as u8 & 0x7F, (size >> 14) as u8 & 0x7F, (size >> 7) as u8 & 0x7F, size as u8 & 0x7F]);
        file.extend_from_slice(&tag);
        for _ in 0..10 {
            let mut mpeg = vec![0u8; 417];
            mpeg[..4].copy_from_slice(&[0xFF, 0xFB, 0x90, 0x00]);
            file.extend_from_slice(&mpeg);
        }
        let mut v1 = [0u8; 128];
        v1[..3].copy_from_slice(b"TAG");
        v1[3..3 + latin1_title.len()].copy_from_slice(latin1_title.as_bytes());
        v1[127] = 255;
        file.extend_from_slice(&v1);
        file
    }

    #[test]
    fn id3v2_wins_over_a_trailing_id3v1() {
        let dir = std::env::temp_dir().join(format!("rustmusic-id3v1-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("viet.mp3");
        std::fs::write(&path, mp3_with_both_tags("Bonjour Việt Nam", "Bonjour Vi?t Nam")).unwrap();
        let file = AudioAnalyser::analyse_audio_file(&path).unwrap();
        assert_eq!(file.tags.title.as_deref(), Some("Bonjour Việt Nam"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
