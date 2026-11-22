//! Les formats audio que le lecteur sait décoder : une seule liste pour l'ouverture d'un
//! fichier, l'import, le scan, l'explorateur de dossiers et le serveur DLNA.

use std::path::Path;

/// Extensions lues (ALAC est dans le .m4a ; WMA et APE ne se lisent pas).
pub const AUDIO_EXTENSIONS: [&str; 11] = ["mp3", "flac", "wav", "ogg", "opus", "m4a", "aac", "aiff", "aif", "dsf", "dff"];

/// L'extension (sans le point, casse indifférente) d'un format lu ? « .FLAC » ou « .Mp3 »
/// de vieux rips comptent aussi.
pub fn is_audio_extension(ext: &str) -> bool {
    AUDIO_EXTENSIONS.iter().any(|known| known.eq_ignore_ascii_case(ext))
}

/// Le fichier porte-t-il l'extension d'un format lu ?
pub fn is_audio_path(path: &Path) -> bool {
    path.extension().and_then(|ext| ext.to_str()).is_some_and(is_audio_extension)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_decoded_format_is_listed_whatever_its_case() {
        for ext in ["wav", "WAV", "Flac", "opus", "aiff", "aif", "aac", "dff"] {
            assert!(is_audio_extension(ext), "{ext}");
        }
        assert!(is_audio_path(Path::new("C:/Musique/Album/01 - Intro.WAV")));
        assert!(!is_audio_path(Path::new("C:/Musique/Album/Album.cue")));
        assert!(!is_audio_path(Path::new("C:/Musique/Album/sans_extension")));
        assert!(!is_audio_extension("wma"));
    }
}
