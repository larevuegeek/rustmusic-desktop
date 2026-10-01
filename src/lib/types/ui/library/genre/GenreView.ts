export type GenreView = {
  name: string;
  total_albums: number;
  total_tracks: number;
  covers: string[];
  /** Les trois artistes les plus présents (absents des pseudo-genres). */
  top_artists?: string[];
  last_played_at?: string | null;
};
