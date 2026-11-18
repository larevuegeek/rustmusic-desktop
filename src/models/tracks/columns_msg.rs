use karbon_ui::ui::drag_drop::PanEvent;
use crate::models::tracks::TrackMenuAt;

#[derive(Debug, Clone, PartialEq)]
pub enum ColumnsMsg {
    Open,
    Close,
    Filter(String),
    Toggle(String),
    Move(usize, bool),
    Reset,
    ResetWidths,
    /// Poignée de l'en-tête tirée : la colonne suit le pointeur, enregistrée au relâchement.
    Resize(String, PanEvent),
    /// Ajustée au contenu de la liste affichée.
    Fit(String),
    /// Rendue à sa largeur d'origine (double-clic sur la poignée ; automatique pour le titre).
    Original(String),
    /// Clic droit sur l'en-tête d'une colonne.
    Menu(String, TrackMenuAt),
    CloseMenu,
    Apply,
}
