use crate::models::sidebar::*;

/// Une section dans « Organiser la barre latérale ».
pub struct SectionRow {
    /// Rang dans l'ordre affiché (glisser-déposer, `SidebarMsg::Shift`).
    pub index: usize,
    pub section: Section,
    pub title: String,
    /// Pas la première : flèche « Monter ».
    pub can_move_up: bool,
    /// Pas la dernière : flèche « Descendre ».
    pub can_move_down: bool,
}
