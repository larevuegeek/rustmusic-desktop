use crate::models::app::*;
use karbon_ui::ui::runtime::Key;
use crate::models::app::Menu;
use crate::models::app::SleepTimer;
use crate::models::app::Route;

#[rustfmt::skip]
#[derive(Debug, Clone, PartialEq)]
pub enum Msg {
    Noop,

    // ─── Navigation ───
    Go(Route),
    Back,
    /// Retour des réglages : la page d'avant les réglages, quelle que soit la section.
    SettingsBack,
    Forward,

    // ─── Fenêtre et barre de titre ───
    Minimize, Maximize, Close, Quit,
    /// Relance l'application (mode de rendu changé).
    Restart,
    /// Mini-lecteur et micro-lecteur.
    Mini(crate::models::player::MiniMsg),
    ToggleSidebar, ToggleTheme, ToggleFooter,

    // ─── Menus ───
    /// Ouvre un menu, ou le ferme s'il est déjà ouvert (son déclencheur).
    Menu(Menu),
    /// `onclose` du popover ouvert.
    CloseMenu,

    // ─── Recherche ───
    Search(String),
    /// Saisie dans un champ de page.
    Input(Field, String),
    /// Pages de bibliothèque : en-tête, morceaux.
    Library(crate::models::library::LibraryMsg),
    Tracks(crate::models::tracks::TracksMsg),
    Albums(crate::models::albums::AlbumsMsg),
    Album(crate::models::albums::AlbumMsg),
    Artist(crate::models::artists::ArtistMsg),
    Genres(crate::models::genres::GenresMsg),
    Genre(crate::models::genres::GenreMsg),
    Years(crate::models::years::YearsMsg),
    Folders(crate::models::folders::FoldersMsg),
    Mixes(crate::models::mixes::MixesMsg),
    Playlists(crate::models::playlists::PlaylistsMsg),
    Playlist(crate::models::playlists::PlaylistMsg),
    SearchPage(crate::models::search::SearchMsg),
    Radios(crate::models::radios::RadiosMsg),
    Podcasts(crate::models::podcasts::PodcastsMsg),
    Show(crate::models::podcasts::ShowMsg),
    Downloads(crate::models::podcasts::DownloadsMsg),
    Settings(crate::models::settings::SettingsMsg),
    AmpRemote(crate::models::amp_remote::AmpRemoteMsg),
    PlayArtist(String),
    /// Carrousel : une page de cartes en avant ou en arrière (identifiant, avant, nombre).
    Slide(&'static str, bool, usize),
    /// Carrousel : sa zone a défilé ou changé de taille.
    CarouselScrolled(&'static str, crate::models::shared::CarouselGeometry),
    EnqueueArtist(String),
    PlayComposer(String),
    EnqueueComposer(String),
    Artists(crate::models::artists::ArtistsMsg),
    /// Lignes et en-tête de la table des morceaux.
    Row(crate::models::tracks::RowMsg),
    SearchKey(Key),
    /// Résultat choisi (index dans `Data::search`).
    SearchPick(usize),
    SearchAll,

    // ─── Lecteur ───
    /// Lecture / pause du morceau chargé ; sans morceau, le premier de la file.
    TogglePlay,
    Stop,
    Next,
    Previous,
    Seek(f64),
    SeekBy(f64),
    Volume(f64),
    /// Préampli du Replay Gain : pendant le glissement, puis au relâchement.
    Preamp(f64),
    /// Fenêtre (`Popin`) : fermer, ou confirmer son action.
    ClosePopin,
    PopinConfirm,
    /// Toast fermé par ✕.
    ToastClose(u64),
    /// Nom d'une première bibliothèque, saisi sur la page d'import.
    ImportName(String),
    /// Fiche d'un morceau.
    Track(crate::models::tracks::TrackMsg),
    /// Atelier de tags.
    Tags(crate::models::tags::TagsMsg),
    PreampSet(f64),
    Mute,
    /// Aime ou n'aime plus ce morceau (chemin).
    Like(String),
    Shuffle,
    Repeat,
    /// Un morceau de la file, par index.
    PlayQueue(usize),
    /// Volet de la file : vider, retirer un titre (`queue_id`), défilement de la liste
    /// (décalage et décalage maximal : fenêtre qui glisse).
    QueueClear,
    QueueRemove(String),
    QueueScrolled(f32, f32),
    /// Pochette du morceau en cours agrandie, ou refermée.
    CoverZoom(bool),
    /// Menu du clic droit sur la pochette agrandie, ou fermé.
    CoverMenu(Option<crate::models::tracks::TrackMenuAt>),
    /// Copie de la pochette agrandie dans le presse-papiers.
    CoverCopy,
    /// Détail de la chaîne audio, sous la qualité du lecteur : ouvert ou refermé.
    ChainMenu,
    /// Ce texte dans le presse-papiers.
    CopyText(String),
    /// Menu contextuel d'un morceau.
    TrackMenu(crate::models::tracks::TrackMenuMsg),
    /// Menu d'une collection : ouverture (position, titre, source), puis ses actions.
    CollOpen(crate::models::tracks::TrackMenuAt, String, crate::models::library::CollectionSource),
    CollectionMenu(crate::models::library::CollectionMenuMsg),
    /// Menu d'un podcast.
    PodcastMenu(crate::models::podcasts::PodcastMenuMsg),
    /// Menu d'une playlist.
    PlaylistMenu(crate::models::playlists::PlaylistMenuMsg),
    /// Barre latérale : menus des lignes et des sections, épingles, « Organiser ».
    Sidebar(crate::models::sidebar::SidebarMsg),
    /// Paroles : chercher à nouveau (en ligne), réessayer après une erreur.
    LyricsRefresh,
    LyricsRetry,
    /// Panneau des lots : annuler, fermer.
    BatchCancel,
    BatchDismiss,
    /// Formulaire d'une playlist (création, modification).
    PlaylistForm(crate::models::playlists::PlaylistFormMsg),
    /// Les nouveautés de la version.
    WhatsNew(crate::models::updates::WhatsNewMsg),
    /// Playlist auto ou mix : règles, recettes, compte vivant.
    Smart(crate::models::playlists::SmartMsg),
    /// L'écran des profils.
    Profiles(crate::models::profiles::ProfileMsg),
    /// Confirmation au mot tapé (réinitialiser, retirer un serveur).
    Typed(crate::models::layout::TypedMsg),
    /// Fenêtres de serveur (ajout, dossiers).
    ServerForm(crate::models::servers::ServerFormMsg),
    /// Sélecteur de dossiers DLNA.
    Dlna(crate::models::servers::DlnaMsg),
    /// Aperçu d'une émission.
    PodcastPreview(crate::models::podcasts::PodcastPreviewMsg),
    /// Fiche d'une sortie audio.
    Device(crate::models::settings::DeviceMsg),
    /// Éditeur de tags d'un fichier.
    TagEditor(crate::models::tags::TagEditorMsg),
    /// Sa recherche Deezer / MusicBrainz.
    Lookup(crate::models::tags::LookupMsg),
    /// Recherche de pochette.
    Cover(crate::models::albums::CoverMsg),
    /// Créer, renommer, supprimer une bibliothèque.
    LibraryForm(crate::models::library::LibraryFormMsg),
    /// Mode sélection.
    Selection(crate::models::tracks::SelectionMsg),
    /// Cartes de soutien et de mise à jour.
    Banner(crate::models::layout::BannerMsg),
    /// Colonnes du tableau des morceaux.
    Columns(crate::models::tracks::ColumnsMsg),
    /// Dossiers d'une bibliothèque.
    LibraryFolders(crate::models::library::LibraryFoldersMsg),
    /// Vérification des artistes.
    ArtistCheck(crate::models::artists::ArtistCheckMsg),
    /// Nettoyage des tags de l'atelier.
    Clean(crate::models::tags::CleanMsg),
    /// Historique des lots.
    History(crate::models::tags::HistoryMsg),
    /// Renommer les fichiers de l'atelier.
    Rename(crate::models::tags::RenameMsg),
    /// Tags de plusieurs fichiers.
    BatchEditor(crate::models::tags::BatchEditorMsg),
    /// Compléter en ligne (atelier).
    Source(crate::models::tags::SourceMsg),
    /// Un morceau par son chemin, la file remplacée par `queue` (chemins) s'il y en a.
    PlayPath(String),
    /// Un morceau récent : la file devient l'historique.
    PlayRecent(String),
    /// Un album entier, par identifiant, depuis le début.
    PlayAlbum(String),
    /// L'album de ce morceau (chemin) : depuis le début (`true`) ou depuis lui.
    PlayAlbumOf(String, bool),
    /// Le mix « Pour vous » numéro `n`.
    PlayMix(usize),
    /// Un morceau au hasard de « Ensuite » (chemin), la file devient le tirage.
    PlayRandom(String),

    // ─── Minuteur de veille ───
    SleepTimer(Option<SleepTimer>),
    SleepInput(String),
    SleepCustom,

    // ─── Barre latérale ───
    /// Index dans les sources (`viewmodel::sources`).
    ChooseSource(usize),
    /// Bibliothèque par identifiant.
    ChooseLibrary(i64),
    TabPlacement(u8),
    /// Index dans `Section::ALL`.
    ToggleTab(usize),
    HideOpen,
    /// 0 : Titres likés, 1 : Récemment joués.
    ToggleShortcut(usize),
    Rescan,
    /// Arrête l'import ou le scan en cours (ce qui est importé reste).
    StopScan,
    /// Page d'import : dossier ou fichiers à importer (`ImportWelcome.importer`).
    AddDirectory,
    AddFiles,
    /// Barre latérale : lire un fichier ou un dossier.
    OpenFile,
    OpenFolder,
    /// Accueil, « Ajouter de la musique ».
    AddMusic,

    // ─── Sortie audio et réseau ───
    /// Index dans `Data::devices`.
    ChooseOutput(usize),
    ToggleExclusive,
    ToggleDop,
    /// 0 : lecteurs UPnP/DLNA, 1 : AirPlay, 2 : serveur DLNA.
    ToggleNetwork(usize),
    /// Relance la recherche des lecteurs réseau.
    SearchNetworkPlayers,
    /// Index dans les lecteurs réseau du menu de sortie.
    ChooseNetworkPlayer(usize),

    // ─── Accueil ───
    RenewMixes,
    /// Un morceau de la page ouverte, par index : la file devient la page.
    PlayPage(usize),
    /// Une radio favorite, par identifiant.
    PlayRadio(i64),
    /// Bascule un réglage booléen (clé, valeur par défaut).
    ToggleSetting(&'static str, bool),
    /// Montre ce fichier dans le gestionnaire de fichiers du système.
    Reveal(String),
    ClearHistory,
}
