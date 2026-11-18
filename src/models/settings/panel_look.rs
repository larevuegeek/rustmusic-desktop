use karbon_ui::ui::prelude::Color;

/// Couleurs d'une case d'un choix segmenté : fond, bord, texte (active), icône, barres.
pub struct PanelLook {
    pub background: Color,
    pub border: Color,
    pub text: Color,
    pub icon: Option<Color>,
    pub bars: [Color; 4],
}
