use crate::models::layout::*;
use crate::app::music_app::MusicApp;

impl Update {
    /// Le message d'une erreur de mise à jour, vide pour un autre état.
    pub fn error_text(&self) -> &str {
        if let Update::Error(e) = self { e } else { "" }
    }

    /// Téléchargé, total et pourcentage.
    pub fn progress(&self) -> (String, f32) {
        let mb = karbon_ui::intl::bytes;
        match self {
            Update::Downloading(_, d, Some(t)) if *t > 0 => (format!("{} / {} · {}%", mb(*d), mb(*t), d * 100 / t), *d as f32 / *t as f32),
            Update::Downloading(_, d, _) => (mb(*d), 0.),
            _ => (String::new(), 0.),
        }
    }
}

impl MusicApp {
    /// La carte de soutien est visible (jamais en même temps que celle de mise à jour).
    pub fn support_visible(&self) -> bool {
        self.views.banners.support && self.views.banners.update == Update::Idle
    }
}
