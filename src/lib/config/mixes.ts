import { teinte } from "#lib/helper/tools/teinte";
import { estDatee, decennieDe, courtDecennie, teinteDecennie } from "#lib/helper/library/periode";
import type { SourceMix } from "#lib/stores/mix/mix.store";
import type { GenreMix } from "#lib/types/ui/library/genre/GenreMix";
import type { AlbumListView } from "#lib/types/ui/library/album/AlbumListView";
import type { Playlist } from "#lib/types/db/playlist/Playlist";

/** Un mix à afficher : ce qu'on tire, et comment on le présente. */
export type SpecMix = {
  cle: string;
  type: string;
  nom: string;
  aide: string;
  /** Teinte de la carte quand il n'y a pas de pochette. */
  h: number;
  source: SourceMix;
  /** Mix de décennie : « Années » et « 80 ». */
  pre?: string;
  court?: string;
  /** Mix créé : la playlist auto qui le porte. */
  playlist?: Playlist;
};

type T = (cle: string) => string;

// En dessous, un genre ou une décennie ne fait pas un mix.
export const MIN_TITRES_MIX = 10;

const specGenre = (g: GenreMix, t: T, type: string): SpecMix => ({
  cle: `genre:${g.nom.toLowerCase()}`, type, nom: g.nom,
  aide: t("mix_view.genre_desc").replace("{n}", String(g.artistes)), h: teinte(g.nom.toLowerCase()),
  source: { kind: "genre", genre: g.nom },
});

/** « Pour vous » : deux genres du jour (ils tournent parmi les plus fournis), les oubliés, le Hi-Res. */
export function mixPourVous(genres: GenreMix[], hires: number, t: T): SpecMix[] {
  if (genres.length === 0) return [];
  const tournants = genres.slice(0, 6);
  const jour = Math.floor(Date.now() / 86_400_000);
  const choisis = tournants.length <= 2 ? tournants : [tournants[jour % tournants.length], tournants[(jour + 1) % tournants.length]];
  const liste: SpecMix[] = choisis.map((g) => specGenre(g, t, t("home.mix_of_day")));
  // Libellés courts sur la pochette, l'explication entière en info-bulle.
  liste.push({ cle: "oublies", type: t("home.rediscover"), nom: t("home.forgotten"), aide: t("home.forgotten_kind"), h: 200, source: { kind: "oublies" } });
  if (hires > 0) liste.push({ cle: "hires", type: t("home.studio_quality"), nom: "Hi-Res", aide: t("home.hires_kind"), h: 150, source: { kind: "hires" } });
  return liste;
}

/** Un mix par genre assez varié (le serveur a déjà écarté les autres), les plus fournis d'abord. */
export function mixGenres(genres: GenreMix[], t: T): SpecMix[] {
  return genres.map((g) => specGenre(g, t, t("mix_view.genre_kind")));
}

/** Une décennie par tranche d'albums datés : « 80 » avant 2000, « 2010 » après. */
export function mixDecennies(albums: AlbumListView[], t: T): SpecMix[] {
  const pistesPar = new Map<number, number>();
  for (const a of albums) {
    if (!estDatee(a.year)) continue;
    const d = decennieDe(a.year);
    pistesPar.set(d, (pistesPar.get(d) ?? 0) + (a.total_tracks ?? 0));
  }
  return [...pistesPar.entries()]
    .filter(([, n]) => n >= MIN_TITRES_MIX)
    .sort(([a], [b]) => a - b)
    .map(([d]) => {
      const court = courtDecennie(d);
      return {
        cle: `decennie:${d}`, type: t("home.decade_pre"), nom: t("home.decade_name").replace("{d}", court),
        aide: t("home.decade_kind").replace("{a}", String(d)).replace("{b}", String(d + 9)),
        h: teinteDecennie(d), source: { kind: "decennie", decennie: d },
        pre: t("home.decade_pre"), court: t("home.decade_short").replace("{d}", court),
      };
    });
}

/** Les mix créés : des playlists auto marquées « mix ». */
export function mixCrees(playlists: Playlist[], t: T): SpecMix[] {
  return playlists
    .filter((p) => p.is_mix)
    .map((p) => ({
      cle: `perso:${p.id}`, type: t("mix_view.custom_kind"), nom: p.name, aide: p.name, h: teinte(p.name),
      source: { playlistId: p.id }, playlist: p,
    }));
}
