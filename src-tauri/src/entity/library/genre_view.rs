use serde::Serialize;

#[derive(Serialize)]
pub struct GenreView {
    pub name: String,
    pub total_albums: i64,
    pub total_tracks: i64,
    pub covers: Vec<String>,
    /// Les trois artistes les plus présents dans le genre.
    pub top_artists: Vec<String>,
    /// Dernière écoute d'un titre du genre.
    pub last_played_at: Option<String>,
}
