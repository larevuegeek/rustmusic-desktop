import { readableArtists } from "#lib/helper/tools/stringTools";
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

const normalize = (s: string | null | undefined) => (s ?? "").trim().toLowerCase();

/**
 * Regroupe l'historique par album, dans l'ordre de la dernière écoute. L'artiste
 * de l'album sert de clé : une compilation reste un seul album. Un morceau sans
 * album forme son propre groupe.
 */
export function groupByAlbum(historique: RecentFileListView[]): AlbumEcoute[] {
  const groupes = new Map<string, AlbumEcoute>();
  for (const r of historique) {
    const artiste = readableArtists(r.album_artist) ?? readableArtists(r.artist);
    const cle = r.album ? `${normalize(r.album)}|${normalize(artiste)}` : `piste:${r.path}`;
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
