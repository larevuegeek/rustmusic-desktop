use serde::Serialize;
use sqlx::FromRow;

/// Un podcast suivi, avec ses compteurs pour la grille.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct PodcastView {
    pub id: i64,
    pub feed_url: String,
    pub title: String,
    pub author: Option<String>,
    /// HTML du flux, nettoyé à l'affichage.
    pub description: Option<String>,
    pub image_url: Option<String>,
    pub cover_path: Option<String>,
    pub link: Option<String>,
    pub checked_at: Option<String>,
    pub auto_download: bool,
    /// Favori : épinglé dans la barre latérale.
    pub pinned: bool,
    pub episode_count: i64,
    pub unplayed_count: i64,
    /// Parus depuis l'abonnement et pas encore écoutés.
    pub new_count: i64,
    pub latest_at: Option<String>,
    /// Date d'abonnement.
    pub created_at: String,
}

/// Un épisode. `description` n'est remplie que pour la fiche d'un épisode.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct EpisodeView {
    pub id: i64,
    pub podcast_id: i64,
    pub title: String,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub published_at: Option<String>,
    pub duration: Option<i64>,
    pub audio_url: String,
    pub audio_type: Option<String>,
    pub audio_size: Option<i64>,
    pub image_url: Option<String>,
    pub season: Option<i64>,
    pub episode: Option<i64>,
    pub file_path: Option<String>,
    pub position: f64,
    pub played: bool,
    pub is_new: bool,
    /// Chemin de l'épisode dans le cache, téléchargé ou non : c'est lui que lit le lecteur.
    pub path: String,
}

/// L'épisode à écouter d'une émission (rangée « À écouter » de l'accueil).
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct LatestEpisode {
    #[sqlx(flatten)]
    #[serde(flatten)]
    pub episode: EpisodeView,
    pub podcast_title: String,
    pub cover_path: Option<String>,
    /// Épisodes de l'émission parus depuis l'abonnement et pas encore écoutés.
    pub new_count: i64,
}

/// Aperçu d'un flux avant l'abonnement : l'émission et ses derniers épisodes.
#[derive(Debug, Clone, Serialize)]
pub struct PodcastPreview {
    pub title: String,
    pub author: Option<String>,
    pub description: Option<String>,
    pub image_url: Option<String>,
    pub link: Option<String>,
    pub episode_count: usize,
    pub episodes: Vec<PreviewEpisode>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PreviewEpisode {
    pub title: String,
    pub summary: Option<String>,
    pub published_at: Option<String>,
    pub duration: Option<i64>,
}

/// Un épisode téléchargé, pour la page Téléchargements.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct DownloadView {
    pub id: i64,
    pub podcast_id: i64,
    pub podcast_title: String,
    pub cover_path: Option<String>,
    pub title: String,
    pub published_at: Option<String>,
    pub duration: Option<i64>,
    pub file_path: String,
    pub position: f64,
    pub played: bool,
    /// Octets sur le disque, lus au moment de la liste.
    #[sqlx(skip)]
    pub size: u64,
}

/// Un épisode prêt à lire : de quoi le télécharger et l'afficher dans le lecteur.
#[derive(Debug, Clone, FromRow)]
pub struct Playable {
    pub title: String,
    pub audio_url: String,
    pub duration: Option<i64>,
    pub position: f64,
    pub played: bool,
    pub podcast_title: String,
    pub author: Option<String>,
    pub cover_path: Option<String>,
}

/// De quoi interroger un flux sans le retélécharger s'il n'a pas changé.
#[derive(Debug, Clone, FromRow)]
pub struct FeedState {
    pub id: i64,
    pub feed_url: String,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub image_url: Option<String>,
    pub cover_path: Option<String>,
}
