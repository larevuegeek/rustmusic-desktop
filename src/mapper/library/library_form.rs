use crate::models::library::*;
use karbon_ui::intl::t;
use crate::models::layout::Popin;
use crate::models::layout::PopinBody;

impl LibraryForm {
    pub fn preview_name(&self) -> String {
        if !self.name.trim().is_empty() {
            return self.name.trim().to_string();
        }
        self.edit.as_ref().map(|e| e.1.clone()).unwrap_or_else(|| t("library.create_library"))
    }

    pub fn preview_detail(&self) -> String {
        format!("{} · {}", t("home.tracks_n").replace("{n}", &self.edit.as_ref().map_or(0, |e| e.2).to_string()), t("nav.library"))
    }

    /// Pied : libellé et icône du bouton principal.
    pub fn submit(&self) -> (String, &'static str) {
        if self.edit.is_some() {
            (t("selector.rename"), "material-symbols:check-rounded")
        } else {
            (t("library.create_library"), "material-symbols:add-rounded")
        }
    }
}

impl Popin {
    pub fn library_form(&self) -> Option<&LibraryForm> {
        match &self.body {
            PopinBody::LibraryForm(f) => Some(f),
            _ => None,
        }
    }
}
