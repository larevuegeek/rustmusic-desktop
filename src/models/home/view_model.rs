use crate::models::home::*;

/// Tout ce que la vue lit, construit une fois par rendu.
pub struct ViewModel {
    pub hero: Vec<Record>,
    /// Le hero montre le morceau du lecteur (sinon le dernier écouté).
    pub hero_loaded: bool,
    /// Position affichée par le hero : celle du lecteur, ou la reprise du dernier écouté.
    pub hero_position: f64,
    /// La position du hero : « 01:12 ».
    pub hero_position_label: String,
    /// Position et durée du lecteur : « 1:12 », « 3:42 ».
    pub position_label: String,
    pub duration_label: String,
    /// Le morceau du lecteur est aimé.
    pub liked: bool,
    pub details: Record,
    pub statut: Statut,
    pub up_next: Vec<Card>,
    pub greeting: String,
    pub first_name: String,
    /// « Bonsoir, David ».
    pub greeting_line: String,
    pub initial: String,
    pub profile_color: String,
    pub stats: Vec<Stat>,
    pub mixes: Vec<Mix>,
    pub home_playlists: Vec<PlaylistRow>,
    pub sidebar_playlists: Vec<PlaylistRow>,
    pub genres: Vec<GenreTile>,
    pub recently_added: Vec<AlbumCard>,
    pub featured: Vec<Featured>,
    pub strip: Vec<AlbumStrip>,
    pub outputs: Vec<Output>,
    pub output: usize,
    /// La sortie choisie, ou « Sortie par défaut ».
    pub output_label: String,
    /// Infobulle du bouton de sortie : « Sortie audio : … ».
    pub output_title: String,
    /// Infobulle du volume : « 80 % ».
    pub volume_label: String,
    /// Infobulle du volume du mini-lecteur : « Volume : 80 % ».
    pub volume_title: String,
    pub sources: Vec<Source>,
    pub current_source: Source,
    /// Plusieurs sources : la barre latérale propose d'en changer.
    pub many_sources: bool,
    pub libraries: Vec<Library>,
    pub library: usize,
    /// La bibliothèque choisie, seule dans sa liste (vide sans bibliothèque).
    pub current_library: Vec<Library>,
    /// Sous le nom de la bibliothèque : « 1 234 titres · 3 biblio. ».
    pub library_summary: String,
    pub results: Vec<SearchHit>,
    pub tiles: Vec<Tile>,
    pub tabs: [bool; 7],
    pub scan: String,
    /// Import ou scan en cours : chargeur et pastille de la barre d'état.
    pub import: Option<crate::models::library::ImportProgressView>,
    pub podcast_covers: Vec<String>,
    pub podcasts_detail: String,
    pub radios_detail: String,
    pub radio_logo: Option<String>,
    pub exclusive: bool,
    pub dop: bool,
    pub network: [bool; 3],
    pub open_buttons: bool,
    pub shortcuts: [bool; 2],
    pub home_empty: bool,
    /// Accueil sans bibliothèque : étapes et sections à venir (vides sinon).
    pub welcome_steps: Vec<WelcomeStep>,
    pub welcome_sections: Vec<WelcomeSection>,
    /// Bibliothèque en cours de chargement : squelette à la place des sections.
    pub home_loading: bool,
    pub show_recent: bool,
}
