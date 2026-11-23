//! Paroles trouvées avec le fichier, avant toute recherche en ligne.

use super::{embedded, sidecar};
use crate::entity::audio_lyrics::lyrics::LyricsSource;

/// Paroles locales : leur source, puis le texte brut ou le LRC synchronisé.
#[derive(Debug, PartialEq)]
pub struct LocalLyrics {
    pub source: LyricsSource,
    pub plain: Option<String>,
    pub synced: Option<String>,
}

/// Tags du fichier puis `.lrc` posé à côté ; une version synchronisée passe avant une
/// version en texte brut, d'où qu'elle vienne.
pub fn read_local_lyrics(audio_path: &str) -> Option<LocalLyrics> {
    choose(embedded::read_embedded_lyrics(audio_path), sidecar::read_sidecar_lrc(audio_path))
}

fn choose(embedded: Option<String>, sidecar: Option<String>) -> Option<LocalLyrics> {
    let found = [(LyricsSource::Embedded, embedded), (LyricsSource::Sidecar, sidecar)];
    let found: Vec<_> = found.into_iter().filter_map(|(s, t)| t.map(|t| (s, t))).collect();
    let synced = found.iter().position(|(_, t)| sidecar::has_synced_timestamps(t));
    let (source, text) = found.into_iter().nth(synced.unwrap_or(0))?;
    Some(match synced {
        Some(_) => LocalLyrics { source, plain: None, synced: Some(text) },
        None => LocalLyrics { source, plain: Some(text), synced: None },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const LRC: &str = "[00:01.00] Un\n[00:02.00] Deux";

    #[test]
    fn the_tags_come_before_the_lrc_file() {
        let got = choose(Some(LRC.into()), Some("[00:05.00] Autre".into())).unwrap();
        assert_eq!(got, LocalLyrics { source: LyricsSource::Embedded, plain: None, synced: Some(LRC.into()) });
    }

    #[test]
    fn a_synced_lrc_file_beats_plain_text_in_the_tags() {
        let got = choose(Some("Un\nDeux".into()), Some(LRC.into())).unwrap();
        assert_eq!(got, LocalLyrics { source: LyricsSource::Sidecar, plain: None, synced: Some(LRC.into()) });
    }

    #[test]
    fn plain_text_is_kept_when_nothing_is_synced() {
        let got = choose(Some("Un\nDeux".into()), None).unwrap();
        assert_eq!(got, LocalLyrics { source: LyricsSource::Embedded, plain: Some("Un\nDeux".into()), synced: None });
        assert_eq!(choose(None, None), None);
    }
}
