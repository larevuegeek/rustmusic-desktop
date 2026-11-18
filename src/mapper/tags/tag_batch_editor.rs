use crate::models::tags::*;
use serde_json::{json, Map, Value};
use karbon_ui::intl::t;
use crate::models::layout::Popin;
use crate::models::layout::PopinBody;
use crate::mapper::tags::EDITOR_FIELDS;

impl BatchEditor {
    /// Nombre de fichiers modifiés ensemble.
    pub fn file_count(&self) -> usize {
        self.paths.len()
    }

    /// « 2 champs seront effacés · 12 fichiers ».
    pub fn erase_warning(&self) -> String {
        let n = self.erasing();
        let key = if n > 1 { "tags.fields_will_be_erased" } else { "tags.field_will_be_erased" };
        format!("{n} {} · {} {}", t(key), self.paths.len(), t("tags.files"))
    }

    /// « 3 champs modifiés », dès qu'un champ change.
    pub fn changed_label(&self) -> Option<String> {
        let n = self.changed();
        (n > 0).then(|| format!("{n} {}", t(if n > 1 { "tags.changed_other" } else { "tags.changed_one" })))
    }

    /// « Appliquer à 12 ».
    pub fn apply_label(&self) -> String {
        format!("{} {}", t("tags.apply_to"), self.paths.len())
    }

    pub fn mixed(&self, i: usize) -> bool {
        self.common.get(i).is_some_and(Option::is_none)
    }

    pub fn is_dirty(&self, i: usize) -> bool {
        if self.mixed(i) {
            self.touched.contains(&i)
        } else {
            self.common.get(i).and_then(|c| c.as_deref()) != Some(self.form[i].as_str())
        }
    }

    pub fn will_erase(&self, i: usize) -> bool {
        self.mixed(i) && self.touched.contains(&i) && self.form[i].trim().is_empty()
    }

    pub fn erasing(&self) -> usize {
        (0..EDITOR_FIELDS.len()).filter(|i| self.will_erase(*i)).count()
    }

    pub fn changed(&self) -> usize {
        (0..EDITOR_FIELDS.len()).filter(|i| self.is_dirty(*i)).count()
    }

    pub fn dirty(&self) -> bool {
        self.changed() > 0 || self.numbering
    }

    pub fn placeholder(&self, i: usize) -> String {
        if self.mixed(i) {
            t("tags.multiple_values")
        } else {
            "—".into()
        }
    }

    pub fn file_name(path: &str) -> String {
        path.rsplit(['/', '\\']).next().unwrap_or(path).to_string()
    }

    pub(crate) fn payload(&self) -> Map<String, Value> {
        let mut edit = Map::new();
        for (i, k) in EDITOR_FIELDS.iter().enumerate() {
            if self.is_dirty(i) {
                edit.insert((*k).into(), json!(self.form[i]));
            }
        }
        edit
    }
}

impl Popin {
    pub fn batch_edit(&self) -> Option<&BatchEditor> {
        match &self.body {
            PopinBody::BatchEdit(b) => Some(b),
            _ => None,
        }
    }
}
