use crate::models::genres::*;
use karbon_ui::ui::prelude::*;
use crate::models::app::Choice;
use crate::models::albums::AlbumTrack;
use crate::models::albums::SortChoice;
use crate::models::albums::AlbumItem;
use crate::models::artists::ArtistItem;
use crate::models::tracks::Table;

pub struct GenreView {
    pub composer: bool,
    /// Compositeur : sa photo (ou celle de l'artiste du même nom), sinon ses initiales.
    pub portrait: Option<String>,
    pub avatar: (String, f32),
    /// Compositeur aussi artiste : sa page.
    pub artist_page: Option<(String, String)>,
    /// Nombres des onglets (albums, morceaux, interprètes) du sélecteur de compositeur.
    pub mode_counts: Vec<String>,
    pub name: String,
    pub kicker: String,
    pub back: String,
    pub empty: String,
    pub artists_desc: String,
    pub period: Option<PeriodHead>,
    pub name_long: bool,
    /// Teinte (degrés) et intensité (0..1) de la pochette : --h et --s de `.album-lueur`.
    pub tint: (f32, f32),
    pub color: Color,
    pub covers: [Option<String>; 4],
    pub tiles: [Vec<Background>; 4],
    pub icon: &'static str,
    pub albums_n: Option<(String, String)>,
    pub tracks_n: (String, String),
    pub artists_n: Option<(String, String)>,
    pub duration: String,
    pub year_span: Option<String>,
    pub has_tracks: bool,
    pub top_artists: Vec<(String, String)>,
    pub tabs: Vec<Choice>,
    pub view: &'static str,
    pub album_sorts: Vec<Choice>,
    pub sorts: Vec<SortChoice>,
    pub albums: Vec<AlbumItem>,
    pub artists: Vec<ArtistItem>,
    /// Les morceaux de la grille, ligne par ligne.
    pub tracks: Vec<AlbumTrack>,
    pub table: Option<Table>,
    pub loading: bool,
}
