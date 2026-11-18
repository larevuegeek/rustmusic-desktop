use crate::models::library::*;
use karbon_ui::ui::prelude::Color;

/// La page Statistiques.
pub struct StatsView {
    /// Teinte du profil : les barres classées.
    pub color: Color,
    /// « Bibliothèque · Musique HD ».
    pub kicker: String,
    /// Durée totale, le chiffre phare (« 25 j 12 h »).
    pub duration: String,
    /// « À raison de 2 h par jour… ».
    pub listen_plan: String,
    /// (valeur, libellé) : morceaux, albums, artistes.
    pub counts: Vec<(String, String)>,
    pub tiles: Vec<StatTile>,
    /// Barre segmentée de la qualité : Hi-Res, Lossless, Lossy (parts non nulles).
    pub quality: Vec<Segment>,
    pub formats: Vec<Bar>,
    pub top_genres: Vec<Bar>,
    pub top_artists: Vec<Bar>,
    pub top_played: Vec<Played>,
}
