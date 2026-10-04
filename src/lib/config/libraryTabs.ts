/** Les sections de navigation, déclarées une fois pour la barre latérale et la barre du haut. */

export type LibraryTabKey = "tracks" | "albums" | "artists" | "genres" | "years" | "folders" | "mixes" | "playlists";

export type LibraryTab = {
  key: LibraryTabKey;
  labelKey: string;
  icon: string;
};

/** `nav.folders` et non `nav.explorer` : c'est le nom de la route et du fil d'ariane. */
export const ONGLETS_BIBLIOTHEQUE: LibraryTab[] = [
  { key: "tracks",    labelKey: "library.tracks",  icon: "material-symbols:music-note-rounded" },
  { key: "artists",   labelKey: "library.artists", icon: "material-symbols:mic-external-on-outline-rounded" },
  { key: "albums",    labelKey: "library.albums",  icon: "material-symbols:album-outline-rounded" },
  { key: "genres",    labelKey: "library.genres",  icon: "material-symbols:sell-outline-rounded" },
  { key: "years",     labelKey: "library.years",   icon: "material-symbols:calendar-month-outline-rounded" },
  { key: "folders",   labelKey: "nav.folders",     icon: "material-symbols:folder-outline-rounded" },
  { key: "mixes",     labelKey: "library.mixes",   icon: "material-symbols:shuffle-rounded" },
  // Hors bibliothèque (les playlists sont au profil) : voir `lienOnglet`.
  { key: "playlists", labelKey: "nav.playlists",   icon: "material-symbols:queue-music-rounded" },
];

/** L'ordre de toutes les sections ; une section cochée reprend sa place ici. */
export const ORDRE_ONGLETS: LibraryTabKey[] = ONGLETS_BIBLIOTHEQUE.map((o) => o.key);

/** Années, Mix, Playlists décochées d'office : une 7e tuile ajouterait une ligne. */
export const ONGLETS_PAR_DEFAUT: LibraryTabKey[] = ORDRE_ONGLETS.filter((c) => c !== "years" && c !== "mixes" && c !== "playlists");

/** L'adresse d'une section : celles de la bibliothèque, et Playlists qui n'en dépend pas. */
export function lienOnglet(libraryId: number, cle: LibraryTabKey): string {
  return cle === "playlists" ? "/playlists" : `/library/${libraryId}/${cle}`;
}

const PAR_CLE = new Map(ONGLETS_BIBLIOTHEQUE.map((o) => [o.key, o]));

/** Où les sections sont proposées. */
export type PlacementOnglets = "sidebar" | "top" | "both";

export const PLACEMENT_PAR_DEFAUT: PlacementOnglets = "both";

export function lirePlacement(brut: string | undefined): PlacementOnglets {
  return brut === "sidebar" || brut === "top" || brut === "both"
    ? brut
    : PLACEMENT_PAR_DEFAUT;
}

/** Lit les sections retenues ; un réglage abîmé (clés inconnues, doublons, vide) retombe sur le défaut. */
export function lireOnglets(brut: string | undefined): LibraryTabKey[] {
  if (!brut) return [...ONGLETS_PAR_DEFAUT];
  try {
    const v = JSON.parse(brut);
    if (!Array.isArray(v)) return [...ONGLETS_PAR_DEFAUT];
    const sortie: LibraryTabKey[] = [];
    for (const c of v) {
      if (typeof c !== "string") continue;
      const cle = c as LibraryTabKey;
      if (!PAR_CLE.has(cle) || sortie.includes(cle)) continue;
      sortie.push(cle);
    }
    // Toujours dans l'ordre de référence : un nouvel ordre s'applique aussi aux réglages enregistrés.
    sortie.sort((a, b) => ORDRE_ONGLETS.indexOf(a) - ORDRE_ONGLETS.indexOf(b));
    return sortie.length > 0 ? sortie : [...ONGLETS_PAR_DEFAUT];
  } catch {
    return [...ONGLETS_PAR_DEFAUT];
  }
}

/**
 * Les sections à afficher. `courant` est ajouté même masqué : on arrive sur un
 * genre depuis un album, et une barre qui n'indique pas où l'on est ment.
 */
export function resoudreOnglets(
  cles: LibraryTabKey[],
  courant?: string | null,
): LibraryTab[] {
  const sortie = cles.map((c) => PAR_CLE.get(c)).filter((o): o is LibraryTab => Boolean(o));

  if (courant && PAR_CLE.has(courant as LibraryTabKey) && !cles.includes(courant as LibraryTabKey)) {
    // À sa place d'origine : le voir sauter en le cochant désorienterait.
    const rang = ORDRE_ONGLETS.indexOf(courant as LibraryTabKey);
    const avant = sortie.filter((o) => ORDRE_ONGLETS.indexOf(o.key) < rang).length;
    sortie.splice(avant, 0, PAR_CLE.get(courant as LibraryTabKey)!);
  }

  return sortie;
}

/** La section ouverte, d'après l'URL. `null` hors d'une bibliothèque. */
export function ongletCourant(pathname: string): LibraryTabKey | null {
  if (pathname === "/playlists" || pathname.startsWith("/playlist/")) return "playlists";
  const segments = pathname.split("/").filter(Boolean);
  if (segments[0] !== "library") return null;
  const cle = segments[2];
  if (!cle) return "tracks"; // `/library/42` montre les morceaux
  return PAR_CLE.has(cle as LibraryTabKey) ? (cle as LibraryTabKey) : null;
}

/** Retenu pour rouvrir la bibliothèque sur la dernière section (Playlists n'en est pas une). */
export function memoriserOnglet(libraryId: number, cle: LibraryTabKey) {
  if (cle === "playlists") return;
  try {
    localStorage.setItem(`lib-tab-${libraryId}`, cle);
  } catch {
    // Stockage bloqué : on perd la mémoire, pas la navigation.
  }
}

export function dernierOnglet(libraryId: number): LibraryTabKey {
  try {
    const cle = localStorage.getItem(`lib-tab-${libraryId}`);
    if (cle && cle !== "playlists" && PAR_CLE.has(cle as LibraryTabKey)) return cle as LibraryTabKey;
  } catch {
    // Voir `memoriserOnglet`.
  }
  return "tracks";
}
