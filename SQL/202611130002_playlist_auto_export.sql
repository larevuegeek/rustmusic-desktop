-- Export automatique d'une playlist en .m3u8 / .m3u (clé USB, téléphone).
ALTER TABLE playlists ADD COLUMN auto_export INTEGER NOT NULL DEFAULT 0;
-- Dernier fichier écrit : à retirer si la playlist est renommée, supprimée ou n'est plus exportée.
ALTER TABLE playlists ADD COLUMN export_file TEXT NULL;
