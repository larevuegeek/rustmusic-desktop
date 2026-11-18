use crate::models::tags::*;
use karbon_ui::intl::t;
use crate::models::layout::Popin;
use crate::models::layout::PopinBody;

/// `PRESETS` : (clé du nom, motif).
pub const PRESETS: [(&str, &str); 4] = [
    ("track_title", "{track:02} - {title}.{ext}"),
    ("track_artist_title", "{track:02}. {artist} - {title}.{ext}"),
    ("artist_title", "{artist} - {title}.{ext}"),
    ("disc_track_title", "[{disc}-]{track:02} - {title}.{ext}"),
];

/// `TREE_PRESETS`.
pub const TREE_PRESETS: [(&str, &str); 4] = [
    ("tree_artist_album", "{albumartist|artist}/{album}/{track:02} - {title}.{ext}"),
    ("tree_artist_album_year", "{albumartist|artist}/{album} ({year})/{track:02} - {title}.{ext}"),
    ("tree_genre_artist_album", "{genre}/{albumartist|artist}/{album}/{track:02} - {title}.{ext}"),
    ("tree_artist_album_disc", "{albumartist|artist}/{album}/[{disc}-]{track:02} - {title}.{ext}"),
];

/// `PATTERN_FIELDS`.
pub const PATTERN_FIELDS: [&str; 10] = ["artist", "albumartist", "album", "title", "year", "genre", "track", "disc", "composer", "ext"];

/// Pour le gabarit.
pub fn pattern_fields() -> [&'static str; 10] {
    PATTERN_FIELDS
}

/// L'exemple du champ vide : le premier préréglage.
pub fn first_preset() -> &'static str {
    PRESETS[0].1
}

impl RenameState {
    pub fn pattern(&self) -> &str {
        if self.restructure {
            &self.tree_pattern
        } else {
            &self.name_pattern
        }
    }

    /// Préréglages du mode : (nom traduit, motif).
    pub fn presets(&self) -> Vec<(String, &'static str)> {
        let list = if self.restructure { &TREE_PRESETS } else { &PRESETS };
        list.iter().map(|(k, p)| (t(&format!("rename.preset_names.{k}")), *p)).collect()
    }

    pub fn changeable(&self) -> Vec<&Move> {
        self.preview.as_ref().map(|p| p.0.iter().filter(|m| !m.unchanged && !m.blocked).collect()).unwrap_or_default()
    }

    pub fn selected(&self) -> usize {
        self.changeable().iter().filter(|m| !self.excluded.contains(&m.from)).count()
    }

    pub fn all_selected(&self) -> bool {
        let c = self.changeable().len();
        c > 0 && self.selected() == c
    }

    pub fn is_on(&self, m: &Move) -> bool {
        !m.blocked && !m.unchanged && !self.excluded.contains(&m.from)
    }

    /// Le compte du pied : « 3 / 5 seront renommés ».
    pub fn will_change(&self) -> String {
        let n = if self.all_selected() { self.selected().to_string() } else { format!("{} / {}", self.selected(), self.changeable().len()) };
        format!("{n} {}", t("rename.will_change"))
    }
}

impl RenameState {
    /// Rang du dossier de destination dans la liste.
    pub fn root_index(&self) -> u64 {
        self.dirs.iter().position(|d| *d == self.root).unwrap_or(0) as u64
    }

    /// « 12 fichiers vont être renommés. »
    pub fn confirm_label(&self) -> String {
        format!("{} {}", self.selected(), t("rename.confirm"))
    }

    /// « 2 bloqués », quand l'aperçu en compte.
    pub fn blocked_label(&self) -> Option<String> {
        self.preview.as_ref().filter(|p| p.1 > 0).map(|p| format!("{} {}", p.1, t("rename.blocked")))
    }

    /// « 5 déjà bien nommés », quand l'aperçu en compte.
    pub fn already_ok_label(&self) -> Option<String> {
        self.preview.as_ref().filter(|p| p.2 > 0).map(|p| format!("{} {}", p.2, t("rename.already_ok")))
    }
}

impl Outcome {
    /// « 12 renommés ».
    pub fn renamed_label(&self) -> String {
        format!("{} {}", self.succeeded, t("rename.renamed"))
    }

    /// « 2 échecs ».
    pub fn failed_label(&self) -> String {
        format!("{} {}", self.failed.len(), t("rename.failed"))
    }

    /// Les échecs : (nom du fichier, message).
    pub fn failures(&self) -> Vec<(String, String)> {
        self.failed
            .iter()
            .map(|(path, message)| (path.rsplit(['/', '\\']).next().unwrap_or_default().to_string(), message.clone()))
            .collect()
    }
}

impl Popin {
    pub fn rename(&self) -> Option<&RenameState> {
        match &self.body {
            PopinBody::Rename(r) => Some(r),
            _ => None,
        }
    }
}
