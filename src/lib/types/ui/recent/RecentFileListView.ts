export interface RecentFileListView {
  id: number;
  library_cache_id: number | null;
  path: string;
  last_played_at: string; // ISO date string
  last_position: number;
  play_count: number;

  // Champs venant de library_cache (flat)
  cache_id: number | null;
  cache_path: string | null;
  title: string | null;
  artist: string | null;
  album: string | null;
  album_artist: string | null;
  year: string | null;
  genre: string | null;
  track_number: number | null;
  disc_number: number | null;
  duration: number | null;
  bitrate: number | null;
  bits_per_sample: number | null;
  sample_rate: number | null;
  channels: number | null;
  audio_format: string | null;
  mime_type: string | null;
  file_size: number | null;
  extra_tags: string | null;
  thumbnail_path: string | null;
  last_scanned_at: string | null;

  liked: boolean;
}