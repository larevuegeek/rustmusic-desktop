//! `LibraryProvider` implementation backed by the existing SQLite library.
//!
//! Reuses the project's existing repositories rather than writing parallel
//! SQL — this guarantees the DLNA view stays in sync with what the rest of
//! the app sees (same JOINs, same album/artist deduplication, etc.).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use sqlx::SqlitePool;

use crate::core::dlna_server::error::DlnaError;
use crate::core::dlna_server::library::provider::{
    CoverKind, DlnaAlbum, DlnaArtist, DlnaFolder, DlnaLibrary, DlnaTrack, FolderEntries,
    LibraryProvider,
};
use crate::repository::library::library_album_repository::LibraryAlbumRepository;
use crate::repository::library::library_artist_repository::LibraryArtistRepository;
use crate::repository::library::library_dirs_repository::LibraryDirRepository;
use crate::repository::library::library_files_repository::LibraryFilesRepository;
use crate::repository::library::library_repository::LibraryRepository;
use crate::repository::library::library_track_repository::LibraryTrackRepository;

const AUDIO_EXTENSIONS: &[&str] = &[
    "mp3", "flac", "wav", "aiff", "aif", "ogg", "opus", "m4a", "aac", "alac", "ape", "wma",
    "dsf", "dff",
];

pub struct SqliteLibraryProvider {
    pool: Arc<SqlitePool>,
}

impl SqliteLibraryProvider {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl LibraryProvider for SqliteLibraryProvider {
    async fn list_libraries(&self) -> Result<Vec<DlnaLibrary>, DlnaError> {
        let libs = LibraryRepository::find_all(&*self.pool)
            .await
            .map_err(|e| DlnaError::Library(format!("list_libraries: {e}")))?;

        Ok(libs
            .into_iter()
            .map(|l| DlnaLibrary {
                id: l.id,
                name: l.name,
                artist_count: l.total_artists.max(0) as u32,
                album_count: l.total_albums.max(0) as u32,
                track_count: l.total_tracks.max(0) as u32,
            })
            .collect())
    }

    async fn list_artists(&self, library_id: i64) -> Result<Vec<DlnaArtist>, DlnaError> {
        let artists =
            LibraryArtistRepository::find_all_artists_by_library_id(&*self.pool, library_id)
                .await
                .map_err(|e| DlnaError::Library(format!("list_artists: {e}")))?;

        Ok(artists
            .into_iter()
            .map(|a| DlnaArtist {
                id: a.id,
                name: a.name,
                album_count: a.total_albums.max(0) as u32,
            })
            .collect())
    }

    async fn list_albums(&self, library_id: i64) -> Result<Vec<DlnaAlbum>, DlnaError> {
        let albums = LibraryAlbumRepository::find_all_albums_by_library_id(
            &*self.pool,
            library_id,
            None,
        )
        .await
        .map_err(|e| DlnaError::Library(format!("list_albums: {e}")))?;

        Ok(albums.into_iter().map(album_view_to_dlna).collect())
    }

    async fn list_albums_by_artist(
        &self,
        library_id: i64,
        artist_id: &str,
    ) -> Result<Vec<DlnaAlbum>, DlnaError> {
        let albums =
            LibraryAlbumRepository::find_albums_by_artist_id(&*self.pool, library_id, artist_id, false)
                .await
                .map_err(|e| DlnaError::Library(format!("list_albums_by_artist: {e}")))?;

        Ok(albums.into_iter().map(album_view_to_dlna).collect())
    }

    async fn list_tracks_by_album(
        &self,
        library_id: i64,
        album_id: &str,
    ) -> Result<Vec<DlnaTrack>, DlnaError> {
        let tracks = LibraryTrackRepository::find_all_tracks_album_by_library_id(
            &*self.pool,
            library_id,
            album_id.to_string(),
        )
        .await
        .map_err(|e| DlnaError::Library(format!("list_tracks_by_album: {e}")))?;

        Ok(tracks.into_iter().map(track_view_to_dlna).collect())
    }

    async fn track_path(&self, track_id: &str) -> Result<PathBuf, DlnaError> {
        let track =
            LibraryTrackRepository::find_track_by_id(&*self.pool, track_id.to_string())
                .await
                .map_err(|e| DlnaError::Library(format!("track_path lookup: {e}")))?;
        Ok(PathBuf::from(track.path))
    }

    async fn cover_path(&self, kind: CoverKind, id: &str) -> Result<PathBuf, DlnaError> {
        let resolved: Option<String> = match kind {
            CoverKind::Album => {
                let view = LibraryAlbumRepository::find_album_by_id(&*self.pool, id.to_string())
                    .await
                    .map_err(|e| DlnaError::Library(format!("cover_path album: {e}")))?;
                view.cover_url
                    .filter(is_local_path)
                    .or_else(|| view.thumbnail_path.filter(is_local_path))
            }
            CoverKind::Artist => {
                let view = LibraryArtistRepository::find_artist_by_id(&*self.pool, id.to_string())
                    .await
                    .map_err(|e| DlnaError::Library(format!("cover_path artist: {e}")))?;
                view.thumbnail_path.filter(is_local_path)
            }
            CoverKind::Track => {
                let view = LibraryTrackRepository::find_track_by_id(&*self.pool, id.to_string())
                    .await
                    .map_err(|e| DlnaError::Library(format!("cover_path track: {e}")))?;
                view.thumbnail_path.filter(is_local_path)
            }
        };

        resolved
            .map(PathBuf::from)
            .ok_or_else(|| DlnaError::Library(format!("no cover for {:?} {}", kind, id)))
    }

    async fn list_root_folders(&self, library_id: i64) -> Result<Vec<DlnaFolder>, DlnaError> {
        let dirs = LibraryDirRepository::find_all_by_library_id(&*self.pool, library_id)
            .await
            .map_err(|e| DlnaError::Library(format!("list_root_folders: {e}")))?;

        Ok(dirs
            .into_iter()
            .map(|d| {
                let name = Path::new(&d.path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(&d.name)
                    .to_string();
                DlnaFolder { path: d.path, name }
            })
            .collect())
    }

    async fn list_folder_entries(
        &self,
        library_id: i64,
        path: &str,
    ) -> Result<FolderEntries, DlnaError> {
        // 1. Validate the path is inside one of the imported library_dirs
        //    of the given library (anti path-traversal). We canonicalize ONLY
        //    for this check — the read_dir below uses the raw path so its
        //    entries match what `library_files.path` stores (Windows : avoids
        //    the `\\?\` prefix that canonicalize introduces).
        let dirs = LibraryDirRepository::find_all_by_library_id(&*self.pool, library_id)
            .await
            .map_err(|e| DlnaError::Library(format!("list_folder_entries dirs: {e}")))?;

        let canonical_target = std::fs::canonicalize(path)
            .map_err(|e| DlnaError::Library(format!("invalid path {}: {}", path, e)))?;

        let allowed = dirs.iter().any(|d| {
            std::fs::canonicalize(&d.path)
                .map(|c| canonical_target.starts_with(&c))
                .unwrap_or(false)
        });
        if !allowed {
            return Err(DlnaError::Library(format!(
                "path {} not inside any imported library_dir of library {}",
                path, library_id
            )));
        }

        // 2. Iterate filesystem on the RAW path so entry.path() matches
        //    the format stored in library_files.path.
        let read_dir = std::fs::read_dir(path)
            .map_err(|e| DlnaError::Library(format!("read_dir {}: {}", path, e)))?;

        let mut subfolders: Vec<DlnaFolder> = Vec::new();
        let mut tracks: Vec<DlnaTrack> = Vec::new();

        for entry in read_dir.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            let entry_path = entry.path();
            let path_str = entry_path.to_string_lossy().to_string();

            let file_type = match entry.file_type() {
                Ok(ft) => ft,
                Err(_) => continue,
            };

            if file_type.is_dir() {
                subfolders.push(DlnaFolder {
                    path: path_str,
                    name,
                });
            } else if file_type.is_file() {
                let ext = entry_path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.to_ascii_lowercase());
                let is_audio = ext
                    .as_deref()
                    .map(|e| AUDIO_EXTENSIONS.contains(&e))
                    .unwrap_or(false);
                if !is_audio {
                    continue;
                }

                let Ok(Some(file)) = LibraryFilesRepository::find_by_path(
                    &*self.pool,
                    library_id,
                    &path_str,
                )
                .await
                else {
                    continue;
                };
                let Ok(Some(view)) = LibraryTrackRepository::find_track_view_by_file_id(
                    &*self.pool,
                    &file.id,
                )
                .await
                else {
                    continue;
                };
                tracks.push(track_view_to_dlna(view));
            }
        }

        // Sort : folders first (alphabetical), then tracks by track_number then title
        subfolders.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        tracks.sort_by(|a, b| match (a.track_number, b.track_number) {
            (Some(x), Some(y)) => x.cmp(&y),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.title.to_lowercase().cmp(&b.title.to_lowercase()),
        });

        Ok(FolderEntries { subfolders, tracks })
    }
}

// ─── View → DTO mappers ──────────────────────────────────────────────

fn album_view_to_dlna(
    a: crate::mapper::library::album::album_list_view::AlbumListView,
) -> DlnaAlbum {
    let has_cover = a.cover_url.as_ref().is_some_and(is_local_path);
    DlnaAlbum {
        id: a.id,
        title: a.title,
        artist: if a.artist.is_empty() { None } else { Some(a.artist) },
        year: a.year.and_then(|y| u16::try_from(y).ok()),
        track_count: a.total_tracks.max(0) as u32,
        has_cover,
    }
}

fn track_view_to_dlna(
    t: crate::mapper::library::track::track_list_item_view::TrackListView,
) -> DlnaTrack {
    let format = t
        .audio_format
        .clone()
        .unwrap_or_else(|| t.extension.clone());
    let mime_type = t
        .mime_type
        .clone()
        .unwrap_or_else(|| guess_mime_from_extension(&t.extension));
    let has_cover = t.thumbnail_path.as_ref().is_some_and(is_local_path);

    DlnaTrack {
        id: t.id,
        title: t.title,
        artist: t.artist,
        album: t.album,
        track_number: t.track_number.and_then(|n| u16::try_from(n).ok()),
        duration_seconds: t.duration.map(|d| d.round() as u32),
        sample_rate: t.sample_rate.and_then(|s| u32::try_from(s).ok()),
        bits_per_sample: t.bits_per_sample.and_then(|b| u32::try_from(b).ok()),
        bitrate: t.bitrate.and_then(|b| u32::try_from(b).ok()),
        channels: t.channels.and_then(|c| u8::try_from(c).ok()),
        file_size: t.file_size.and_then(|s| u64::try_from(s).ok()),
        mime_type,
        format,
        has_cover,
    }
}

/// True if `s` looks like a usable local filesystem path.
/// Filters out empty strings and HTTP(S) URLs that occasionally end up in
/// `cover_url` when the Deezer download fallback couldn't save locally.
fn is_local_path(s: &String) -> bool {
    !s.is_empty()
        && !s.starts_with("http://")
        && !s.starts_with("https://")
}

/// Fallback MIME guess when `library_cache.mime_type` is missing.
fn guess_mime_from_extension(ext: &str) -> String {
    match ext.to_ascii_lowercase().as_str() {
        "mp3" => "audio/mpeg",
        "flac" => "audio/flac",
        "wav" => "audio/wav",
        "aiff" | "aif" => "audio/aiff",
        "ogg" | "opus" => "audio/ogg",
        "m4a" | "aac" => "audio/mp4",
        "ape" => "audio/x-ape",
        "wma" => "audio/x-ms-wma",
        "dsf" | "dff" => "audio/x-dsd",
        _ => "application/octet-stream",
    }
    .to_string()
}
