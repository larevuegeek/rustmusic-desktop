/** Un podcast suivi, avec ses compteurs (voir `PodcastView` côté Rust). */
export type PodcastView = {
  id: number;
  feed_url: string;
  title: string;
  author: string | null;
  /** HTML du flux : à passer par `safeHtml` avant affichage. */
  description: string | null;
  image_url: string | null;
  cover_path: string | null;
  link: string | null;
  checked_at: string | null;
  auto_download: boolean;
  /** Favori : épinglé dans la barre latérale. */
  pinned: boolean;
  episode_count: number;
  unplayed_count: number;
  /** Parus depuis l'abonnement et pas encore écoutés. */
  new_count: number;
  latest_at: string | null;
  /** Date d'abonnement. */
  created_at: string;
};

export type EpisodeView = {
  id: number;
  podcast_id: number;
  title: string;
  summary: string | null;
  /** Rempli seulement par `get_podcast_episode`. */
  description: string | null;
  /** UTC, « AAAA-MM-JJ HH:MM:SS ». */
  published_at: string | null;
  duration: number | null;
  audio_url: string;
  audio_type: string | null;
  audio_size: number | null;
  image_url: string | null;
  season: number | null;
  episode: number | null;
  file_path: string | null;
  /** Secondes écoutées. */
  position: number;
  played: boolean;
  is_new: boolean;
  /** Chemin de l'épisode dans le cache (téléchargé ou non) : celui que lit le lecteur. */
  path: string;
};

/** Aperçu d'un flux avant l'abonnement. */
export type PodcastPreview = {
  title: string;
  author: string | null;
  description: string | null;
  image_url: string | null;
  link: string | null;
  episode_count: number;
  episodes: { title: string; summary: string | null; published_at: string | null; duration: number | null }[];
};

/** Résultat d'un annuaire. */
export type DirectoryResult = {
  title: string;
  author: string | null;
  feed_url: string;
  image_url: string | null;
  genre: string | null;
  episode_count: number | null;
  /** Date du dernier épisode (ISO 8601). */
  released_at: string | null;
  source: string;
};

/** Un épisode téléchargé (page Téléchargements). */
export type DownloadView = {
  id: number;
  podcast_id: number;
  podcast_title: string;
  cover_path: string | null;
  title: string;
  published_at: string | null;
  duration: number | null;
  file_path: string;
  position: number;
  played: boolean;
  /** Octets sur le disque. */
  size: number;
};

/** Un épisode à écouter, avec son émission (accueil des podcasts). */
export type LatestEpisode = EpisodeView & { podcast_title: string; cover_path: string | null; new_count: number };
