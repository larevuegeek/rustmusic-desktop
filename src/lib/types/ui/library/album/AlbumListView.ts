export interface AlbumListView {
  id: string;
  library_id: number;
  title: string;
  title_normalized: string;
  album_type: string;
  musicbrainz_id: string | null;
  artist_id: string | null;
  artist: string | null;
  year: number | null;
  genre: string | null;
  cover_url: string | null;
  total_tracks: number;
  total_duration: number;
  notes: string | null;
  /** L'artiste n'est qu'invité sur cet album. */
  participation: boolean;
  created_at: string;
  updated_at: string; 
}
