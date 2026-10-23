export interface Library {
  id: number | null;
  profil_id: number | null;
  name: string;
  description: string | null;
  cover: string | null;
  position: number;
  total_tracks: number,
  total_albums: number,
  total_artists: number,
  /** Bibliothèque ouverte au démarrage. Au plus une par profil. */
  is_default: boolean;
  created_at: string;
  updated_at: string | null;
  /** Bibliothèque distante : son serveur (sinon locale). */
  server_id?: number | null;
  server_name?: string | null;
}

/** Icône d'une bibliothèque : disque dur si locale, nuage si distante. */
export const libraryIcon = (l: Pick<Library, "server_id"> | null | undefined) =>
  l?.server_id ? "material-symbols:cloud-outline-rounded" : "material-symbols:hard-drive-outline";

/** Préfixe des morceaux d'un serveur : `remote://<serveur>/<album>/<morceau>`. */
export const REMOTE_SCHEME = "remote://";

export const isRemotePath = (path: string | null | undefined): boolean => !!path?.startsWith(REMOTE_SCHEME);

/** Serveur d'un morceau distant, sinon `null`. */
export const remoteServerId = (path: string | null | undefined): number | null =>
  isRemotePath(path) ? Number(path!.split("/")[2]) || null : null;

export interface LibraryCreate {
  profil_id: number | null;
  name: string;
  description: string | null;
}