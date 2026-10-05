import { artistesLisibles } from "#lib/helper/tools/stringTools";
import type { RecentFileListView } from "#lib/types/ui/recent/RecentFileListView";

/** Un album de l'historique : les morceaux écoutés, du plus récent au plus ancien. */
export type AlbumEcoute = {
  cle: string;
  album: string;
  artiste: string | null;
  pochette: string | null;
  pistes: RecentFileListView[];
  derniereEcoute: string;
};

const norme = (s: string | null | undefined) => (s ?? "").trim().toLowerCase();

/**
 * Regroupe l'historique par album, dans l'ordre de la dernière écoute. L'artiste
 * de l'album sert de clé : une compilation reste un seul album. Un morceau sans
 * album forme son propre groupe.
 */
export function grouperParAlbum(historique: RecentFileListView[]): AlbumEcoute[] {
  const groupes = new Map<string, AlbumEcoute>();
  for (const r of historique) {
    const artiste = artistesLisibles(r.album_artist) ?? artistesLisibles(r.artist);
    const cle = r.album ? `${norme(r.album)}|${norme(artiste)}` : `piste:${r.path}`;
    const g = groupes.get(cle);
    if (g) {
      g.pistes.push(r);
      g.pochette ??= r.thumbnail_path;
      continue;
    }
    groupes.set(cle, {
      cle,
      album: r.album ?? r.title ?? "",
      artiste,
      pochette: r.thumbnail_path,
      pistes: [r],
      derniereEcoute: r.last_played_at,
    });
  }
  return [...groupes.values()];
}
