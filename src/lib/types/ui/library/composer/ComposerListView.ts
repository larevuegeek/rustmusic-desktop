export type ComposerListView = {
  name: string;
  total_albums: number;
  total_tracks: number;
  total_duration: number;
  thumbnail_path?: string | null;
  /** Le même nom parmi les artistes de la bibliothèque. */
  artist_id?: string | null;
};

export const composerHref = (libraryId: number, name: string) => `/library/${libraryId}/composers/${encodeURIComponent(name)}`;
