//! L'éditeur de tags : ce qu'affichent ses gabarits, bâti depuis l'état de la page.

use crate::models::tags::*;
use karbon_ui::intl::t;
use crate::models::layout::Popin;
use crate::models::layout::PopinBody;

/// Les champs, dans l'ordre du formulaire d'édition des tags.
pub const EDITOR_FIELDS: [&str; 12] =
    ["title", "artist", "album", "album_artist", "year", "genre", "composer", "comment", "track_number", "total_tracks", "disc_number", "total_discs"];

pub const COVER: u8 = 3;

/// Libellé d'un type d'image.
pub fn type_label(kind: u8) -> String {
    let key = match kind {
        0 => "tags.pic.other",
        3 => "tags.pic.cover_front",
        4 => "tags.pic.cover_back",
        5 => "tags.pic.leaflet",
        6 => "tags.pic.media_label",
        7 => "tags.pic.lead_artist",
        8 => "tags.pic.artist",
        11 => "tags.pic.composer",
        18 => "tags.pic.illustration",
        _ => return format!("{} ({kind})", t("tags.pic.other")),
    };
    t(key)
}

/// Pour les gabarits : la clé d'un champ.
pub fn key(i: usize) -> &'static str {
    EDITOR_FIELDS[i]
}

/// « 3 champs modifiés · images modifiées ».
pub fn summary(e: &TagEditor) -> String {
    let n = e.changed();
    let mut parts = Vec::new();
    if n > 0 {
        parts.push(format!("{n} {}", t(if n > 1 { "tags.changed_other" } else { "tags.changed_one" })));
    }
    if e.media_dirty() {
        parts.push(t("tags.media_changed"));
    }
    parts.join(" · ")
}

/// Les cinq actions d'une image : icône, libellé, message, désactivée, rouge.
pub fn actions(i: usize, len: usize, cover: bool) -> Vec<(&'static str, String, crate::models::app::Msg, bool, bool)> {
    use crate::models::app::Msg;
    let m = |x: TagEditorMsg| Msg::TagEditor(x);
    vec![
        ("lucide:star", t("tags.set_cover"), m(TagEditorMsg::SetCover(i)), cover, false),
        ("lucide:arrow-up", t("tags.move_up"), m(TagEditorMsg::Move(i, false)), i == 0, false),
        ("lucide:arrow-down", t("tags.move_down"), m(TagEditorMsg::Move(i, true)), i + 1 == len, false),
        ("lucide:refresh-cw", t("tags.replace_image"), Msg::Noop, false, false),
        ("lucide:trash-2", t("tags.remove_image"), m(TagEditorMsg::Remove(i)), false, true),
    ]
}

/// Le libellé d'un champ de tag (« Titre », « Artiste de l'album »…) par sa clé.
pub fn tag_label(key: &str) -> String {
    t(&format!("tags.{key}"))
}

/// Le libellé du champ de rang `i` du formulaire.
pub fn field_label(i: usize) -> String {
    tag_label(key(i))
}

impl Slot {
    /// « JPEG ».
    pub fn format_label(&self) -> String {
        self.mime.replace("image/", "").to_uppercase()
    }

    /// « 245 Ko ».
    pub fn size_label(&self) -> String {
        karbon_ui::intl::bytes(self.bytes as u64)
    }

    /// « JPEG · 245 Ko ».
    pub fn summary(&self) -> String {
        format!("{} · {}", self.format_label(), self.size_label())
    }

    /// « image/jpeg · 245 Ko » (image agrandie).
    pub fn zoom_summary(&self) -> String {
        format!("{} · {}", self.mime, self.size_label())
    }
}

impl Remote {
    /// « 245 Ko », ou « 245 Ko · réduite » après recompression.
    pub fn size_line(&self) -> String {
        let size = karbon_ui::intl::bytes(self.bytes as u64);
        if self.recompressed { format!("{size} · {}", t("tags.reduced")) } else { size }
    }
}

impl crate::core::metadata_source::model::TrackHit {
    /// « Artiste · Album · 3:42 ».
    pub fn summary(&self) -> String {
        format!("{} · {} · {}", self.artist, self.album, super::duration(self.duration))
    }
}

impl TagEditor {
    /// Nombre d'images du fichier.
    pub fn slot_count(&self) -> usize {
        self.slots.len()
    }

    /// Poids des images : « 1,2 Mo ».
    pub fn media_size(&self) -> String {
        karbon_ui::intl::bytes(self.media_bytes() as u64)
    }

    /// Infobulle du poids des images : « Images · 1,2 Mo ».
    pub fn media_title(&self) -> String {
        format!("{} · {}", t("tags.media"), self.media_size())
    }

    pub fn changed(&self) -> usize {
        self.form.iter().zip(&self.original).filter(|(a, b)| a != b).count()
    }

    pub fn is_dirty(&self, i: usize) -> bool {
        self.form.get(i) != self.original.get(i)
    }

    pub fn media_dirty(&self) -> bool {
        let sig = |s: &[Slot]| s.iter().map(|x| format!("{}:{}", x.id.clone().or(x.path.clone()).unwrap_or_default(), x.picture_type)).collect::<Vec<_>>().join("|");
        sig(&self.slots) != sig(&self.original_slots)
    }

    pub fn dirty(&self) -> bool {
        self.changed() > 0 || self.media_dirty()
    }

    /// La pochette typée, sinon la première image.
    pub fn cover(&self) -> Option<&Slot> {
        self.slots.iter().find(|s| s.picture_type == COVER).or(self.slots.first())
    }

    pub fn filename(&self) -> String {
        self.path.rsplit(['/', '\\']).next().unwrap_or(&self.path).to_string()
    }

    pub fn extension(&self) -> Option<String> {
        let f = self.filename();
        f.rsplit_once('.').map(|(_, e)| e.to_uppercase()).filter(|e| e.len() <= 5)
    }

    pub fn media_bytes(&self) -> usize {
        self.slots.iter().map(|s| s.bytes).sum()
    }

    pub fn value(&self, key: &str) -> String {
        EDITOR_FIELDS.iter().position(|f| *f == key).and_then(|i| self.form.get(i).cloned()).unwrap_or_default()
    }

    /// Infobulle du bouton « rétablir » : l'ancienne valeur, sauf pour le commentaire.
    pub fn revert_label(&self, i: usize) -> String {
        if EDITOR_FIELDS[i] == "comment" {
            return t("tags.revert");
        }
        let v = self.original.get(i).filter(|v| !v.is_empty()).cloned().unwrap_or_else(|| "—".into());
        format!("{} : {v}", t("tags.revert"))
    }

    /// Le résumé des champs repris de Deezer.
    pub fn filled_label(&self) -> String {
        self.filled.iter().map(|k| t(&format!("tags.{k}"))).collect::<Vec<_>>().join(", ")
    }
}

impl Popin {
    pub fn tag_edit(&self) -> Option<&TagEditor> {
        match &self.body {
            PopinBody::TagEdit(e) => Some(e),
            _ => None,
        }
    }
}
