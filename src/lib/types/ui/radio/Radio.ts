/** Une station de l'annuaire Radio Browser, ou décrite depuis une adresse collée. */
export type RadioResult = {
  uuid: string | null;
  name: string;
  stream_url: string;
  homepage: string | null;
  favicon: string | null;
  country_code: string | null;
  tags: string[];
  codec: string | null;
  bitrate: number | null;
  /** AAC+ : le son passe, mais étouffé. */
  reduced_quality: boolean;
};

/** Une radio favorite du profil. */
export type RadioView = {
  id: number;
  uuid: string | null;
  name: string;
  stream_url: string;
  homepage: string | null;
  favicon_url: string | null;
  logo_path: string | null;
  country_code: string | null;
  /** Genres séparés par des virgules. */
  tags: string;
  codec: string | null;
  bitrate: number | null;
  reduced_quality: boolean;
  position: number;
  last_played_at: string | null;
  created_at: string;
};

/** Une radio écoutée récemment (favorite ou non). */
export type RadioRecent = Omit<RadioView, "position" | "last_played_at" | "created_at"> & { played_at: string };
