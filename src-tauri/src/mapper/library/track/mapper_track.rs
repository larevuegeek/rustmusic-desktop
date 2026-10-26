
use crate::entity::artist::artist::Artist;
use crate::entity::library::library_album::LibraryAlbum;
use crate::entity::library::library_artist::LibraryArtist;
use crate::entity::library::library_track::LibraryTrack;
use crate::entity::library::library_files::LibraryFile;
use crate::entity::library::library_cache::LibraryCache;
use crate::mapper::library::track::track_list_item_view::TrackListView;

pub fn to_track_list_view(
    track: &LibraryTrack,
    file: &LibraryFile,
    cache: &LibraryCache,
    artist: &Artist,
    library_artist: Option<&LibraryArtist>,
    album: Option<&LibraryAlbum>,
) -> TrackListView {
    TrackListView {
        // ===== Identité =====
        id: track.id.clone(),

        // ===== Fichier =====
        path: file.path.clone(),
        filename: file.filename.clone(),
        extension: file.extension.clone(),
        size: file.size,
        status: file.status.clone(),
        is_available: file.is_available,
        error_message: file.error_message.clone(),

        // ===== Métadonnées =====
        title: track.title.clone(),
        title_normalized: track.title_normalized.clone(),
        artist_id: Some(artist.id.clone()),
        library_artist_id: library_artist.map(|a| a.id.clone()),
        album_id: album.map(|a|a.id.clone()),
        artist: Some(artist.name.clone()),
        album: album.map(|a|a.title.clone()),
        album_artist: cache.album_artist.clone(),
        year: cache.year.clone(),
        genre: cache.genre.clone(),
        track_number: track.track_number,
        disc_number: track.disc_number,

        // ===== Technique =====
        duration: track.duration.or(cache.duration),
        bitrate: track.bitrate.or(cache.bitrate),
        bits_per_sample: cache.bits_per_sample,
        sample_rate: track.sample_rate.or(cache.sample_rate),
        channels: cache.channels,
        audio_format: cache.audio_format.clone(),
        mime_type: cache.mime_type.clone(),
        file_size: cache.file_size,

        // ===== Cache enrichi =====
        extra_tags: cache.extra_tags.clone(),
        thumbnail_path: cache.thumbnail_path.clone(),
        last_scanned_at: cache.last_scanned_at,
        tags: track.tags.clone(),

        // ===== Stats =====
        play_count: track.play_count,
        last_played_at: track.last_played_at.clone(),
        rating: track.rating,
        favorite: track.favorite,

        // ===== Timestamps =====
        created_at: track.created_at,
        updated_at: track.updated_at,
    }
}

