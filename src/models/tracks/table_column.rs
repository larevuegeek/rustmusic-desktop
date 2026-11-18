use crate::models::tracks::*;
use crate::query::track::track_list_row::TrackListRow;

/// Une colonne : clé enregistrée, intitulé de repli, largeur de départ, valeur.
#[derive(Clone)]
pub struct TableColumn {
    pub key: String,
    pub fallback: String,
    pub width: f32,
    pub flexible: bool,
    pub min_width: f32,
    pub right: bool,
    pub numeric: bool,
    pub widget: Widget,
    pub value: fn(&TrackListRow) -> Option<String>,
}
