use crate::models::playlists::*;
use crate::helper::color::hex;
use karbon_ui::intl::t;

pub const COLORS: [&str; 10] = ["#8b5cf6", "#22c55e", "#0ea5e9", "#f59e0b", "#f43f5e", "#6366f1", "#ec4899", "#14b8a6", "#f97316", "#84cc16"];

pub const ICONS: [&str; 12] = [
    "mynaui:music", "mynaui:heart", "mynaui:star", "mynaui:fire", "mynaui:moon", "mynaui:sun", "mynaui:lightning", "mynaui:cloud", "mynaui:coffee",
    "mynaui:leaf", "mynaui:gift", "mynaui:headphones",
];

/// « mynaui:heart » → la traduction de `playlist_icons.heart`.
pub fn icon_label(id: &str) -> String {
    t(&format!("playlist_icons.{}", id.split(':').nth(1).unwrap_or(id)))
}

/// Pour les gabarits : couleurs et icônes proposées.
pub fn colors() -> [&'static str; 10] {
    COLORS
}

pub fn icons() -> [&'static str; 12] {
    ICONS
}

/// Fond de l'aperçu : `linear-gradient(135deg, color-mix(color 12%, transparent), transparent)`.
pub fn preview(color: &str) -> Vec<karbon_ui::ui::prelude::Background> {
    use karbon_ui::ui::prelude::{Color, GradientStop, LinearGradient};
    let c = hex(color);
    vec![LinearGradient::new(
        45.,
        [GradientStop { position: 0., color: Color { alpha: 31, ..c } }, GradientStop { position: 1., color: Color { alpha: 0, ..c } }],
    )
    .into()]
}

impl PlaylistForm {
    pub fn color_hex(&self) -> &'static str {
        COLORS[self.color.min(COLORS.len() - 1)]
    }

    pub fn icon_id(&self) -> &'static str {
        ICONS[self.icon.min(ICONS.len() - 1)]
    }
}
