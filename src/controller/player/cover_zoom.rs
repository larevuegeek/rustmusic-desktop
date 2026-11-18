//! La pochette du lecteur agrandie : ouverture, menu du clic droit, copie de l'image.

use crate::app::music_app::MusicApp;
use crate::models::layout::ToastKind;
use crate::models::tracks::TrackMenuAt;
use karbon_ui::intl::t;

impl MusicApp {
    /// Ouvre ou referme la pochette agrandie (et son menu).
    pub(crate) fn cover_zoom(&mut self, open: bool) -> bool {
        self.views.cover_zoom = open;
        self.views.cover_menu = None;
        true
    }

    /// Menu du clic droit sur la pochette agrandie, ou fermé.
    pub(crate) fn cover_menu(&mut self, at: Option<TrackMenuAt>) -> bool {
        self.views.cover_menu = at;
        true
    }

    /// Copie la pochette affichée par le lecteur dans le presse-papiers.
    pub(crate) fn cover_copy(&mut self) -> bool {
        self.views.cover_menu = None;
        let Some(cover) = crate::mapper::home::home_resume::details(&self.data).and_then(|d| d.cover_path) else { return true };
        let (path, _) = crate::app::resources::assets::split_key(&cover);
        let path = std::path::PathBuf::from(path);
        self.backend.call(
            async move {
                crate::app::platform::runtime::spawn_blocking(move || crate::core::system::clipboard::copy_image(&path))
                    .await
                    .map_err(|e| e.to_string())
                    .and_then(|r| r)
            },
            |me, result| {
                match result {
                    Ok(()) => me.toast(ToastKind::Success, t("notify.cover_copied"), None),
                    Err(e) => {
                        log::error!("[pochette] copie : {e}");
                        me.toast(ToastKind::Error, t("notify.cover_copy_failed"), None);
                    }
                }
                true
            },
        );
        true
    }
}
