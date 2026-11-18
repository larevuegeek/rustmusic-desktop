//! Le détail de la chaîne audio : ouverture au clic sur la qualité du lecteur, copie.

use crate::app::music_app::MusicApp;
use crate::models::app::Menu;
use crate::models::layout::ToastKind;
use karbon_ui::intl::t;

/// Identifiant du menu et de son ancre (le bloc qualité de la barre du lecteur).
pub(crate) const CHAIN_MENU: &str = "player.chain";

impl MusicApp {
    /// Ouvre le détail de la chaîne, ou le referme s'il est ouvert.
    pub(crate) fn chain_menu(&mut self) -> bool {
        self.menu = if self.popup(CHAIN_MENU) { None } else { Some(Menu::Popup(CHAIN_MENU)) };
        true
    }

    /// Copie `text` dans le presse-papiers, puis le confirme.
    pub(crate) fn copy_text(&mut self, text: String) -> bool {
        self.backend.call(
            async move {
                crate::app::platform::runtime::spawn_blocking(move || crate::core::system::clipboard::copy_text(text))
                    .await
                    .map_err(|e| e.to_string())
                    .and_then(|r| r)
            },
            |me, result| {
                match result {
                    Ok(()) => me.toast(ToastKind::Success, t("common.copied"), None),
                    Err(e) => log::error!("[presse-papiers] copie : {e}"),
                }
                true
            },
        );
        false
    }
}
