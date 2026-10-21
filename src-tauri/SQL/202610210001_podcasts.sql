-- Podcasts suivis, par profil, et leurs épisodes.
-- La progression d'écoute vit ici : les épisodes ne passent pas par les récents.

CREATE TABLE podcasts (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  profil_id INTEGER NOT NULL,
  feed_url TEXT NOT NULL,
  title TEXT NOT NULL,
  author TEXT NULL,
  description TEXT NULL,
  image_url TEXT NULL,
  cover_path TEXT NULL,
  link TEXT NULL,
  language TEXT NULL,
  -- Validateurs HTTP : un flux inchangé n'est pas retéléchargé.
  etag TEXT NULL,
  last_modified TEXT NULL,
  checked_at DATETIME NULL,
  auto_download INTEGER NOT NULL DEFAULT 0,
  position INTEGER NOT NULL DEFAULT 0,
  created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY (profil_id) REFERENCES profil(id) ON DELETE CASCADE,
  UNIQUE (profil_id, feed_url)
);

CREATE TABLE podcast_episodes (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  podcast_id INTEGER NOT NULL,
  -- Identifiant stable donné par le flux (à défaut, l'adresse du fichier).
  guid TEXT NOT NULL,
  title TEXT NOT NULL,
  -- HTML du flux, tel quel : nettoyé à l'affichage.
  description TEXT NULL,
  -- Texte brut court, pour les listes.
  summary TEXT NULL,
  published_at DATETIME NULL,
  duration INTEGER NULL,
  audio_url TEXT NOT NULL,
  audio_type TEXT NULL,
  audio_size INTEGER NULL,
  image_url TEXT NULL,
  season INTEGER NULL,
  episode INTEGER NULL,
  -- Fichier téléchargé, s'il l'est.
  file_path TEXT NULL,
  -- Secondes écoutées.
  position REAL NOT NULL DEFAULT 0,
  played INTEGER NOT NULL DEFAULT 0,
  played_at DATETIME NULL,
  -- Paru depuis l'abonnement et pas encore écouté : le badge « nouveaux ».
  is_new INTEGER NOT NULL DEFAULT 0,
  FOREIGN KEY (podcast_id) REFERENCES podcasts(id) ON DELETE CASCADE,
  UNIQUE (podcast_id, guid)
);

CREATE INDEX idx_podcasts_profil ON podcasts(profil_id);
CREATE INDEX idx_podcast_episodes_published ON podcast_episodes(podcast_id, published_at DESC);
