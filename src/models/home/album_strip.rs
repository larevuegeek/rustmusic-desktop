/// Un album écouté avant.
#[derive(Debug, Clone)]
pub struct AlbumStrip {
    pub key: String,
    pub title: String,
    pub artist_name: String,
    pub listen: String,
    /// « Artiste · il y a 2 jours », ou l'écoute seule sans artiste.
    pub subtitle: String,
    pub tracks_list: String,
    pub cover_path: Option<String>,
    pub path: String,
}
