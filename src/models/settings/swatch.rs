use karbon_ui::ui::prelude::Color;
use crate::models::app::Msg;

/// Une pastille de `AccentPicker`.
#[derive(Debug, Clone)]
pub struct Swatch {
    pub color: Color,
    pub label: String,
    pub selected: bool,
    pub msg: Msg,
}
