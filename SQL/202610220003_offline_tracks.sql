-- Morceaux d'un serveur gardés hors ligne : téléchargés en entier, jouables serveur éteint.

CREATE TABLE offline_tracks (
  -- Chemin `subsonic://…` du morceau.
  path TEXT PRIMARY KEY,
  server_id INTEGER NOT NULL,
  -- Fichier téléchargé ; NULL tant qu'il est en attente.
  file TEXT NULL,
  size INTEGER NULL,
  added_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY (server_id) REFERENCES media_servers(id) ON DELETE CASCADE
);

CREATE INDEX idx_offline_tracks_server ON offline_tracks(server_id);
