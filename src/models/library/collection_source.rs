/// D'où viennent les morceaux.
#[derive(Debug, Clone, PartialEq)]
pub enum CollectionSource {
    Album { id: String, artist: Option<String> },
    Genre(String),
    Composer(String),
    /// Années incluses (une décennie, ou une seule année).
    Period(i64, i64),
    Playlist(i64),
}
