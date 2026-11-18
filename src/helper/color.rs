//! Couleurs enregistrées en texte (`#rrggbb` des playlists, des profils et des réglages).

use karbon_ui::ui::theme::Color;

/// Vert de RustMusic, couleur par défaut d'une playlist ou d'un profil.
pub const GREEN: Color = Color::hex(0x22c55e);

/// La couleur du texte `#rrggbb` ; le vert de RustMusic si le texte n'en est pas une.
pub fn hex(text: &str) -> Color {
    Color::parse(text).unwrap_or(GREEN)
}
