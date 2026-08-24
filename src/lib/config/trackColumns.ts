import type { TrackListView } from "$lib/types/ui/library/track/TrackListView";
import { formatBitrate, formatChannels } from "$lib/helper/tools/audioFormatTools";

/**
 * Les colonnes de la vue liste, définies une seule fois.
 *
 * L'en-tête et les lignes lisaient chacun leur propre suite de `<div>` avec
 * leurs largeurs recopiées à la main. Deux descriptions de la même chose
 * finissent toujours par diverger — une largeur changée d'un côté, un titre
 * décalé de l'autre. Ici la description est unique : l'en-tête, la ligne et le
 * sélecteur la lisent tous.
 */

export type TrackColumn = {
  /** Identifiant stable, celui qu'on enregistre dans les réglages. */
  key: string;
  /** Intitulé affiché dans l'en-tête et dans le sélecteur. */
  label: string;
  /** Largeur, en classe Tailwind. `flex-1` est réservé au titre. */
  width: string;
  /** Aligné à droite pour les valeurs qu'on compare du regard : durées, tailles. */
  align?: "left" | "right";
  /** Chiffres à chasse fixe, pour que les colonnes de nombres s'alignent. */
  numeric?: boolean;
  /**
   * Rendue par un composant plutôt que par du texte. La notation est
   * interactive : elle ne peut pas être une simple chaîne.
   */
  widget?: "rating";
  /** Texte de la cellule. `null` laisse la cellule vide. */
  value?: (t: TrackListView) => string | null;
  /**
   * Clé de tri, quand le texte affiché ne se range pas correctement.
   *
   * Une durée s'écrit `3:09` et `12:05` : rangées comme du texte, la plus
   * longue passe avant la plus courte. Une taille s'écrit `9.4 Mo` et
   * `112 Mo`. Chaque fois que la mise en forme masque la grandeur réelle, on
   * trie sur celle-ci et non sur ce qu'on lit.
   *
   * Par défaut, le tri se fait sur `value` en comparaison de texte.
   */
  sortOn?: (t: TrackListView) => string | number | null;
  /** `number` range par grandeur, `text` par ordre alphabétique local. */
  sortKind?: "text" | "number";
};

/** Le sens d'un tri. */
export type SortDir = "asc" | "desc";

/**
 * Compare deux pistes selon une colonne.
 *
 * # Les vides finissent toujours en bas
 * Quel que soit le sens, une piste sans valeur se range après celles qui en
 * ont une. Inverser le sens pour faire remonter une colonne de cases vides
 * n'apprend rien à personne : ce qu'on cherche en triant, ce sont les
 * extrêmes des valeurs présentes.
 */
export function comparateur(
  col: TrackColumn,
  dir: SortDir,
): (a: TrackListView, b: TrackListView) => number {
  const lire = col.sortOn ?? ((t: TrackListView) => col.value?.(t) ?? null);
  const signe = dir === "desc" ? -1 : 1;
  const numerique = col.sortKind === "number";

  return (a, b) => {
    const va = lire(a);
    const vb = lire(b);

    const videA = va === null || va === undefined || va === "";
    const videB = vb === null || vb === undefined || vb === "";
    if (videA && videB) return 0;
    if (videA) return 1;
    if (videB) return -1;

    if (numerique) {
      return signe * (Number(va) - Number(vb));
    }
    // `localeCompare` avec `numeric` range « piste 2 » avant « piste 10 »,
    // ce qu'une comparaison de chaînes brutes ne ferait pas.
    return (
      signe *
      String(va).localeCompare(String(vb), undefined, {
        sensitivity: "base",
        numeric: true,
      })
    );
  };
}

/** Horodatage exploitable pour le tri, ou `null` si la date est illisible. */
function instant(iso: string | null): number | null {
  if (!iso) return null;
  const t = new Date(iso).getTime();
  return Number.isNaN(t) ? null : t;
}

// ────────────────────────────────────────────────────────────────────────────
// Lecture des tags du fichier
// ────────────────────────────────────────────────────────────────────────────
//
// `track.tags` porte le jeu complet des tags, sérialisé au moment du scan.
// L'analyser à chaque cellule serait absurde — une piste peut afficher six
// colonnes de tags — d'où ce cache, tenu par identifiant de piste.
//
// Il ne grandit pas indéfiniment : la vue liste ne détient qu'une page de cent
// pistes à la fois, et le cache est vidé quand la page change de contenu.

type TagBag = {
  champs: Record<string, unknown>;
  libres: Record<string, string>;
};

const cacheTags = new Map<string, TagBag | null>();

/** À appeler quand la liste change : le cache ne doit pas survivre aux données. */
export function resetTagCache(): void {
  cacheTags.clear();
}

function lireTags(track: TrackListView): TagBag | null {
  const connu = cacheTags.get(track.id);
  if (connu !== undefined) return connu;

  let sac: TagBag | null = null;
  if (track.tags) {
    try {
      const brut = JSON.parse(track.tags);
      if (brut && typeof brut === "object") {
        const libres: Record<string, string> = {};
        // `custom_tags` est une liste de paires `[clé, valeur]`, pas un objet :
        // un fichier peut porter deux fois la même clé, ce qu'un objet
        // écraserait. On garde la première rencontrée.
        if (Array.isArray(brut.custom_tags)) {
          for (const paire of brut.custom_tags) {
            if (Array.isArray(paire) && paire.length === 2) {
              const [k, v] = paire;
              if (typeof k === "string" && typeof v === "string" && !(k in libres)) {
                libres[k] = v;
              }
            }
          }
        }
        sac = { champs: brut, libres };
      }
    } catch {
      // Un tag illisible n'est pas une raison de casser la liste : la colonne
      // reste vide pour cette piste, les autres s'affichent.
      sac = null;
    }
  }

  cacheTags.set(track.id, sac);
  return sac;
}

/**
 * Valeur d'un tag pour une piste.
 *
 * `cle` vient du recensement fait côté Rust : soit un champ nommé
 * (`composer`), soit un tag libre préfixé (`custom:Label`).
 */
export function valeurDeTag(track: TrackListView, cle: string): string | null {
  const sac = lireTags(track);
  if (!sac) return null;

  if (cle.startsWith("custom:")) {
    return sac.libres[cle.slice("custom:".length)] ?? null;
  }

  const v = sac.champs[cle];
  if (v === null || v === undefined) return null;
  if (typeof v === "string") return v.trim() || null;
  if (typeof v === "number" || typeof v === "boolean") return String(v);
  return null;
}

// ────────────────────────────────────────────────────────────────────────────
// Mise en forme
// ────────────────────────────────────────────────────────────────────────────

function duree(secondes: number | null): string | null {
  if (!secondes) return null;
  const m = Math.floor(secondes / 60);
  const s = Math.floor(secondes % 60);
  return `${m}:${String(s).padStart(2, "0")}`;
}

function poids(octets: number | null): string | null {
  if (!octets) return null;
  const mo = octets / (1024 * 1024);
  return mo >= 100 ? `${Math.round(mo)} Mo` : `${mo.toFixed(1)} Mo`;
}

function date(iso: string | null): string | null {
  if (!iso) return null;
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? null : d.toLocaleDateString();
}

function frequence(hz: number | null): string | null {
  if (!hz) return null;
  // Trois décimales pour distinguer 44,1 de 48, sans traîner de zéros.
  return `${(hz / 1000).toFixed(1).replace(/\.0$/, "")} kHz`;
}

// ────────────────────────────────────────────────────────────────────────────
// Colonnes bâties sur les champs de la vue
// ────────────────────────────────────────────────────────────────────────────

export const COLONNES_FIXES: TrackColumn[] = [
  { key: "artist", label: "Artiste", width: "w-40", value: (t) => t.artist },
  { key: "album", label: "Album", width: "w-44", value: (t) => t.album },
  { key: "album_artist", label: "Artiste d'album", width: "w-40", value: (t) => t.album_artist },
  { key: "year", label: "Année", width: "w-14", numeric: true, value: (t) => t.year },
  { key: "genre", label: "Genre", width: "w-28", value: (t) => t.genre },
  { key: "rating", label: "Notation", width: "w-20", widget: "rating", sortOn: (t) => t.rating, sortKind: "number" },
  { key: "duration", label: "Durée", width: "w-12", align: "right", numeric: true, value: (t) => duree(t.duration), sortOn: (t) => t.duration, sortKind: "number" },
  { key: "track_number", label: "N° piste", width: "w-12", align: "right", numeric: true, value: (t) => (t.track_number ? String(t.track_number) : null), sortOn: (t) => t.track_number, sortKind: "number" },
  { key: "disc_number", label: "N° disque", width: "w-12", align: "right", numeric: true, value: (t) => (t.disc_number ? String(t.disc_number) : null), sortOn: (t) => t.disc_number, sortKind: "number" },
  { key: "play_count", label: "Écoutes", width: "w-14", align: "right", numeric: true, value: (t) => (t.play_count ? String(t.play_count) : null), sortOn: (t) => t.play_count || null, sortKind: "number" },
  { key: "last_played_at", label: "Dernière écoute", width: "w-24", numeric: true, value: (t) => date(t.last_played_at), sortOn: (t) => instant(t.last_played_at), sortKind: "number" },
  { key: "created_at", label: "Ajouté le", width: "w-24", numeric: true, value: (t) => date(t.created_at), sortOn: (t) => instant(t.created_at), sortKind: "number" },
  { key: "audio_format", label: "Format", width: "w-16", value: (t) => t.audio_format },
  { key: "extension", label: "Extension", width: "w-16", value: (t) => t.extension },
  { key: "bitrate", label: "Débit", width: "w-20", align: "right", numeric: true, value: (t) => formatBitrate(t.bitrate) || null, sortOn: (t) => t.bitrate, sortKind: "number" },
  { key: "sample_rate", label: "Fréquence", width: "w-20", align: "right", numeric: true, value: (t) => frequence(t.sample_rate), sortOn: (t) => t.sample_rate, sortKind: "number" },
  { key: "bits_per_sample", label: "Bits", width: "w-12", align: "right", numeric: true, value: (t) => (t.bits_per_sample ? `${t.bits_per_sample} bit` : null), sortOn: (t) => t.bits_per_sample, sortKind: "number" },
  { key: "channels", label: "Canaux", width: "w-16", value: (t) => formatChannels(t.channels) || null, sortOn: (t) => t.channels, sortKind: "number" },
  { key: "file_size", label: "Taille", width: "w-20", align: "right", numeric: true, value: (t) => poids(t.file_size ?? t.size), sortOn: (t) => t.file_size ?? t.size, sortKind: "number" },
  { key: "filename", label: "Nom de fichier", width: "w-52", value: (t) => t.filename },
  { key: "path", label: "Chemin", width: "w-72", value: (t) => t.path },
];

/**
 * Tags déjà servis par une colonne bâtie.
 *
 * Le tag `artist` du fichier et la colonne « Artiste » disent la même chose
 * neuf fois sur dix ; proposer les deux ne ferait qu'embrouiller le choix.
 * Quand ils divergent, c'est la valeur de la bibliothèque qui fait foi — c'est
 * elle qui sert au tri et au regroupement.
 */
const TAGS_DEJA_COUVERTS = new Set([
  "title",
  "artist",
  "album",
  "album_artist",
  "year",
  "genre",
  "track_number",
  "disc_number",
  "duration",
]);

/** Intitulés lisibles pour les champs nommés de la structure de tags. */
const INTITULES: Record<string, string> = {
  comment: "Commentaire",
  composer: "Compositeur",
  copyright: "Copyright",
  encoded_by: "Encodé par",
  encoding_settings: "Réglages d'encodage",
  bpm: "BPM",
  language: "Langue",
  publisher: "Éditeur",
  original_artist: "Artiste original",
  conductor: "Chef d'orchestre",
  lyricist: "Parolier",
  remix_artist: "Remixeur",
  arranged_by: "Arrangé par",
  interpreted_by: "Interprété par",
  mood: "Ambiance",
  isrc: "ISRC",
  subtitle: "Sous-titre",
  key: "Tonalité",
  compilation: "Compilation",
  media_type: "Support",
  part_of_set: "Partie de coffret",
  total_discs: "Nombre de disques",
};

/** Rend lisible une clé de tag dont on n'a pas d'intitulé traduit. */
export function intituleDeTag(cle: string): string {
  if (cle.startsWith("custom:")) return cle.slice("custom:".length);
  return INTITULES[cle] ?? cle.replace(/_/g, " ");
}

/** Construit la colonne correspondant à un tag recensé. */
export function colonneDeTag(cle: string): TrackColumn {
  return {
    key: `tag:${cle}`,
    label: intituleDeTag(cle),
    width: "w-32",
    value: (t) => valeurDeTag(t, cle),
  };
}

/** Un tag mérite-t-il d'être proposé, ou une colonne bâtie le sert-elle déjà ? */
export function tagProposable(cle: string): boolean {
  return !TAGS_DEJA_COUVERTS.has(cle);
}

// ────────────────────────────────────────────────────────────────────────────
// Résolution
// ────────────────────────────────────────────────────────────────────────────

const PAR_CLE = new Map(COLONNES_FIXES.map((c) => [c.key, c]));

/** Colonnes affichées par défaut : celles de la mise en page d'origine. */
export const COLONNES_PAR_DEFAUT = ["artist", "album", "rating", "duration"];

/**
 * Traduit une liste de clés enregistrées en colonnes affichables.
 *
 * Une clé inconnue est ignorée sans bruit : un tag peut disparaître d'une
 * bibliothèque entre deux sessions, et ce n'est pas une raison pour vider
 * toute la mise en page.
 */
export function resoudreColonnes(cles: string[]): TrackColumn[] {
  const sortie: TrackColumn[] = [];
  for (const cle of cles) {
    if (cle.startsWith("tag:")) {
      sortie.push(colonneDeTag(cle.slice("tag:".length)));
    } else {
      const c = PAR_CLE.get(cle);
      if (c) sortie.push(c);
    }
  }
  return sortie;
}

/**
 * Colonne correspondant à une clé de tri, y compris celles qui ne sont pas
 * affichées.
 *
 * On peut trier sur le titre, qui n'est jamais une colonne configurable, et
 * sur un tag qu'on vient de retirer de l'affichage. Résoudre à part du tableau
 * affiché évite que retirer une colonne ne casse silencieusement le tri.
 */
function colonneDeTri(cle: string): TrackColumn | null {
  if (cle === "title") {
    return { key: "title", label: "Titre", width: "flex-1", value: (t) => t.title };
  }
  if (cle.startsWith("tag:")) return colonneDeTag(cle.slice("tag:".length));
  return PAR_CLE.get(cle) ?? null;
}

/**
 * Range une liste de pistes déjà en mémoire.
 *
 * Rend un nouveau tableau plutôt que de trier sur place : la liste d'origine
 * reste disponible pour revenir à l'ordre naturel — celui de l'album, disque
 * puis piste — que rien ne saurait reconstituer autrement.
 */
export function trierPistes(
  tracks: TrackListView[],
  cle: string | null,
  dir: SortDir,
): TrackListView[] {
  if (!cle) return tracks;
  const col = colonneDeTri(cle);
  if (!col) return tracks;
  return [...tracks].sort(comparateur(col, dir));
}

/** Lit la liste enregistrée, en retombant sur la mise en page d'origine. */
export function lireColonnes(brut: string | undefined): string[] {
  if (!brut) return COLONNES_PAR_DEFAUT;
  try {
    const v = JSON.parse(brut);
    return Array.isArray(v) && v.every((x) => typeof x === "string")
      ? v
      : COLONNES_PAR_DEFAUT;
  } catch {
    return COLONNES_PAR_DEFAUT;
  }
}
