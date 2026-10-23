/** Une bibliothèque distante d'un serveur. */
export type ServerLibrary = { id: number; name: string; remote_folder_id: string | null; total_tracks: number };

/** La sorte de serveur. */
export type ServerKind = "subsonic" | "jellyfin" | "dlna";

/** Un serveur du profil (OpenSubsonic ou Jellyfin) et ses bibliothèques. */
export type ServerEntry = {
  id: number;
  kind: ServerKind;
  name: string;
  url: string;
  username: string;
  enabled: boolean;
  server_type: string | null;
  server_version: string | null;
  open_subsonic: boolean;
  /** Extensions OpenSubsonic, séparées par des virgules. */
  extensions: string;
  transcode_format: string | null;
  transcode_bitrate: number | null;
  /** Ses playlists épinglées dans la barre latérale. */
  pin_playlists: boolean;
  last_sync_at: string | null;
  created_at: string;
  libraries: ServerLibrary[];
};

/** Un dossier musical proposé par le serveur. */
export type MusicFolder = { id: string; name: string };

/** Résultat d'un test de connexion (rien n'est enregistré). */
export type ServerProbe = {
  url: string;
  name: string;
  info: { kind: string | null; version: string | null; open_subsonic: boolean; extensions: string[] };
  folders: MusicFolder[];
};

/** Un serveur DLNA trouvé sur le réseau. */
export type DlnaFound = { id: string; name: string; model: string; location: string };

/** Un dossier d'un serveur DLNA ; `children` : nombre annoncé d'éléments. */
export type DlnaFolder = { id: string; name: string; children: number | null };
