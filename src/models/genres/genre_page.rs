use crate::query::track::track_list_row::TrackListRow;
use crate::models::shared::ListWindow;

pub struct GenrePage {
    pub name: String,
    /// Période (décennie, année choisie) ; `None` pour un genre.
    pub period: Option<(i64, Option<i64>)>,
    /// Page d'un compositeur (`name` est son nom).
    pub composer: bool,
    /// Photo du compositeur cherchée sur Deezer (`fetch_composer_image`).
    pub photo: Option<String>,
    pub tracks: Vec<TrackListRow>,
    pub loading: bool,
    pub error: bool,
    pub view: Option<&'static str>,
    pub sort_albums: &'static str,
    pub sort: String,
    pub asc: bool,
    /// Les éléments bâtis de l'onglet ouvert (albums, morceaux ou artistes).
    pub window: ListWindow,
}
