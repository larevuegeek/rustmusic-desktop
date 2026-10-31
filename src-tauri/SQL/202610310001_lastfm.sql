-- Last.fm : la clé d'API de l'utilisateur, s'il a choisi la sienne (sinon celle de RustMusic).
ALTER TABLE scrobble_accounts ADD COLUMN api_key TEXT NULL;
ALTER TABLE scrobble_accounts ADD COLUMN api_secret TEXT NULL;
