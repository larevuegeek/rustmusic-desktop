use crate::models::tags::*;
use crate::app::music_app::MusicApp;
use crate::models::layout::Popin;
use crate::models::layout::PopinBody;
use crate::mapper::tags::FIELDS;

/// `SOURCE_FIELDS` (noms de champ, communs à l'atelier).
pub const SOURCE_FIELDS: [&str; 10] = ["title", "artist", "album", "album_artist", "year", "genre", "track_number", "total_tracks", "disc_number", "composer"];

pub fn field_index(f: &str) -> usize {
    FIELDS.iter().position(|x| *x == f).unwrap_or(0)
}

impl MusicApp {
    /// Valeur d'un champ d'un fichier de l'atelier.
    pub(crate) fn workshop_value(&self, path: &str, field: &str) -> String {
        self.views.tags.as_ref().and_then(|p| p.files.iter().find(|f| f.path == path)).map(|f| f.value(field_index(field)).trim().to_string()).unwrap_or_default()
    }
}

impl Popin {
    pub fn source(&self) -> Option<&SourceState> {
        match &self.body {
            PopinBody::Source(s) => Some(s),
            _ => None,
        }
    }
}
