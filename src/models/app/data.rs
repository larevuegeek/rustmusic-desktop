use crate::models::app::*;
use std::collections::{HashMap, HashSet};
use crate::commands::audio::device_command::AudioDeviceInfo;
use crate::commands::library::library_command::GenreMix;
use crate::entity::library::genre_view::GenreView;
use crate::entity::library::library::Library;
use crate::entity::library::library_stats::LibraryStats;
use crate::entity::playlist::playlist::Playlist;
use crate::entity::podcast::podcast::PodcastView;
use crate::entity::profile::profile::Profile;
use crate::entity::radio::radio::RadioView;
use crate::entity::search::search_result::SearchResult;
use crate::entity::server::server::ServerEntry;
use crate::query::track::track_list_row::TrackListRow;
use crate::query::queue::queue_state_row::QueueStateRow;
use crate::query::recent::recent_file_list_row::RecentFileListRow;

#[derive(Default)]
pub struct Data {
    pub profiles: Vec<Profile>,
    pub profile: Option<Profile>,
    pub settings: HashMap<String, String>,
    pub libraries: Vec<Library>,
    pub library_id: Option<i64>,
    pub last_scan: Option<String>,
    pub scan: Option<ScanProgress>,

    pub stats: Option<LibraryStats>,
    pub genres: Vec<GenreView>,
    pub mix_genres: Vec<GenreMix>,
    pub albums: Vec<crate::query::album::album_list_row::AlbumListRow>,
    pub artists: Vec<crate::query::artist::artist_list_row::ArtistListRow>,
    pub mixes: HashMap<String, MixDraw>,
    /// Quatre morceaux au hasard pour compléter « Ensuite » (`get_mix_tracks hasard`).
    pub random: Vec<TrackListRow>,

    pub playlists: Vec<Playlist>,
    /// `get_sidebar_covers` : `playlist:<id>`, `liked`, `recent` → pochettes.
    pub covers: HashMap<String, Vec<String>>,
    pub recent: Vec<RecentFileListRow>,
    pub liked: HashSet<String>,

    pub queue: QueueStateRow,
    pub player: PlayerState,
    pub volume: u8,
    pub devices: Vec<AudioDeviceInfo>,

    pub servers: Vec<ServerEntry>,
    /// `servers_check` : serveur joignable ou non.
    pub server_online: HashMap<i64, bool>,
    pub podcasts: Vec<PodcastView>,
    pub radios: Vec<RadioView>,

    pub search: Vec<SearchResult>,
    /// Texte qui a donné `search` (choix du meilleur résultat de la recherche rapide).
    pub search_query: String,
    /// Les dossiers de la bibliothèque choisie (en-tête, menu « Gérer »).
    pub library_dirs: Vec<crate::entity::library::library_dirs::LibraryDir>,
    /// La page ouverte hors accueil et ses données.
    pub page: Option<crate::models::tracks::Page>,
    /// Événements du moteur dont l'interface ne fait rien encore : le dernier, affiché
    /// dans la barre d'état pour le voir passer.
    pub last_event: Option<String>,
    /// Les données de la bibliothèque (albums, genres, statistiques) sont en route :
    /// l'accueil montre son squelette.
    pub library_loading: bool,
}
