-- Playlists d'un serveur OpenSubsonic : synchronisées dans les deux sens.

ALTER TABLE playlists ADD COLUMN server_id INTEGER NULL REFERENCES media_servers(id) ON DELETE CASCADE;
ALTER TABLE playlists ADD COLUMN remote_id TEXT NULL;
-- Morceaux de la playlist côté serveur absents de RustMusic (dossier non ajouté) : renvoyés tels quels.
ALTER TABLE playlists ADD COLUMN remote_extra TEXT NULL;

CREATE UNIQUE INDEX idx_playlists_remote ON playlists(server_id, remote_id);
