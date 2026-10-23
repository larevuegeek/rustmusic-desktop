-- Identifiant réseau d'un serveur DLNA (UDN) : il ne change pas quand son adresse IP change.
ALTER TABLE media_servers ADD COLUMN device_id TEXT NULL;
