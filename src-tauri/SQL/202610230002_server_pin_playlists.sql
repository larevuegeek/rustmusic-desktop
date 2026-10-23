-- Playlists d'un serveur épinglées dans la barre latérale ? Non par défaut : certains serveurs
-- (Navidrome) importent un `.m3u` par album, des centaines de playlists.
ALTER TABLE media_servers ADD COLUMN pin_playlists INTEGER NOT NULL DEFAULT 0;

UPDATE playlists SET pinned = 0 WHERE server_id IS NOT NULL;
