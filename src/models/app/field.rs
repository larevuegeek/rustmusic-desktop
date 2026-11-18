/// Les champs de saisie des pages : `Msg::Input(field, text)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    TracksSearch,
    AlbumsSearch,
    ArtistsSearch,
    GenresSearch,
    PlaylistsSearch,
    PageSearch,
}
