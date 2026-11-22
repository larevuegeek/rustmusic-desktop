/// L'état des pages traduites à l'identique, une par route.
#[derive(Default)]
pub struct Views {
    /// Dialogue de fichiers ouvert et ce que deviendra son choix.
    pub dialog: Option<crate::models::layout::Choice>,
    pub tracks: Option<crate::models::tracks::TracksPage>,
    pub albums: Option<crate::models::albums::AlbumsPage>,
    pub album: Option<crate::models::albums::AlbumPage>,
    pub artists: Option<crate::models::artists::ArtistsPage>,
    pub artist: Option<crate::models::artists::ArtistPage>,
    pub genres: Option<crate::models::genres::GenresPage>,
    pub genre: Option<crate::models::genres::GenrePage>,
    pub years: Option<crate::models::years::YearsPage>,
    pub folders: Option<crate::models::folders::FoldersPage>,
    pub playlists: crate::models::playlists::PlaylistsPage,
    pub playlist: Option<crate::models::playlists::PlaylistPage>,
    pub search: crate::models::search::SearchPage,
    pub radios: crate::models::radios::RadiosPage,
    pub podcasts: crate::models::podcasts::PodcastsPage,
    pub show: Option<crate::models::podcasts::ShowPage>,
    pub downloads: crate::models::podcasts::DownloadsPage,
    pub settings: crate::models::settings::SettingsState,
    /// Télécommande d'ampli : l'état publié par la session et celui du panneau.
    pub amp_remote: crate::models::amp_remote::AmpRemoteState,
    /// La carte dont le menu d'actions est ouvert (page Mix).
    pub mix_menu: Option<String>,
    /// `artist_name:jaquettes` : jaquettes par titre sur la page d'un artiste.
    pub artist_artwork: bool,
    /// Les carrousels de la page ouverte : géométrie et glissement demandé.
    pub carousels: std::collections::HashMap<&'static str, crate::models::shared::CarouselScroll>,
    /// Génération des carrousels : une autre page repart du début de chacun.
    pub carousel_view: u64,
    /// Dernier jeton de glissement demandé (tous carrousels confondus).
    pub carousel_token: u64,
    /// `album:jaquettes` : jaquettes masquées dans la grille des titres d'un album.
    pub without_artwork: bool,
    /// Grille ou liste, commun aux albums, artistes, playlists.
    pub view_mode: crate::models::albums::ViewMode,
    /// La fenêtre ouverte au-dessus de l'application.
    pub popin: Option<crate::models::layout::Popin>,
    /// Les nouveautés : diapo affichée, vues, masquées au lancement.
    pub whats_new: crate::models::updates::WhatsNewState,
    /// L'écran « Qui écoute ? ».
    pub profiles: Option<crate::models::profiles::ProfileScreen>,
    /// La confirmation au mot tapé.
    pub typed: Option<crate::models::layout::TypedConfirm>,
    /// La fiche d'une sortie audio.
    pub device_details: Option<crate::models::settings::DeviceDetails>,
    /// La recherche de pochette chez Deezer.
    pub cover_search: Option<crate::models::albums::CoverSearch>,
    /// Le mode sélection des listes de morceaux.
    pub selection: Option<crate::models::tracks::Selection>,
    /// Chemins envoyés à l'atelier de tags par la barre de sélection.
    pub workshop_paths: Option<Vec<String>>,
    /// Nom de ce qui a envoyé ces chemins (le dossier), sinon « la sélection ».
    pub workshop_source: Option<String>,
    /// Cartes de soutien et de mise à jour.
    pub banners: crate::models::layout::Banners,
    /// Le choix des colonnes du tableau des morceaux.
    pub columns: Option<crate::models::tracks::ColumnsPopin>,
    /// Le menu « Gérer » ouvert depuis la liste de la barre latérale : la bibliothèque
    /// et ses dossiers (lus à l'ouverture).
    pub library_manage: Option<(i64, Vec<crate::entity::library::library_dirs::LibraryDir>)>,
    /// Ce menu « Gérer » est ouvert par-dessus la liste déroulante des bibliothèques (son
    /// bouton « … »), qui reste ouverte.
    pub library_manage_nested: bool,
    /// Les dossiers d'une bibliothèque.
    pub library_folders: Option<crate::models::library::LibraryFolders>,
    /// La vérification des artistes, en cours ou terminée.
    pub artist_check: crate::models::artists::ArtistCheck,
    pub toasts: crate::models::layout::Toasts,
    /// Nom saisi pour une première bibliothèque (page d'import).
    pub import_name: String,
    /// La fiche d'un morceau.
    pub track: Option<crate::models::tracks::TrackPage>,
    /// L'atelier de tags.
    pub tags: Option<crate::models::tags::WorkshopPage>,
    /// Pochette du morceau en cours agrandie (clic sur celle du lecteur).
    pub cover_zoom: bool,
    /// Menu du clic droit sur la pochette agrandie.
    pub cover_menu: Option<crate::models::tracks::TrackMenuAt>,
    /// Menu du clic droit sur l'en-tête d'une colonne du tableau des morceaux.
    pub column_menu: Option<crate::models::tracks::ColumnMenu>,
    /// Titres à venir bâtis dans le volet de la file (ou celui du mini-lecteur).
    pub queue_window: crate::models::shared::ListWindow,
    /// Le menu contextuel d'un morceau ouvert.
    pub track_menu: Option<crate::models::tracks::TrackMenu>,
    /// Le menu d'une collection (album, genre, playlist) ouvert.
    pub collection_menu: Option<crate::models::library::CollectionMenu>,
    /// Le menu d'une station ouvert.
    pub radio_menu: Option<crate::models::radios::RadioMenu>,
    /// Le menu d'un podcast ouvert.
    pub podcast_menu: Option<crate::models::podcasts::PodcastMenu>,
    /// Le menu d'une playlist ouvert.
    pub playlist_menu: Option<crate::models::playlists::PlaylistMenu>,
    /// Barre latérale : épingles, morceau localisé, mode « Organiser », menu d'un épinglé.
    pub sidebar: crate::models::sidebar::SidebarState,
    /// Le panneau des paroles.
    pub lyrics: crate::models::player::LyricsState,
    /// Le lot en cours (écriture de tags).
    pub batch: Option<crate::models::layout::BatchState>,
    /// Le mini-lecteur et le micro-lecteur.
    pub mini: crate::models::player::MiniState,
}
