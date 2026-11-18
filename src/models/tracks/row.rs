use crate::models::tracks::*;
use crate::models::app::Route;

/// Une ligne prête à dessiner.
#[derive(Debug, Clone)]
pub struct Row {
    /// Rang dans la liste entière (début de la fenêtre chargée compris) : la clé de mémoire
    /// et les messages d'une ligne ne changent pas quand la fenêtre perd ses premières
    /// lignes.
    pub n: usize,
    /// Première ligne affichée (sans trait de séparation au-dessus).
    pub first: bool,
    pub id: String,
    pub title: String,
    pub artist: String,
    pub artist_route: Option<Route>,
    pub album: String,
    pub album_route: Option<Route>,
    pub year: String,
    pub cover: Option<String>,
    pub number: String,
    pub in_progress: bool,
    pub played: bool,
    pub liked: bool,
    pub tier: Tier,
    pub tier_label: String,
    pub details: String,
    pub short: String,
    pub rating: f64,
    pub rated: bool,
    pub cells: Vec<Cell>,
}
