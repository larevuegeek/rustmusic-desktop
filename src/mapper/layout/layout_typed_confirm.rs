use crate::models::layout::*;
use karbon_ui::intl::t;

/// `settings.reset_confirm_word`.
pub fn word() -> String {
    t("settings.reset_confirm_word")
}

/// La phrase coupée autour de `{b}`.
pub fn bold_parts(key: &str) -> (String, String) {
    let s = t(key);
    let mut it = s.splitn(2, "{b}");
    (it.next().unwrap_or_default().to_string(), it.next().unwrap_or_default().to_string())
}

/// Ce que la réinitialisation efface.
pub const RESET_ITEMS: [&str; 5] = ["libraries", "playlists", "liked", "settings", "thumbnails"];

/// Ce que le retrait d'un serveur emporte.
pub const SERVER_ITEMS: [&str; 4] = ["catalog", "traces", "playing", "cache"];

/// La phrase `key` coupée autour de `{b}`, avec `bold` au milieu.
fn bold_line(key: &str, bold: String) -> BoldLine {
    let (before, after) = bold_parts(key);
    BoldLine { before, bold, after }
}

impl TypedConfirm {
    pub fn locked(&self) -> bool {
        self.busy || self.typed != word()
    }

    pub fn title(&self) -> String {
        match self.kind {
            TypedKind::Reset => t("settings.reset"),
            TypedKind::RemoveServer { .. } => t("servers.remove_title"),
        }
    }

    /// Tout réinitialiser : l'avertissement et ce qui sera effacé.
    pub fn reset_view(&self) -> Option<ResetView> {
        (self.kind == TypedKind::Reset).then(|| ResetView {
            intro: bold_line("settings.reset_intro", t("settings.reset_irreversible")),
            items: RESET_ITEMS
                .iter()
                .map(|el| bold_line(&format!("settings.reset_item_{el}"), t(&format!("settings.reset_item_{el}_b"))))
                .collect(),
        })
    }

    /// Retirer un serveur : son nom, ses bibliothèques et ce que le retrait emporte.
    pub fn remove_server_view(&self) -> Option<RemoveServerView> {
        let TypedKind::RemoveServer { name, libraries, tracks, .. } = &self.kind else {
            return None;
        };
        let tracks = karbon_ui::intl::number(*tracks);
        Some(RemoveServerView {
            intro: bold_line("servers.remove_intro", name.clone()),
            libraries: libraries
                .iter()
                .map(|(library, count)| (library.clone(), format!("· {}", karbon_ui::intl::number(*count))))
                .collect(),
            items: SERVER_ITEMS
                .iter()
                .map(|item| karbon_ui::intl::t_with(&format!("servers.remove_item_{item}"), &[("n", &tracks)]))
                .collect(),
        })
    }

    pub fn confirm_label(&self) -> String {
        match self.kind {
            TypedKind::Reset => t("settings.reset_confirm_all"),
            TypedKind::RemoveServer { .. } => t("servers.remove_title"),
        }
    }
}
