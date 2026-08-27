import { invoke } from "@tauri-apps/api/core";
import { selectionStore } from "$lib/stores/ui/selection.store";
import type { TrackListView } from "$lib/types/ui/library/track/TrackListView";

/**
 * Cocher un album, un artiste, un genre ou un dossier.
 *
 * # Pourquoi développer en pistes
 * Le magasin de sélection ne connaît que des pistes, et c'est délibéré : le
 * compteur annonce alors « 47 titres » plutôt que « 3 éléments », et chaque
 * action — mettre en file, éditer les tags, ajouter à une playlist — continue
 * de travailler sur une seule sorte d'objet. Retenir les groupes tels quels
 * aurait obligé chaque action à savoir les développer, et deux d'entre elles
 * auraient fini par le faire différemment.
 *
 * Le coût est une requête au moment du clic. Elle est locale et porte sur un
 * album ou un artiste, pas sur la bibliothèque.
 */

export type GroupKind = "album" | "artist" | "genre" | "folder";

/** Commande et paramètres qui rendent les pistes d'un groupe. */
function requete(kind: GroupKind, libraryId: number, id: string) {
  switch (kind) {
    case "album":
      return ["get_tracks_by_album", { libraryId, libraryAlbumId: id }] as const;
    case "artist":
      return ["get_tracks_by_artist", { libraryId, artistId: id }] as const;
    case "genre":
      return ["get_tracks_by_genre", { libraryId, genre: id }] as const;
    case "folder":
      return ["get_tracks_by_dir", { libraryId, dirId: id }] as const;
  }
}

/**
 * Clé stable d'un groupe dans la sélection.
 *
 * Préfixée par la nature : un album et un artiste peuvent porter le même
 * identifiant sans être le même objet, et deux genres homonymes dans deux
 * bibliothèques non plus.
 */
export function cleGroupe(kind: GroupKind, libraryId: number, id: string): string {
  return `${kind}:${libraryId}:${id}`;
}

/**
 * Coche ou décoche un groupe.
 *
 * Le décochage ne redemande rien : le magasin sait ce que ce groupe avait
 * apporté. Inutile de refaire une requête pour retirer ce qu'on connaît déjà —
 * et le résultat pourrait avoir changé entre-temps, ce qui laisserait des
 * pistes orphelines cochées.
 */
export async function toggleGroupSelection(
  kind: GroupKind,
  libraryId: number,
  id: string,
): Promise<void> {
  const cle = cleGroupe(kind, libraryId, id);

  if (selectionStore.hasGroup(cle)) {
    selectionStore.toggleGroup(cle, []);
    return;
  }

  try {
    const [commande, params] = requete(kind, libraryId, id);
    const tracks = await invoke<TrackListView[]>(commande, params);
    selectionStore.toggleGroup(
      cle,
      (tracks ?? []).map((t) => ({ id: t.id, track: t })),
    );
  } catch (e) {
    console.error(`Sélection ${kind} :`, e);
  }
}
