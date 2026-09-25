import { invoke } from "@tauri-apps/api/core";

/** Où mène un morceau : sa fiche, son artiste, son album. */
export type TrackLocation = {
  path: string;
  library_id: number;
  library_track_id: string;
  artist_id: string | null;
  library_album_id: string | null;
};

// Un chemin hors bibliothèque vaut `null`, et on le retient aussi : sans ça on
// le redemanderait à chaque affichage.
const connus = new Map<string, TrackLocation | null>();

export async function localiser(paths: string[]): Promise<Map<string, TrackLocation | null>> {
  const manquants = [...new Set(paths.filter((p) => p && !connus.has(p)))];

  if (manquants.length > 0) {
    try {
      const trouves = await invoke<TrackLocation[]>('get_track_locations', { paths: manquants });
      for (const lieu of trouves) connus.set(lieu.path, lieu);
      for (const p of manquants) if (!connus.has(p)) connus.set(p, null);
    } catch (e) {
      console.error("Localisation des morceaux :", e);
    }
  }

  const reponse = new Map<string, TrackLocation | null>();
  for (const p of paths) reponse.set(p, connus.get(p) ?? null);
  return reponse;
}

export async function localiserUn(path: string): Promise<TrackLocation | null> {
  if (!path) return null;
  return (await localiser([path])).get(path) ?? null;
}

/** Après un scan ou une réparation : les identifiants d'hier ont pu changer. */
export function oublierLocalisations() {
  connus.clear();
}

/** L'adresse de la fiche du morceau, ou `null` s'il n'est pas en bibliothèque. */
export function lienTitre(lieu: TrackLocation | null): string | null {
  return lieu ? `/library/${lieu.library_id}/tracks/${lieu.library_track_id}` : null;
}

export function lienArtiste(lieu: TrackLocation | null): string | null {
  return lieu?.artist_id ? `/library/${lieu.library_id}/artists/${lieu.artist_id}` : null;
}

export function lienAlbum(lieu: TrackLocation | null): string | null {
  return lieu?.library_album_id ? `/library/${lieu.library_id}/albums/${lieu.library_album_id}` : null;
}
