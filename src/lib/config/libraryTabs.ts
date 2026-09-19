/**
 * Les sections d'une bibliothèque, déclarées une seule fois.
 *
 * La barre latérale et la barre du haut les lisent toutes les deux ; elles les
 * déclaraient chacune de leur côté et avaient fini par diverger.
 */

export type LibraryTabKey = "tracks" | "albums" | "artists" | "genres" | "folders";

export type LibraryTab = {
  key: LibraryTabKey;
  labelKey: string;
  icon: string;
};

/** `nav.folders` et non `nav.explorer` : c'est le nom de la route et du fil d'ariane. */
export const ONGLETS_BIBLIOTHEQUE: LibraryTab[] = [
  { key: "tracks",  labelKey: "library.tracks",  icon: "mynaui:music" },
  { key: "albums",  labelKey: "library.albums",  icon: "lucide:disc-album" },
  { key: "artists", labelKey: "library.artists", icon: "lucide:mic-2" },
  { key: "genres",  labelKey: "library.genres",  icon: "lucide:tag" },
  { key: "folders", labelKey: "nav.folders",     icon: "lucide:folder-open" },
];

export const ONGLETS_PAR_DEFAUT: LibraryTabKey[] = ONGLETS_BIBLIOTHEQUE.map((o) => o.key);

const PAR_CLE = new Map(ONGLETS_BIBLIOTHEQUE.map((o) => [o.key, o]));

/** Où les sections sont proposées. */
export type PlacementOnglets = "sidebar" | "top" | "both";

export const PLACEMENT_PAR_DEFAUT: PlacementOnglets = "both";

export function lirePlacement(brut: string | undefined): PlacementOnglets {
  return brut === "sidebar" || brut === "top" || brut === "both"
    ? brut
    : PLACEMENT_PAR_DEFAUT;
}

/**
 * Lit les sections retenues. Réglage modifiable et exporté, donc il peut
 * arriver abîmé : clés inconnues, doublons et liste vide retombent sur tout.
 */
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
    const rang = ONGLETS_PAR_DEFAUT.indexOf(courant as LibraryTabKey);
    const avant = sortie.filter((o) => ONGLETS_PAR_DEFAUT.indexOf(o.key) < rang).length;
    sortie.splice(avant, 0, PAR_CLE.get(courant as LibraryTabKey)!);
  }

  return sortie;
}

/** La section ouverte, d'après l'URL. `null` hors d'une bibliothèque. */
export function ongletCourant(pathname: string): LibraryTabKey | null {
  const segments = pathname.split("/").filter(Boolean);
  if (segments[0] !== "library") return null;
  const cle = segments[2];
  if (!cle) return "tracks"; // `/library/42` montre les morceaux
  return PAR_CLE.has(cle as LibraryTabKey) ? (cle as LibraryTabKey) : null;
}

/** Les deux barres passent par ici ; seule celle du haut le faisait. */
export function memoriserOnglet(libraryId: number, cle: LibraryTabKey) {
  try {
    localStorage.setItem(`lib-tab-${libraryId}`, cle);
  } catch {
    // Stockage bloqué : on perd la mémoire, pas la navigation.
  }
}

export function dernierOnglet(libraryId: number): LibraryTabKey {
  try {
    const cle = localStorage.getItem(`lib-tab-${libraryId}`);
    if (cle && PAR_CLE.has(cle as LibraryTabKey)) return cle as LibraryTabKey;
  } catch {
    // Voir `memoriserOnglet`.
  }
  return "tracks";
}
