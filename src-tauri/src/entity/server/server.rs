use serde::Serialize;
use sqlx::FromRow;

/// Un serveur OpenSubsonic du profil (sans ses identifiants).
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct ServerView {
    pub id: i64,
    /// `subsonic` ou `jellyfin`.
    pub kind: String,
    pub name: String,
    pub url: String,
    pub username: String,
    pub enabled: bool,
    pub server_type: Option<String>,
    pub server_version: Option<String>,
    pub open_subsonic: bool,
    /// Extensions OpenSubsonic, séparées par des virgules.
    pub extensions: String,
    pub transcode_format: Option<String>,
    pub transcode_bitrate: Option<i64>,
    /// Ses playlists épinglées dans la barre latérale.
    pub pin_playlists: bool,
    /// Identifiant réseau (DLNA : UDN).
    pub device_id: Option<String>,
    pub last_sync_at: Option<String>,
    pub created_at: String,
}

/// Une bibliothèque distante d'un serveur.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct ServerLibrary {
    pub id: i64,
    pub name: String,
    pub remote_folder_id: Option<String>,
    pub total_tracks: i64,
}

/// Un serveur et ses bibliothèques, pour les réglages.
#[derive(Debug, Clone, Serialize)]
pub struct ServerEntry {
    #[serde(flatten)]
    pub server: ServerView,
    pub libraries: Vec<ServerLibrary>,
}

/// Identifiants et adresse : de quoi joindre le serveur.
#[derive(Debug, Clone, FromRow)]
pub struct ServerAccess {
    pub kind: String,
    pub url: String,
    pub username: String,
    pub token: Option<String>,
    pub salt: Option<String>,
    pub api_key: Option<String>,
    pub user_id: Option<String>,
}
