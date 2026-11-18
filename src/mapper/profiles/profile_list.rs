use crate::models::profiles::*;
use crate::helper::color::hex;
use karbon_ui::intl::t;

/// `PALETTE`.
pub const PALETTE: [&str; 10] = ["#22c55e", "#8b5cf6", "#0ea5e9", "#f59e0b", "#f43f5e", "#6366f1", "#84cc16", "#f97316", "#ec4899", "#14b8a6"];

/// Deux initiales au plus, en capitales.
pub fn initials(name: &str) -> String {
    let base = if name.trim().is_empty() { "A" } else { name };
    base.split(' ').filter_map(|w| w.chars().next()).take(2).collect::<String>().to_uppercase()
}

/// `profile.role === 'admin'`.
pub fn is_admin(p: &crate::entity::profile::profile::Profile) -> bool {
    matches!(p.role, crate::entity::profile::profile::ProfileRole::Admin)
}

/// Pour les gabarits.
pub fn palette() -> [&'static str; 10] {
    PALETTE
}

/// Séparateur : `linear-gradient(to right, transparent, color 25, transparent)`.
pub fn rule(color: &str) -> Vec<karbon_ui::ui::prelude::Background> {
    use karbon_ui::ui::prelude::{Color, GradientStop, LinearGradient};
    let c = hex(color);
    let stop = |position: f32, alpha: u8| GradientStop { position, color: Color { alpha, ..c } };
    vec![LinearGradient::new(0., [stop(0., 0), stop(0.5, 37), stop(1., 0)]).into()]
}

/// Le profil 1 (administrateur d'origine) ne se supprime pas.
pub fn can_delete(s: &ProfileScreen) -> bool {
    matches!(s.mode, ProfileMode::Edit(id) if id != 1)
}

impl ProfileScreen {
    pub fn color_hex(&self) -> &'static str {
        PALETTE[self.color.min(PALETTE.len() - 1)]
    }

    /// Le mot à taper pour supprimer (`profile.delete_word`), en minuscules.
    pub fn delete_word() -> String {
        t("profil.delete_word").to_lowercase()
    }

    pub fn delete_ready(&self) -> bool {
        self.delete_text.trim().to_lowercase() == Self::delete_word()
    }

    /// L'avertissement coupé autour du mot.
    pub fn warning() -> (String, String) {
        let w = t("profil.delete_warning");
        let mut parts = w.splitn(2, "{word}");
        (parts.next().unwrap_or_default().to_string(), parts.next().unwrap_or_default().to_string())
    }
}
