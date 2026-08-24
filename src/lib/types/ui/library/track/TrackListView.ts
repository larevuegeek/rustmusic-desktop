export interface TrackListView {
  id: string;
  
  path: string;
  filename: string;
  extension: string;
  size: number;
  status: string;
  is_available: boolean;
  error_message: string | null;

  title: string;
  title_normalized: string;
  artist_id: string | null;
  library_artist_id: string | null;
  album_id: string | null;
  artist: string | null;
  album: string | null;
  album_artist: string | null;
  year: string | null;
  genre: string | null;
  track_number: number | null;
  disc_number: number;

  duration: number | null;
  bitrate: number | null;
  bits_per_sample: number | null;
  sample_rate: number | null;
  channels: number | null;
  audio_format: string | null;
  mime_type: string | null;
  file_size: number | null;

  extra_tags: string | null;
  /** Jeu complet des tags du fichier, en JSON. Source des colonnes de tags. */
  tags: string | null;
  thumbnail_path: string | null;
  last_scanned_at: string | null;

  play_count: number;
  last_played_at: string | null;
  /** Étoiles, 0,5 à 5,0 par demi (3.5 = trois et demie). `null` = jamais noté. */
  rating: number | null;
  favorite: boolean;

  created_at: string;
  updated_at: string;
}
