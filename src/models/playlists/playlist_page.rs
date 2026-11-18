use crate::models::app::Route;
use crate::query::track::track_list_row::TrackListRow;

pub struct PlaylistPage {
    pub route: Route,
    pub title: String,
    pub kicker: String,
    pub color: String,
    pub icon: String,
    pub smart: bool,
    pub server: bool,
    pub tracks: Vec<TrackListRow>,
    /// Ligne de playlist de chaque morceau (identifiant de piste → ligne) : c'est elle qu'on retire.
    pub items: Vec<(String, i64)>,
    pub loading: bool,
    pub failed: bool,
    pub sort: String,
    pub asc: bool,
    pub confirm: bool,
}
