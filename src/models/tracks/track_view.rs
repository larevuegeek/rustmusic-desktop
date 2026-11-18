use crate::models::tracks::*;
use crate::models::app::Route;
use crate::models::albums::AlbumTrack;
use crate::models::tracks::Tier;

pub struct TrackView {
    pub cover: Option<String>,
    /// Teinte (degrés) et intensité (0..1) de la pochette : --h et --s de `.album-lueur`.
    pub tint: (f32, f32),
    pub quality: Option<(Tier, String)>,
    pub title: String,
    pub title_long: bool,
    /// Artistes crédités, avec leur page s'ils sont dans la bibliothèque.
    pub credits: Vec<(String, Option<Route>)>,
    pub album: Option<(String, Option<Route>)>,
    pub year: Option<(String, Route)>,
    pub duration: Option<String>,
    /// « Piste 3 sur 12 ».
    pub number: Option<(String, Option<String>)>,
    pub listen: String,
    pub added: Option<String>,
    pub liked: bool,
    pub rating: f64,
    /// Résumé de la qualité : intitulé, détail, palier, niveau.
    pub resume: Option<(String, String, Tier, String)>,
    pub audio: Vec<InfoRow>,
    pub tags: Vec<InfoRow>,
    pub remote: bool,
    pub cards: Vec<LinkCard>,
    pub folder: String,
    pub file: String,
    pub scanned: Option<String>,
    /// Le reste de l'album : sous-titre, colonnes, page de l'album.
    pub same: Option<(String, Vec<AlbumTrack>, Vec<AlbumTrack>, Route)>,
    pub prev: Option<Route>,
    pub next: Option<Route>,
}
