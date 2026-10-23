-- Sources : « Cet ordinateur » (0) ou un serveur (son id). J'aime, récents et playlists
-- portent leur source, indexée : filtrer par source ne lit jamais les chemins.

ALTER TABLE track_liked ADD COLUMN source_id INTEGER NOT NULL DEFAULT 0;
ALTER TABLE recent_files ADD COLUMN source_id INTEGER NOT NULL DEFAULT 0;
ALTER TABLE playlists ADD COLUMN source_id INTEGER NOT NULL DEFAULT 0;

-- Source d'un chemin distant : `remote://<serveur>/…` ; les autres sont locaux.
UPDATE track_liked SET source_id = CAST(substr(path, 10, instr(substr(path, 10), '/') - 1) AS INTEGER)
WHERE path LIKE 'remote://%';
UPDATE recent_files SET source_id = CAST(substr(path, 10, instr(substr(path, 10), '/') - 1) AS INTEGER)
WHERE path LIKE 'remote://%';

-- Playlist : celle de son serveur, sinon celle de son premier morceau.
UPDATE playlists SET source_id = COALESCE(
    server_id,
    (SELECT l.server_id FROM playlist_items pi
       JOIN library_tracks lt ON lt.id = pi.library_track_id
       JOIN library l ON l.id = lt.library_id
      WHERE pi.playlist_id = playlists.id
      ORDER BY pi.sort_index LIMIT 1),
    0);

CREATE INDEX IF NOT EXISTS idx_track_liked_source ON track_liked (profil_id, source_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_recent_files_source ON recent_files (source_id, last_played_at DESC);
CREATE INDEX IF NOT EXISTS idx_playlists_source ON playlists (profil_id, source_id);
