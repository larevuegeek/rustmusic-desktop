/**
 * État de l'atelier de tags.
 *
 * # Le modèle : une modification par fichier
 *
 * L'éditeur de sélection applique **la même** valeur à tous les fichiers.
 * L'atelier fait l'inverse : chaque fichier porte ses propres modifications,
 * et une action de masse n'est qu'une écriture répétée sur les lignes visées.
 *
 * Ce choix n'est pas un détail d'implémentation, c'est ce qui rend les deux
 * vues possibles avec un seul état. Le tableur produit naturellement des
 * modifications par ligne ; le panneau, qui édite N fichiers à la fois, écrit
 * la même valeur sur N lignes. Sans ce modèle il faudrait deux états et une
 * traduction de l'un vers l'autre à chaque bascule.
 *
 * # Ce qui est stocké
 * `original` garde ce que le fichier contient, `edits` uniquement ce qui a
 * changé. Un champ absent d'`edits` n'est pas modifié — la même convention que
 * le backend, ce qui évite toute traduction à l'écriture.
 */

import { writable, get } from "svelte/store";
import { readWorkshopTracks } from "$lib/services/tags/tagEditor.service";
import type { TagEditItem } from "$lib/services/batch/batch.service";

/** Les champs que l'atelier sait travailler, dans l'ordre des colonnes. */
export const WORKSHOP_FIELDS = [
  "track_number",
  "title",
  "artist",
  "album",
  "album_artist",
  "year",
  "genre",
  "disc_number",
  "total_tracks",
  "total_discs",
  "composer",
  "comment",
] as const;

export type WorkshopField = (typeof WORKSHOP_FIELDS)[number];

/** Champs affichés par défaut dans le tableur — les autres se déplient. */
export const DEFAULT_COLUMNS: WorkshopField[] = [
  "track_number",
  "title",
  "artist",
  "album",
  "year",
  "genre",
];

/** Champs qui se comparent comme des nombres, pas comme du texte. */
const NUMERIC_FIELDS: WorkshopField[] = [
  "track_number",
  "disc_number",
  "total_tracks",
  "total_discs",
  "year",
];

/** Ceux dont l'absence signale un fichier à corriger. */
const ESSENTIAL_FIELDS: WorkshopField[] = ["title", "artist", "album"];

export type WorkshopFile = {
  path: string;
  /** Valeurs lues dans le fichier. */
  original: Record<string, string>;
  /** Uniquement les champs modifiés. Vide = rien à écrire. */
  edits: Record<string, string>;
  /** Faux quand le fichier n'a pas pu être lu — il reste visible mais inerte. */
  readable: boolean;
  /** Vignette de la pochette actuelle, `null` s'il n'y en a pas. */
  cover: string | null;
  /**
   * Pochette choisie mais pas encore écrite.
   *
   * Séparée des champs texte parce qu'elle ne se compare pas de la même façon :
   * on garde le chemin du fichier source et son aperçu, pas une valeur.
   */
  pendingCover: { path: string; src: string } | null;
};

export type WorkshopView = "grid" | "panel";

export type WorkshopState = {
  /** Ce qu'on est venu corriger, pour l'afficher en en-tête. */
  source: string;
  /**
   * Adresse d'où l'on vient, pour pouvoir y retourner.
   *
   * Nommer la destination vaut mieux qu'un retour arrière aveugle : l'atelier
   * s'ouvre depuis trois endroits — un album, un dossier, une sélection — et
   * l'utilisateur doit savoir où le bouton le ramène.
   */
  origin: string | null;
  files: WorkshopFile[];
  /** Chemins cochés. Le panneau édite ceux-là, le tableur les met en avant. */
  selected: Set<string>;
  view: WorkshopView;
  loading: boolean;
  error: string | null;
};

const initialState: WorkshopState = {
  source: "",
  origin: null,
  files: [],
  selected: new Set(),
  view: "grid",
  loading: false,
  error: null,
};

const store = writable<WorkshopState>(initialState);

/** Valeur affichée : la modification si elle existe, sinon celle du fichier. */
export function valueOf(file: WorkshopFile, field: string): string {
  return file.edits[field] ?? file.original[field] ?? "";
}

export function isDirty(file: WorkshopFile, field: string): boolean {
  return field in file.edits;
}

export function fileIsDirty(file: WorkshopFile): boolean {
  return Object.keys(file.edits).length > 0 || file.pendingCover !== null;
}

/** Un champ essentiel manque — c'est ce que l'atelier signale en priorité. */
export function isIncomplete(file: WorkshopFile): boolean {
  return ESSENTIAL_FIELDS.some((field) => valueOf(file, field).trim() === "");
}

function replaceFile(
  files: WorkshopFile[],
  path: string,
  change: (file: WorkshopFile) => WorkshopFile,
): WorkshopFile[] {
  return files.map((file) => (file.path === path ? change(file) : file));
}

/**
 * Pose une valeur, ou retire la modification si elle revient à l'original.
 *
 * Ce retour à zéro compte : sans lui, corriger une faute puis la défaire
 * laisserait le fichier marqué comme modifié et le ferait réécrire pour rien —
 * or réécrire un fichier n'est jamais gratuit, surtout sur un partage réseau.
 */
function applyValue(file: WorkshopFile, field: string, value: string): WorkshopFile {
  const edits = { ...file.edits };
  if (value === (file.original[field] ?? "")) {
    delete edits[field];
  } else {
    edits[field] = value;
  }
  return { ...file, edits };
}

export const tagWorkshop = {
  subscribe: store.subscribe,

  /**
   * Charge un jeu de travail.
   *
   * Les tags sont lus **dans les fichiers**, pas dans la base : celle-ci ne
   * stocke ni commentaire, ni compositeur, ni totaux, et les afficher vides
   * les effacerait à l'écriture.
   */
  async load(paths: string[], source: string, origin: string | null = null) {
    store.set({ ...initialState, source, origin, loading: true, view: get(store).view });

    // Un seul appel pour tout le jeu de travail : cent allers-retours par le
    // pont IPC coûtaient bien plus que la lecture elle-même.
    const tracks = await readWorkshopTracks(paths).catch(() => []);

    // Un fichier au format non réinscriptible n'a rien à faire dans l'atelier :
    // aucune écriture ne pourra jamais l'enregistrer.
    const results: WorkshopFile[] = tracks.map((track) => {
      const original: Record<string, string> = {};
      for (const field of WORKSHOP_FIELDS) {
        const raw = (track.tags as any)[field];
        original[field] = raw === null || raw === undefined ? "" : String(raw);
      }
      return {
        path: track.path,
        original,
        edits: {},
        readable: track.readable && track.writable,
        cover: track.cover,
        pendingCover: null,
      };
    });

    store.update((s) => ({
      ...s,
      files: results,
      selected: new Set(results.filter((f) => f.readable).map((f) => f.path)),
      loading: false,
      error: results.every((f) => !f.readable) && results.length > 0
        ? "unreadable"
        : null,
    }));
  },

  setView: (view: WorkshopView) => store.update((s) => ({ ...s, view })),

  set(path: string, field: string, value: string) {
    store.update((s) => ({
      ...s,
      files: replaceFile(s.files, path, (file) => applyValue(file, field, value)),
    }));
  },

  /** Écrit la même valeur sur tous les fichiers cochés — action du panneau. */
  setOnSelected(field: string, value: string) {
    store.update((s) => ({
      ...s,
      files: s.files.map((file) =>
        s.selected.has(file.path) && file.readable
          ? applyValue(file, field, value)
          : file,
      ),
    }));
  },

  /**
   * Recopie la valeur de la première ligne cochée sur toutes les autres.
   *
   * L'action reine du tableur : douze fichiers d'un même album ont le même
   * artiste et la même année, et un seul est juste.
   */
  fillDown(field: string) {
    store.update((s) => {
      const targets = s.files.filter((f) => f.readable && s.selected.has(f.path));
      if (targets.length === 0) return s;

      const reference = valueOf(targets[0], field);
      return {
        ...s,
        files: s.files.map((file) =>
          s.selected.has(file.path) && file.readable
            ? applyValue(file, field, reference)
            : file,
        ),
      };
    });
  },

  /** Numérote les lignes cochées de 1 à N, dans l'ordre affiché, et pose le total. */
  numberSelected() {
    store.update((s) => {
      const targets = s.files.filter((f) => f.readable && s.selected.has(f.path));
      const total = String(targets.length);
      let position = 0;

      return {
        ...s,
        files: s.files.map((file) => {
          if (!s.selected.has(file.path) || !file.readable) return file;
          position += 1;
          // Numéroter sans le total laisse le travail à moitié fait.
          return applyValue(
            applyValue(file, "track_number", String(position)),
            "total_tracks",
            total,
          );
        }),
      };
    });
  },

  /** Vide la colonne sur les lignes cochées. */
  clearOnSelected(field: string) {
    store.update((s) => ({
      ...s,
      files: s.files.map((file) =>
        s.selected.has(file.path) && file.readable ? applyValue(file, field, "") : file,
      ),
    }));
  },

  /**
   * Trie le jeu de travail.
   *
   * Le tri réordonne **le magasin**, pas seulement l'affichage. C'est
   * indispensable : « numéroter de 1 à N dans l'ordre affiché » n'aurait aucun
   * sens si l'ordre affiché et l'ordre interne divergeaient.
   *
   * Les champs numériques se comparent comme des nombres — sinon la piste 10
   * passerait avant la 9.
   */
  sortBy(field: WorkshopField, ascending: boolean) {
    store.update((s) => {
      const numeric = NUMERIC_FIELDS.includes(field);
      const files = [...s.files].sort((a, b) => {
        const left = valueOf(a, field);
        const right = valueOf(b, field);

        let comparison: number;
        if (numeric) {
          // Une valeur absente n'est pas zéro : elle se range à part, toujours
          // en fin, pour ne pas se mêler aux vraies valeurs basses.
          const l = left.trim() === "" ? Number.NaN : Number(left);
          const r = right.trim() === "" ? Number.NaN : Number(right);
          if (Number.isNaN(l) && Number.isNaN(r)) comparison = 0;
          else if (Number.isNaN(l)) return 1;
          else if (Number.isNaN(r)) return -1;
          else comparison = l - r;
        } else {
          if (left === "" && right === "") comparison = 0;
          else if (left === "") return 1;
          else if (right === "") return -1;
          else comparison = left.localeCompare(right, undefined, { numeric: true });
        }

        return ascending ? comparison : -comparison;
      });

      return { ...s, files };
    });
  },

  /**
   * Pose une pochette sur les lignes cochées.
   *
   * Elle remplace la seule pochette avant et laisse les autres images de chaque
   * fichier en place — on ignore ce qu'ils contiennent (cf. `ImagePlan::SetCover`).
   */
  setCoverOnSelected(cover: { path: string; src: string }) {
    store.update((s) => ({
      ...s,
      files: s.files.map((file) =>
        s.selected.has(file.path) && file.readable
          ? { ...file, pendingCover: cover }
          : file,
      ),
    }));
  },

  /**
   * Pose une pochette sur des fichiers **désignés**.
   *
   * Distinct de `setCoverOnSelected` : la récupération depuis une source en
   * ligne ne concerne que les fichiers qui ont trouvé leur piste, pas toute la
   * sélection. Poser la pochette d'un album sur les morceaux qu'il ne contient
   * pas serait faux.
   */
  setCoverOn(paths: string[], cover: { path: string; src: string }) {
    const wanted = new Set(paths);
    store.update((s) => ({
      ...s,
      files: s.files.map((file) =>
        wanted.has(file.path) && file.readable ? { ...file, pendingCover: cover } : file,
      ),
    }));
  },

  clearPendingCover(path: string) {
    store.update((s) => ({
      ...s,
      files: replaceFile(s.files, path, (file) => ({ ...file, pendingCover: null })),
    }));
  },

  revertField(path: string, field: string) {
    store.update((s) => ({
      ...s,
      files: replaceFile(s.files, path, (file) => {
        const edits = { ...file.edits };
        delete edits[field];
        return { ...file, edits };
      }),
    }));
  },

  revertAll() {
    store.update((s) => ({
      ...s,
      files: s.files.map((file) => ({ ...file, edits: {}, pendingCover: null })),
    }));
  },

  toggle(path: string) {
    store.update((s) => {
      const selected = new Set(s.selected);
      if (selected.has(path)) selected.delete(path);
      else selected.add(path);
      return { ...s, selected };
    });
  },

  selectAll(paths?: string[]) {
    store.update((s) => ({
      ...s,
      selected: new Set(paths ?? s.files.filter((f) => f.readable).map((f) => f.path)),
    }));
  },

  selectNone: () => store.update((s) => ({ ...s, selected: new Set() })),

  /**
   * Traduit l'état en charge utile d'écriture.
   *
   * Seuls les fichiers réellement modifiés y figurent : réécrire un fichier
   * intact n'apporte rien et coûte une recopie complète.
   */
  pendingWrites(): TagEditItem[] {
    return get(store)
      .files.filter((file) => file.readable && fileIsDirty(file))
      .map((file) => ({
        path: file.path,
        edit: file.pendingCover
          ? { ...file.edits, cover: { path: file.pendingCover.path, picture_type: 3 } }
          : { ...file.edits },
      }));
  },

  /** Retire les modifications des fichiers écrits avec succès. */
  markWritten(paths: string[]) {
    const written = new Set(paths);
    store.update((s) => ({
      ...s,
      files: s.files.map((file) =>
        written.has(file.path)
          ? {
              ...file,
              original: { ...file.original, ...file.edits },
              edits: {},
              // La pochette posée devient celle du fichier.
              cover: file.pendingCover?.src ?? file.cover,
              pendingCover: null,
            }
          : file,
      ),
    }));
  },

  clear: () => store.set({ ...initialState, view: get(store).view }),
};
