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

/// Avatars proposés : icônes Phosphor (duotone) embarquées, posées sur la couleur du
/// profil ; enregistrés `preset:<icône>` dans la colonne `avatar`, à côté des images
/// importées (chemin d'un fichier).
pub const AVATAR_PRESETS: [&str; 12] = [
    "ph:headphones-duotone",
    "ph:vinyl-record-duotone",
    "ph:guitar-duotone",
    "ph:microphone-stage-duotone",
    "ph:piano-keys-duotone",
    "ph:music-notes-duotone",
    "ph:cassette-tape-duotone",
    "ph:speaker-hifi-duotone",
    "ph:radio-duotone",
    "ph:waveform-duotone",
    "ph:equalizer-duotone",
    "ph:metronome-duotone",
];
const PRESET: &str = "preset:";

/// Pour les gabarits.
pub fn avatar_presets() -> [&'static str; 12] {
    AVATAR_PRESETS
}

/// Valeur enregistrée pour l'avatar proposé de rang `i`.
pub fn preset_value(i: usize) -> Option<String> {
    AVATAR_PRESETS.get(i).map(|icon| format!("{PRESET}{icon}"))
}

/// Icône d'un avatar proposé ; `None` pour une image, aucun avatar ou une icône inconnue.
pub fn avatar_preset(avatar: &Option<String>) -> Option<&'static str> {
    let name = avatar.as_deref()?.strip_prefix(PRESET)?;
    AVATAR_PRESETS.iter().copied().find(|icon| *icon == name)
}

/// Chemin d'une image importée ; `None` pour un avatar proposé ou aucun avatar.
pub fn avatar_image(avatar: &Option<String>) -> Option<&String> {
    avatar.as_ref().filter(|a| !a.is_empty() && !a.starts_with(PRESET))
}

/// Fond d'un avatar proposé : la couleur du profil, assombrie vers le bas à droite.
pub fn avatar_fill(color: &str) -> Vec<karbon_ui::ui::prelude::Background> {
    use karbon_ui::ui::prelude::{Color, GradientStop, LinearGradient};
    let c = hex(color);
    let shade = |v: u8| (f32::from(v) * 0.62).round() as u8;
    let dark = Color { red: shade(c.red), green: shade(c.green), blue: shade(c.blue), ..c };
    vec![LinearGradient::new(45., [GradientStop { position: 0., color: c }, GradientStop { position: 1., color: dark }]).into()]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_and_images_are_told_apart() {
        let preset = preset_value(2);
        assert_eq!(avatar_preset(&preset), Some("ph:guitar-duotone"));
        assert_eq!(avatar_image(&preset), None);
        let image = Some("/data/avatars/a.jpg".to_string());
        assert_eq!(avatar_preset(&image), None);
        assert_eq!(avatar_image(&image), image.as_ref());
        // Icône retirée d'une version suivante : ni icône ni image, les initiales.
        let unknown = Some("preset:ph:kazoo".to_string());
        assert_eq!((avatar_preset(&unknown), avatar_image(&unknown)), (None, None));
        assert_eq!(preset_value(99), None);
    }
}
