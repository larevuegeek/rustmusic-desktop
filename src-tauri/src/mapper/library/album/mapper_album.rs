use crate::{entity::{artist::artist::Artist, library::library_album::LibraryAlbum}, mapper::library::album::{album_detail_view::AlbumDetailView, album_list_view::AlbumListView}};

pub fn to_album_list_view(
    album: &LibraryAlbum,
    artist: &Artist,
    _thumbnail_path: Option<String>,
) -> AlbumListView {
    AlbumListView {
        // ===== Identité =====
        id: album.id.clone(),
        library_id: album.library_id,

        // ===== Infos Album =====
        title: album.title.clone(),
        title_normalized: album.title_normalized.clone(),
        album_type: album.album_type.clone(),
        musicbrainz_id: album.musicbrainz_id.clone(),

        // ===== Artist =====
        artist_id: Some(artist.id.clone()),
        artist: artist.name.clone(),

        // ===== Métadonnées =====
        year: album.year,
        genre: album.genre.clone(),

        // ===== Cover =====
        cover_url: album.cover_url.clone(),

        // ===== Statistiques =====
        total_tracks: album.total_tracks,
        total_duration: album.total_duration,

        // ===== Notes =====
        notes: album.notes.clone(),

        // Ce mappeur part de l'album, donc de son propriétaire.
        participation: false,

        // ===== Timestamps =====
        created_at: album.created_at,
        updated_at: album.updated_at,
    }
}

pub fn to_album_detail_view(
    album: &LibraryAlbum,
    artist: &Artist,
    thumbnail_path: Option<String>,
) -> AlbumDetailView {

    AlbumDetailView {
        // ===== Identité =====
        id: album.id.clone(),
        library_id: album.library_id,

        // ===== Infos Album =====
        title: album.title.clone(),
        title_normalized: album.title_normalized.clone(),
        album_type: album.album_type.clone(),
        musicbrainz_id: album.musicbrainz_id.clone(),

        // ===== Artist =====
        artist_id: artist.id.clone(),
        artist: artist.name.clone(),

        // ===== Métadonnées =====
        year: album.year,
        genre: album.genre.clone(),
        notes: album.notes.clone(),

        // ===== Cover =====
        cover_url: album.cover_url.clone(),
        thumbnail_path,

        // ===== Statistiques =====
        total_tracks: album.total_tracks,
        total_duration: album.total_duration,

        // ===== Timestamps =====
        created_at: album.created_at,
        updated_at: album.updated_at,
    }
}