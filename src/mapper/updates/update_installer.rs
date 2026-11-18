use crate::models::updates::*;
use crate::app::music_app::MusicApp;
use karbon_ui::intl::t;
use crate::models::layout::Update;

/// Flatpak : Flathub tient l'application à jour.
pub fn flathub() -> bool {
    std::env::var_os("FLATPAK_ID").is_some()
}

impl MusicApp {
    /// Où en sont les mises à jour, en une ligne (carte « version » et
    /// « À propos »).
    pub fn update_label(&self) -> String {
        if flathub() {
            return t("settings.update_flathub");
        }
        let banners = &self.views.banners;
        if banners.checking {
            return t("settings.update_checking");
        }
        match &banners.update {
            Update::Available(version) => t("settings.update_available").replace("{version}", version),
            Update::Downloading(_, done, Some(total)) if *total > 0 => format!("{} {} %", t("settings.update_downloading"), done * 100 / total),
            Update::Downloading(..) => t("settings.update_downloading"),
            Update::Installing => t("settings.update_installing"),
            Update::Ready(_) => t("settings.update_ready"),
            Update::Error(_) => t("settings.update_failed"),
            Update::Idle => t(match banners.last_check {
                LastCheck::UpToDate => "settings.update_latest",
                LastCheck::Available => "settings.update_pending",
                LastCheck::Failed => "settings.update_failed",
                LastCheck::Never => "settings.update_unchecked",
            }),
        }
    }

    /// Une version est prête à être installée (bouton « Installer » de la carte « version »).
    pub fn update_installable(&self) -> bool {
        matches!(self.views.banners.update, Update::Available(_))
    }
}
