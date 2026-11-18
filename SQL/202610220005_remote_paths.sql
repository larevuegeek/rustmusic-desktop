-- Serveurs de plusieurs sortes (OpenSubsonic, Jellyfin) : chemins de morceaux neutres, `remote://`.

ALTER TABLE media_servers ADD COLUMN kind TEXT NOT NULL DEFAULT 'subsonic';
-- Jellyfin : l'utilisateur côté serveur (les requêtes de bibliothèque passent par lui).
ALTER TABLE media_servers ADD COLUMN user_id TEXT NULL;

UPDATE library_files SET path = 'remote://' || substr(path, 12) WHERE path LIKE 'subsonic://%';
UPDATE library_cache SET path = 'remote://' || substr(path, 12) WHERE path LIKE 'subsonic://%';
UPDATE track_liked SET path = 'remote://' || substr(path, 12) WHERE path LIKE 'subsonic://%';
UPDATE queue_tracks SET path = 'remote://' || substr(path, 12) WHERE path LIKE 'subsonic://%';
UPDATE recent_files SET path = 'remote://' || substr(path, 12) WHERE path LIKE 'subsonic://%';
UPDATE offline_tracks SET path = 'remote://' || substr(path, 12) WHERE path LIKE 'subsonic://%';
UPDATE settings SET value = replace(value, 'subsonic://', 'remote://') WHERE value LIKE '%subsonic://%';
