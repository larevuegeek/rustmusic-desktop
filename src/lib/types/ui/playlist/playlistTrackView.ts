export interface PlaylistTrackView {
  playlist_item_id: number;
  playlist_id: number;
  sort_index: number;
  library_track_id: string;
  title: string | null;
  duration: number | null;
  /** Nombre d'écoutes. Zéro tant que le morceau n'a pas franchi le seuil. */
  play_count: number;
  track_number: number | null;
  disc_number: number | null;
  album_id: string | null;
  album_title: string | null;
  artist_id: string | null;
  artist_name: string | null;
  path: string | null;
  thumbnail_path: string | null;
}
