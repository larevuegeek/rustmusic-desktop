use crate::models::app::Msg;
use karbon_ui::ui::scroll::ScrollRequest;

/// Titre, sous-titre, cartes ; la zone défile à l'horizontale.
pub struct Carousel<T> {
    pub id: &'static str,
    pub title: String,
    pub subtitle: String,
    pub items: Vec<T>,
    pub total: usize,
    /// Génération de la zone : une autre page en bâtit une nouvelle, au début.
    pub view: u64,
    /// Flèches actives : la zone n'est pas au début, des cartes dépassent à droite.
    pub back: bool,
    pub forward: bool,
    /// Glissement demandé par une flèche.
    pub request: Option<ScrollRequest>,
    /// Tri « Année / Titre » de l'en-tête (autres albums de l'artiste) : année choisie.
    pub by_year: Option<bool>,
    /// Messages des deux choix.
    pub by_year_msg: Option<(Msg, Msg)>,
}
