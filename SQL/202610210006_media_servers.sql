-- Serveurs OpenSubsonic (Navidrome…) : chaque dossier musical choisi devient une bibliothèque distante.
-- Le mot de passe n'est pas gardé : seulement l'empreinte (token = md5(mot de passe + sel)) ou une clé d'API.

CREATE TABLE media_servers (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  profil_id INTEGER NOT NULL,
  name TEXT NOT NULL,
  url TEXT NOT NULL,
  username TEXT NOT NULL,
  token TEXT NULL,
  salt TEXT NULL,
  api_key TEXT NULL,
  -- Désactivé : ses bibliothèques disparaissent du sélecteur, sans rien perdre.
  enabled INTEGER NOT NULL DEFAULT 1,
  server_type TEXT NULL,
  server_version TEXT NULL,
  open_subsonic INTEGER NOT NULL DEFAULT 0,
  -- Extensions OpenSubsonic, séparées par des virgules.
  extensions TEXT NOT NULL DEFAULT '',
  -- Transcodage demandé au serveur ; NULL = fichier d'origine.
  transcode_format TEXT NULL,
  transcode_bitrate INTEGER NULL,
  last_sync_at DATETIME NULL,
  created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY (profil_id) REFERENCES profil(id) ON DELETE CASCADE,
  UNIQUE (profil_id, url, username)
);

-- Bibliothèque distante : son serveur et l'identifiant du dossier musical côté serveur.
ALTER TABLE library ADD COLUMN server_id INTEGER NULL REFERENCES media_servers(id) ON DELETE CASCADE;
ALTER TABLE library ADD COLUMN remote_folder_id TEXT NULL;
