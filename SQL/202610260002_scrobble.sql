-- Comptes de suivi d'écoute (ListenBrainz, puis Last.fm), un par profil et par service.
CREATE TABLE scrobble_accounts (
  profil_id INTEGER NOT NULL,
  service TEXT NOT NULL,
  token TEXT NOT NULL,
  user_name TEXT NOT NULL,
  -- Annoncer aussi le morceau en cours.
  now_playing INTEGER NOT NULL DEFAULT 1,
  created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (profil_id, service),
  FOREIGN KEY (profil_id) REFERENCES profil(id) ON DELETE CASCADE
);

-- Écoutes pas encore envoyées (hors connexion, service en panne).
CREATE TABLE scrobble_queue (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  profil_id INTEGER NOT NULL,
  service TEXT NOT NULL,
  listened_at INTEGER NOT NULL,
  artist TEXT NOT NULL,
  title TEXT NOT NULL,
  album TEXT NULL,
  duration_ms INTEGER NULL,
  FOREIGN KEY (profil_id) REFERENCES profil(id) ON DELETE CASCADE
);
CREATE INDEX idx_scrobble_queue ON scrobble_queue (profil_id, service, listened_at);
