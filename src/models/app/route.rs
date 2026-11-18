use crate::models::app::*;

/// Une page de l'application.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Route {
    Home,
    /// Une section de la bibliothèque choisie.
    Library(Section),
    Album(String),
    Artist(String),
    Genre(String),
    Composer(String),
    Decade(i64),
    Track(String),
    /// Atelier de tags, sur un album (`/tags?album=`).
    Tags(Option<String>),
    /// Un dossier de la bibliothèque (identifiant).
    Folder(String, String),
    Playlists,
    Playlist(i64),
    Liked,
    Recent,
    Podcasts,
    Podcast(i64),
    PodcastDownloads,
    PodcastSubscriptions,
    Radios,
    Search(String),
    Settings(SettingsPage),
    Stats,
    Import,
    Feedback,
}
