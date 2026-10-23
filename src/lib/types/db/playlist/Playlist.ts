import { remoteServerId } from "#lib/types/db/library/Library";

/** Une playlist n'accepte que les morceaux de sa source (0 : cet ordinateur). */
export const playlistAccepts = (pl: Pick<Playlist, "source_id">, path: string | null | undefined) =>
  (pl.source_id ?? 0) === (remoteServerId(path) ?? 0);

/** Même règle pour une collection (album, artiste…) d'une bibliothèque, distante ou non. */
export const playlistAcceptsLibrary = (pl: Pick<Playlist, "source_id">, libraryServerId: number | null | undefined) =>
  (pl.source_id ?? 0) === (libraryServerId ?? 0);

export interface Playlist {
  id: number;
  profil_id: number;
  library_id: number | null;
  name: string;
  description: string | null;
  color: string;
  icon: string;
  cover: string | null;
  track_count: number;
  duration: number;
  position: number;
  /** Vrai si le contenu se calcule à partir de règles au lieu d'être rangé. */
  is_smart: boolean;
  /** Un mix : playlist auto tirée au hasard chaque jour, rangée sur la page Mix. */
  is_mix: boolean;
  /** Affichée dans la barre latérale. */
  pinned: boolean;
  /** Playlist d'un serveur, synchronisée avec lui ; sinon locale. */
  server_id?: number | null;
  /** Source à laquelle elle appartient : 0 = cet ordinateur, sinon l'id du serveur. */
  source_id?: number;
  created_at: string;
  updated_at: string | null;
}