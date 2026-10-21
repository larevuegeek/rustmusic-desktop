-- Radios écoutées récemment, par profil (favorites ou non). Une station n'y figure qu'une fois.

CREATE TABLE radio_recent (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  profil_id INTEGER NOT NULL,
  uuid TEXT NULL,
  name TEXT NOT NULL,
  stream_url TEXT NOT NULL,
  homepage TEXT NULL,
  favicon_url TEXT NULL,
  logo_path TEXT NULL,
  country_code TEXT NULL,
  tags TEXT NOT NULL DEFAULT '',
  codec TEXT NULL,
  bitrate INTEGER NULL,
  reduced_quality INTEGER NOT NULL DEFAULT 0,
  played_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY (profil_id) REFERENCES profil(id) ON DELETE CASCADE,
  UNIQUE (profil_id, stream_url)
);

CREATE INDEX idx_radio_recent_profil ON radio_recent(profil_id, played_at);
