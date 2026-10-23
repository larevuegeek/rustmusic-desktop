-- Actions à renvoyer à un serveur injoignable au moment où elles ont eu lieu :
-- favoris, notes, écoutes. Vidée dans l'ordre à la synchronisation suivante.

CREATE TABLE server_outbox (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  server_id INTEGER NOT NULL,
  -- star | unstar | rating | scrobble
  action TEXT NOT NULL,
  song_id TEXT NOT NULL,
  -- Note 0 à 5, ou instant de l'écoute (millisecondes).
  value INTEGER NULL,
  created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY (server_id) REFERENCES media_servers(id) ON DELETE CASCADE
);

CREATE INDEX idx_server_outbox_server ON server_outbox(server_id, id);
