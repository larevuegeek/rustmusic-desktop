/// D'où viennent les morceaux.
#[derive(Debug, Clone, PartialEq)]
pub enum CollectionSource {
    Album { id: String, artist: Option<String> },
    Genre(String),
    Playlist(i64),
}
