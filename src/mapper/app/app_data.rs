//! Les données de l'interface (profil, réglages,
//! bibliothèques, playlists, file, lecteur…), chargé par les commandes du moteur et mis
//! à jour par ses événements. Les vues en dérivent leurs modèles (`viewmodel`).

use crate::models::app::*;
use crate::entity::library::library::Library;
use crate::entity::queue::queue_track::QueueTrack;
use crate::query::queue::queue_state_row::QueueStateRow;
use crate::models::app::Route;
use crate::models::app::Section;
use karbon_ui::intl::{t, t_with};

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            path: None,
            audio: None,
            status: Status::Ready,
            position: 0.,
            duration: 0.,
            preparing: false,
            pipeline: None,
            seek_when_ready: None,
            stale_output: false,
            volume_before_mute: 80,
        }
    }
}

impl Default for QueueStateRow {
    fn default() -> Self {
        Self {
            profile_id: 0,
            current_index: -1,
            is_shuffled: false,
            repeat_mode: "off".into(),
            tracks: Vec::new(),
        }
    }
}

/// Page ouverte par `--page <name>` (aperçu hors écran, ou fenêtre ouverte sur elle).
pub(crate) fn page_route(p: &str) -> Route {
    match p {
        "playlists" => Route::Playlists,
        "settings" => Route::Settings(SettingsPage::General),
        "appearance" => Route::Settings(SettingsPage::Appearance),
        "settings-audio" => Route::Settings(SettingsPage::Audio),
        "settings-network" => Route::Settings(SettingsPage::Network),
        "settings-online" => Route::Settings(SettingsPage::Online),
        "settings-remote" => Route::Settings(SettingsPage::Remote),
        "settings-storage" => Route::Settings(SettingsPage::Storage),
        "settings-shortcuts" => Route::Settings(SettingsPage::Shortcuts),
        "settings-guide" => Route::Settings(SettingsPage::Guide),
        "settings-about" => Route::Settings(SettingsPage::About),
        "import" => Route::Import,
        "feedback" => Route::Feedback,
        "stats" => Route::Stats,
        "liked" => Route::Liked,
        "recent" => Route::Recent,
        "podcasts" => Route::Podcasts,
        "subscriptions" => Route::PodcastSubscriptions,
        "downloads" => Route::PodcastDownloads,
        "radios" => Route::Radios,
        other => match other.split_once(':') {
            Some(("search", q)) => Route::Search(q.to_string()),
            Some(("album", id)) => Route::Album(id.to_string()),
            Some(("track", id)) => Route::Track(id.to_string()),
            Some(("tags", id)) => Route::Tags(Some(id.to_string())),
            Some(("artist", id)) => Route::Artist(id.to_string()),
            Some(("playlist", id)) => Route::Playlist(id.parse().unwrap_or(0)),
            Some(("genre", g)) => Route::Genre(g.to_string()),
            Some(("decade", d)) => Route::Decade(d.parse().unwrap_or(0)),
            Some(("composer", c)) => Route::Composer(c.to_string()),
            Some(("podcast", id)) => Route::Podcast(id.parse().unwrap_or(0)),
            _ => Section::from_key(other).map(Route::Library).unwrap_or(Route::Home),
        },
    }
}

impl Data {
    /// Un réglage, ou sa valeur par défaut.
    pub fn setting(&self, key: &str) -> Option<&str> {
        self.settings.get(key).map(String::as_str)
    }

    pub fn flag(&self, key: &str) -> bool {
        self.setting(key) == Some("true")
    }

    /// Vrai sans réglage.
    pub fn flag_default_on(&self, key: &str) -> bool {
        self.setting(key) != Some("false")
    }

    pub fn profile_id(&self) -> Option<i64> {
        self.profile.as_ref().map(|p| p.id)
    }

    pub fn library(&self) -> Option<&Library> {
        let id = self.library_id?;
        self.libraries.iter().find(|l| l.id == id)
    }

    /// Le morceau courant de la file.
    pub fn current_track(&self) -> Option<&QueueTrack> {
        usize::try_from(self.queue.current_index)
            .ok()
            .and_then(|i| self.queue.tracks.get(i))
    }

    /// Les morceaux de la file après le courant.
    pub fn upcoming(&self) -> impl Iterator<Item = (usize, &QueueTrack)> {
        let start = usize::try_from(self.queue.current_index + 1).unwrap_or(0);
        self.queue.tracks.iter().enumerate().skip(start)
    }

    /// La bibliothèque par défaut, sinon la première.
    pub fn pick_default_library(&self) -> Option<i64> {
        self.libraries
            .iter()
            .find(|l| l.is_default)
            .or_else(|| self.libraries.first())
            .map(|l| l.id)
    }

    /// Les sections de bibliothèque affichées, dans l'ordre de référence.
    pub fn library_tabs(&self) -> Vec<crate::models::app::Section> {
        use crate::models::app::Section;
        let default = || {
            Section::ALL
                .iter()
                .map(|(s, ..)| *s)
                .filter(|s| !matches!(s, Section::Years | Section::Mixes))
                .collect::<Vec<_>>()
        };
        let Some(raw) = self.setting("library_tabs") else {
            return default();
        };
        let Ok(keys) = serde_json::from_str::<Vec<String>>(raw) else {
            return default();
        };
        let mut out: Vec<Section> = Section::ALL
            .iter()
            .map(|(s, ..)| *s)
            .filter(|s| keys.iter().any(|k| k == s.key()))
            .collect();
        if out.is_empty() {
            out = default();
        }
        out
    }

    /// L'onglet « Playlists » est coché dans l'apparence : il mène à la page des playlists,
    /// hors des sections de la bibliothèque.
    pub fn playlists_tab(&self) -> bool {
        self.setting("library_tabs")
            .and_then(|raw| serde_json::from_str::<Vec<String>>(raw).ok())
            .is_some_and(|keys| keys.iter().any(|k| k == "playlists"))
    }

    /// 0 à gauche (barre), 1 en haut, 2 les deux (défaut).
    pub fn tabs_placement(&self) -> u8 {
        match self.setting("library_tabs_position") {
            Some("sidebar") => 0,
            Some("top") => 1,
            _ => 2,
        }
    }
}

impl Section {
    /// Clé, clé de traduction du libellé et icône de chaque onglet, dans l'ordre de référence.
    pub const ALL: [(Section, &'static str, &'static str, &'static str); 7] = [
        (Section::Tracks, "tracks", "library.tracks", "material-symbols:music-note-rounded"),
        (
            Section::Artists,
            "artists",
            "library.artists",
            "material-symbols:mic-external-on-outline-rounded",
        ),
        (Section::Albums, "albums", "library.albums", "material-symbols:album-outline-rounded"),
        (Section::Genres, "genres", "library.genres", "material-symbols:sell-outline-rounded"),
        (
            Section::Years,
            "years",
            "library.years",
            "material-symbols:calendar-month-outline-rounded",
        ),
        (Section::Folders, "folders", "nav.folders", "material-symbols:folder-outline-rounded"),
        (Section::Mixes, "mixes", "library.mixes", "material-symbols:shuffle-rounded"),
    ];

    /// Le libellé traduit de l'onglet.
    pub fn label(self) -> String {
        Self::ALL.iter().find(|(s, ..)| *s == self).map(|(_, _, l, _)| t(l)).unwrap_or_default()
    }

    pub fn key(self) -> &'static str {
        Self::ALL.iter().find(|(s, ..)| *s == self).map(|(_, k, ..)| *k).unwrap_or("tracks")
    }

    pub fn from_key(key: &str) -> Option<Section> {
        Self::ALL.iter().find(|(_, k, ..)| *k == key).map(|(s, ..)| *s)
    }
}

impl Route {
    /// `/settings…` : les pages pleine largeur, sans barre latérale ni recherche.
    pub fn is_full_page(&self) -> bool {
        matches!(self, Route::Settings(_))
    }

    /// La section ouverte, pour la tuile allumée dans la navigation.
    pub fn section(&self) -> Option<Section> {
        match self {
            Route::Library(s) => Some(*s),
            Route::Album(_) => Some(Section::Albums),
            Route::Artist(_) | Route::Composer(_) => Some(Section::Artists),
            Route::Genre(_) => Some(Section::Genres),
            Route::Decade(_) => Some(Section::Years),
            Route::Folder(..) => Some(Section::Folders),
            Route::Track(_) => Some(Section::Tracks),
            Route::Tags(_) => Some(Section::Albums),
            _ => None,
        }
    }

    /// Titre de la page, pour les pages génériques (en-tête, page en attente).
    pub fn title(&self) -> String {
        let named = |key: &str, name: &str| format!("{} {name}", t(key));
        match self {
            Route::Home => t("nav.home"),
            Route::Library(s) => s.label(),
            Route::Album(id) => named("album_view.type_album", id),
            Route::Artist(id) => named("artist_view.kicker", id),
            Route::Genre(g) => named("genre_view.kicker", g),
            Route::Composer(c) => named("composers_view.kicker", c),
            Route::Decade(d) => t_with("home.decade_name", &[("d", &d.to_string())]),
            Route::Track(id) => named("track_view.kicker", id),
            Route::Tags(_) => t("workshop.title"),
            Route::Folder(_, name) => name.clone(),
            Route::Playlists => t("nav.playlists"),
            Route::Playlist(id) => named("home.playlist", &id.to_string()),
            Route::Liked => t("playlist_page.liked_title"),
            Route::Recent => t("playlist_page.recent_title"),
            Route::Podcasts => t("podcasts.title"),
            Route::Podcast(id) => named("podcasts.kicker_podcast", &id.to_string()),
            Route::PodcastDownloads => t("podcasts.downloads"),
            Route::PodcastSubscriptions => t("podcasts.subscriptions"),
            Route::Radios => t("radios.title"),
            Route::Search(q) => t_with("search.title_query", &[("q", q)]),
            Route::Settings(_) => t("sidebar.settings"),
            Route::Stats => t("stats.title"),
            Route::Import => t("settings.import_btn"),
            Route::Feedback => t("feedback.title"),
        }
    }
}

impl Views {
    /// Les pochettes que la page de cette route montre.
    pub fn covers(&self, route: &Route) -> Vec<String> {
        match route {
            Route::Library(Section::Tracks) => self
                .tracks
                .iter()
                .flat_map(|p| p.tracks.iter().filter_map(|t| t.thumbnail_path.as_deref().map(crate::app::resources::assets::small_key)))
                .collect(),
            _ => Vec::new(),
        }
    }
}
