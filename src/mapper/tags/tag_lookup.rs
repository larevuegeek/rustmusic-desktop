use crate::models::tags::*;
use karbon_ui::intl::t;
use karbon_ui::storage;
use crate::core::metadata_source::model::Source;

pub(crate) const SOURCE_KEY: &str = "tags:source";

/// Libellé « 3 modifications retenues ».
pub fn changes_label(n: usize) -> String {
    if n > 0 {
        format!("{n} {}", t("source.selected_changes"))
    } else {
        t("source.nothing_selected")
    }
}

impl Lookup {
    pub fn new(query: String) -> Self {
        Lookup {
            query,
            musicbrainz: storage::local().get(SOURCE_KEY).as_deref() == Some("musicbrainz"),
            hits: Vec::new(),
            searching: false,
            searched: false,
            loading: None,
            error: None,
            seq: 0,
            picked: None,
            cover: None,
            cover_loading: false,
            cover_error: None,
            take_cover: false,
        }
    }

    pub(crate) fn source(&self) -> Source {
        if self.musicbrainz {
            Source::Musicbrainz
        } else {
            Source::Deezer
        }
    }

    pub fn source_name(&self) -> &'static str {
        if self.musicbrainz {
            "MusicBrainz"
        } else {
            "Deezer"
        }
    }

    /// Lignes que l'on peut cocher.
    pub fn selectable(&self) -> Vec<&'static str> {
        self.picked.as_ref().map(|p| p.1.iter().filter(|r| r.state == 0).map(|r| r.key).collect()).unwrap_or_default()
    }

    pub fn all_checked(&self) -> bool {
        let s = self.selectable();
        !s.is_empty() && self.picked.as_ref().is_some_and(|p| s.iter().all(|k| p.2.contains(k)))
    }

    pub fn change_count(&self) -> usize {
        let s = self.selectable();
        self.picked.as_ref().map_or(0, |p| p.2.iter().filter(|k| s.contains(k)).count()) + usize::from(self.take_cover)
    }

    pub fn is_checked(&self, key: &str) -> bool {
        self.picked.as_ref().is_some_and(|p| p.2.contains(&key))
    }
}
