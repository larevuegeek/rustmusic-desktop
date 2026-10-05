
import type { LibraryCache } from "#lib/types/db/library/LibraryCache";
import type { RecentFile } from "#lib/types/db/recent/RecentFile";
import type { RecentFileListView } from "#lib/types/ui/recent/RecentFileListView";

export function mapRecentFile(view: RecentFileListView): RecentFile {
  let library: LibraryCache | null = null;

  if (view.cache_id) {
    library = {
      id: view.cache_id,
      path: view.cache_path ?? view.path,
      title: view.title,
      artist: view.artist,
      album: view.album,
      album_artist: view.album_artist,
      year: view.year,
      genre: view.genre,
      track_number: view.track_number,
      disc_number: view.disc_number,
      duration: view.duration,
      bitrate: view.bitrate,
      bits_per_sample: view.bits_per_sample,
      sample_rate: view.sample_rate,
      channels: view.channels,
      audio_format: view.audio_format,
      mime_type: view.mime_type,
      file_size: view.file_size,
      extra_tags: view.extra_tags,
      thumbnail_path: view.thumbnail_path,
      last_scanned_at: view.last_scanned_at
    };
  }

  return {
    id: view.id,
    library_id: view.library_id ?? 0,
    path: view.path,
    last_played_at: view.last_played_at,
    last_position: view.last_position,
    play_count: view.play_count,
    library
  };
}