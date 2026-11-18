/// Un album ajouté récemment.
#[derive(Debug, Clone)]
pub struct AlbumCard {
    pub id: String,
    pub title: String,
    pub artist_name: String,
    pub year: String,
    /// « Artiste · 1994 », ou l'artiste seul sans année.
    pub subtitle: String,
    pub cover_path: Option<String>,
}
