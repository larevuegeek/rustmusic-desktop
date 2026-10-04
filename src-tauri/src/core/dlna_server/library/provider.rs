//! Trait that abstracts the data source the DLNA server browses.
//!
//! The DLNA server itself doesn't know about SQLite, Tauri state, or the
//! existing `library_*` repositories. It only knows it can call this trait
//! to enumerate artists, albums, tracks, and resolve a track/cover to a
//! filesystem path for streaming.
//!
//! Phase 1 implementation: `SqliteLibraryProvider` (sister file) backed by
//! the existing repositories. Future implementations could expose playlists,
//! liked tracks, or completely different data sources.

use std::path::PathBuf;

use async_trait::async_trait;

use crate::core::dlna_server::error::DlnaError;

/// Lightweight library record (the top-level RustMusic concept).
/// One DLNA root container per library when the user has multiple.
#[derive(Debug, Clone)]
pub struct DlnaLibrary {
    pub id: i64,
    pub name: String,
    pub artist_count: u32,
    pub album_count: u32,
    pub track_count: u32,
}

/// Lightweight artist record returned by the provider.
#[derive(Debug, Clone)]
pub struct DlnaArtist {
    pub id: String,
    pub name: String,
    pub album_count: u32,
}

/// Lightweight album record.
#[derive(Debug, Clone)]
pub struct DlnaAlbum {
    pub id: String,
    pub title: String,
    pub artist: Option<String>,
    pub year: Option<u16>,
    pub track_count: u32,
    /// `true` when the album has a resolvable cover (own `cover_url` or
    /// a fallback track thumbnail). Drives whether DIDL emits
    /// `<upnp:albumArtURI>`.
    pub has_cover: bool,
}

/// Lightweight track record.
#[derive(Debug, Clone)]
pub struct DlnaTrack {
    pub id: String,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub track_number: Option<u16>,
    pub duration_seconds: Option<u32>,
    pub sample_rate: Option<u32>,
    pub bits_per_sample: Option<u32>,
    pub bitrate: Option<u32>,
    pub channels: Option<u8>,
    pub file_size: Option<u64>,
    pub mime_type: String,
    pub format: String,
    /// `true` when this track has a resolvable thumbnail (cache-side).
    /// Drives whether DIDL emits `<upnp:albumArtURI>`.
    pub has_cover: bool,
}

/// Which kind of cover to resolve (for `cover_path`).
#[derive(Debug, Clone, Copy)]
pub enum CoverKind {
    Album,
    Artist,
    Track,
}

/// A folder entry in the filesystem-based hierarchy.
/// Path is the absolute filesystem path; `name` is the basename.
#[derive(Debug, Clone)]
pub struct DlnaFolder {
    /// Absolute filesystem path (will be base64-encoded into DLNA object IDs).
    pub path: String,
    /// Basename for display.
    pub name: String,
}

/// Result of listing a folder : its subfolders + the audio tracks it contains.
#[derive(Debug, Clone, Default)]
pub struct FolderEntries {
    pub subfolders: Vec<DlnaFolder>,
    pub tracks: Vec<DlnaTrack>,
}

/// The contract every data source must satisfy to be exposed via DLNA.
/// `async_trait` is used so we can call sqlx async queries while keeping
/// the trait `dyn`-compatible (`Arc<dyn LibraryProvider>`).
#[async_trait]
pub trait LibraryProvider: Send + Sync {
    /// All libraries the provider exposes (one DLNA root container per lib).
    async fn list_libraries(&self) -> Result<Vec<DlnaLibrary>, DlnaError>;

    /// All artists in the given library.
    async fn list_artists(&self, library_id: i64) -> Result<Vec<DlnaArtist>, DlnaError>;

    /// All albums in the given library.
    async fn list_albums(&self, library_id: i64) -> Result<Vec<DlnaAlbum>, DlnaError>;

    /// All albums of a given artist within the library.
    async fn list_albums_by_artist(
        &self,
        library_id: i64,
        artist_id: &str,
    ) -> Result<Vec<DlnaAlbum>, DlnaError>;

    /// All tracks of a given album, sorted by track number.
    async fn list_tracks_by_album(
        &self,
        library_id: i64,
        album_id: &str,
    ) -> Result<Vec<DlnaTrack>, DlnaError>;

    /// Resolve a track id to its filesystem path (for streaming).
    /// Track IDs are unique across libraries, so no library_id needed.
    async fn track_path(&self, track_id: &str) -> Result<PathBuf, DlnaError>;

    /// Resolve a cover image to its filesystem path.
    async fn cover_path(&self, kind: CoverKind, id: &str) -> Result<PathBuf, DlnaError>;

    /// Top-level folders : the directories the user imported into the library.
    async fn list_root_folders(&self, library_id: i64) -> Result<Vec<DlnaFolder>, DlnaError>;

    /// List subfolders and audio tracks of the given filesystem path.
    /// The implementation must validate that `path` is inside an imported
    /// library_dir of the given library to prevent path traversal.
    async fn list_folder_entries(
        &self,
        library_id: i64,
        path: &str,
    ) -> Result<FolderEntries, DlnaError>;

    /// Langue de l'app (fr, en, es, de, it) pour les titres des menus ; français par défaut, comme l'app.
    async fn language(&self) -> String {
        "fr".to_string()
    }
}
