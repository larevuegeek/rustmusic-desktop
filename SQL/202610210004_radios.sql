-- Radios favorites, par profil. Le flux est l'identité : une même station n'est gardée qu'une fois.

CREATE TABLE radio_stations (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  profil_id INTEGER NOT NULL,
  -- Identifiant Radio Browser ; NULL pour une adresse collée.
  uuid TEXT NULL,
  name TEXT NOT NULL,
  stream_url TEXT NOT NULL,
  homepage TEXT NULL,
  favicon_url TEXT NULL,
  logo_path TEXT NULL,
  country_code TEXT NULL,
  -- Genres séparés par des virgules.
  tags TEXT NOT NULL DEFAULT '',
  codec TEXT NULL,
  bitrate INTEGER NULL,
  reduced_quality INTEGER NOT NULL DEFAULT 0,
  position INTEGER NOT NULL DEFAULT 0,
  last_played_at DATETIME NULL,
  created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY (profil_id) REFERENCES profil(id) ON DELETE CASCADE,
  UNIQUE (profil_id, stream_url)
);

CREATE INDEX idx_radio_stations_profil ON radio_stations(profil_id, position);
