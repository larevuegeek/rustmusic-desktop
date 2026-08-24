export interface LibraryTrack {
  id: string;
  library_id: number;
  file_id: string;
  cache_id: number | null;

  artist_id: string | null;
  library_album_id: string | null;

  title: string;
  title_normalized: string;

  track_number: number | null;
  disc_number: number;

  tags: string | null; // JSON string

  duration: number | null;
  bitrate: number | null;
  sample_rate: number | null;

  play_count: number;
  last_played_at: string | null;

  /** Étoiles, 0,5 à 5,0 par demi (3.5 = trois et demie). `null` = jamais noté. */
  rating: number | null;
  favorite: boolean;

  created_at: string; // ISO
  updated_at: string; // ISO
}
