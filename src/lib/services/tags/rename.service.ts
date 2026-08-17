/**
 * Renommage des fichiers d'après un motif.
 * Miroir de `src-tauri/src/commands/rename_command.rs`.
 *
 * Ces appels **n'écrivent rien** : ils calculent où chaque fichier irait, et
 * ce qui l'en empêche. Corriger cinq mille fichiers avec un motif erroné n'est
 * pas rattrapable à la main — l'aperçu à blanc est ce qui permet d'oser.
 */

import { invoke } from "@tauri-apps/api/core";

/** Ce qui arriverait à un fichier. */
export type MoveView = {
  from: string;
  /** Nom court, pour l'affichage — le chemin complet ne tient pas. */
  from_name: string;
  to: string;
  to_name: string;
  /** La cible est identique à la source : rien à faire. */
  unchanged: boolean;
  blocked: boolean;
  /** Identifiants d'obstacles, clés de traduction `rename.issue.<id>`. */
  issues: string[];
  /** Champs cités par le motif et vides sur ce fichier. */
  missing: string[];
};

export type RenamePreview = {
  moves: MoveView[];
  ready: number;
  blocked: number;
  unchanged: number;
};

/** Les tags d'un fichier, sous les noms de l'atelier. */
export type RenameInput = {
  path: string;
  tags: Record<string, string>;
};

/**
 * Vérifie un motif et rend les champs qu'il cite.
 *
 * Séparé de l'aperçu pour que le message d'erreur s'affiche à la frappe, sans
 * attendre le calcul sur des milliers de fichiers.
 */
export async function checkPattern(pattern: string): Promise<string[]> {
  return invoke<string[]>("check_pattern", { pattern });
}

export async function previewRename(
  files: RenameInput[],
  pattern: string,
  restructure = false,
  libraryRoot: string | null = null,
): Promise<RenamePreview> {
  return invoke<RenamePreview>("preview_rename", {
    files,
    pattern,
    restructure,
    libraryRoot,
  });
}

/** Ce qu'un lot a produit. */
export type MoveOutcome = {
  /** Identifiant du lot au journal — c'est par lui qu'on annule. */
  batch_id: string;
  succeeded: number;
  failed: { path: string; message: string }[];
  /**
   * Les couples avant/après réellement appliqués.
   *
   * Indispensable à l'interface : ses écrans tiennent des chemins en mémoire,
   * et recharger avec les anciens ne trouverait plus rien.
   */
  moved: [string, string][];
};

/** Un lot du journal. */
export type JournalEntry = {
  id: string;
  kind: string;
  pattern: string | null;
  created_at: string;
  total: number;
  succeeded: number;
  failed: number;
  undone_at: string | null;
};

/**
 * Ordonne les déplacements pour que les chaînes se résolvent.
 *
 * `a → b` et `b → c` : déplacer `a` en premier écraserait `b`. Échoue sur une
 * permutation circulaire, qui ne se dénoue qu'en deux passes.
 */
export async function orderMoves(
  moves: [string, string][],
): Promise<[string, string][]> {
  return invoke<[string, string][]>("order_moves", { moves });
}

/**
 * Exécute le lot : déplace les fichiers **et** met la base à jour.
 *
 * On envoie les déplacements et non le motif : l'aperçu a déjà tranché, et
 * recalculer côté serveur ouvrirait un écart entre ce qui a été montré et ce
 * qui est fait.
 */
/** Ce qui accompagne un déplacement. */
export type MoveOptions = {
  /** Emporter paroles, feuillet et pochette. */
  satellites: boolean;
  /** Supprimer les dossiers devenus vides. */
  cleanup_empty: boolean;
  /** Renseigné par le backend depuis la base — jamais par l'interface. */
  roots: string[];
};

export async function applyRename(
  moves: [string, string][],
  pattern: string,
  libraryId: number | null = null,
  restructure = false,
  options: Partial<MoveOptions> | null = null,
): Promise<MoveOutcome> {
  return invoke<MoveOutcome>("apply_rename", {
    libraryId,
    pattern,
    restructure,
    moves,
    options: options
      ? { satellites: false, cleanup_empty: false, roots: [], ...options }
      : null,
  });
}

/**
 * Les règles de nettoyage, dans leur ordre d'application.
 *
 * Aucune n'est cochée d'office : ce qui est du bruit chez l'un est une
 * intention chez l'autre. La mise en capitales est explicitement anglaise —
 * l'appliquer à un catalogue francophone abîmerait des titres corrects.
 */
export const CLEAN_RULES = [
  "underscores",
  "leading_number",
  "featuring",
  "spaces",
  "title_case",
] as const;

export type CleanRule = (typeof CLEAN_RULES)[number];

/**
 * Propose une version nettoyée d'un jeu de valeurs.
 *
 * Ne rend **que ce qui change** : renvoyer le reste ferait passer tout le lot
 * pour modifié dans le tableur.
 */
export async function cleanTags(
  values: [string, string][],
  rules: CleanRule[],
): Promise<[string, string][]> {
  return invoke<[string, string][]>("clean_tags", { values, rules });
}

/** Les dossiers scannés d'une bibliothèque, destinations possibles. */
export type LibraryDir = { id: string; path: string; name: string };

export async function libraryDirs(libraryId: number): Promise<LibraryDir[]> {
  return invoke<LibraryDir[]>("get_library_dirs", { libraryId });
}

export async function listBatchJournal(limit = 20): Promise<JournalEntry[]> {
  return invoke<JournalEntry[]>("list_batch_journal", { limit });
}

export async function undoBatch(batchId: string): Promise<MoveOutcome> {
  return invoke<MoveOutcome>("undo_batch", { batchId });
}

/**
 * Les conventions qu'on rencontre réellement.
 *
 * Quatre, pas quinze : une liste de motifs prédéfinis n'a d'intérêt que si on
 * la lit en entier. Au-delà, on ne les compare plus et on retape le sien.
 */
export const PRESETS: { label: string; pattern: string }[] = [
  { label: "01 - Titre", pattern: "{track:02} - {title}.{ext}" },
  { label: "01. Artiste - Titre", pattern: "{track:02}. {artist} - {title}.{ext}" },
  { label: "Artiste - Titre", pattern: "{artist} - {title}.{ext}" },
  // Le groupe optionnel évite le « 1-01 » sur les albums à disque unique.
  { label: "1-01 - Titre (coffret)", pattern: "[{disc}-]{track:02} - {title}.{ext}" },
];

/**
 * Motifs d'arborescence.
 *
 * Le repli `{albumartist|artist}` est ce qui évite d'éclater une compilation en
 * autant de dossiers qu'elle compte d'artistes.
 */
export const TREE_PRESETS: { label: string; pattern: string }[] = [
  {
    label: "Artiste / Album / 01 - Titre",
    pattern: "{albumartist|artist}/{album}/{track:02} - {title}.{ext}",
  },
  {
    label: "Artiste / Album (Année) / 01 - Titre",
    pattern: "{albumartist|artist}/{album} ({year})/{track:02} - {title}.{ext}",
  },
  {
    label: "Genre / Artiste / Album / 01 - Titre",
    pattern: "{genre}/{albumartist|artist}/{album}/{track:02} - {title}.{ext}",
  },
  {
    label: "Artiste / Album / 1-01 - Titre",
    pattern: "{albumartist|artist}/{album}/[{disc}-]{track:02} - {title}.{ext}",
  },
];

/** Les champs qu'un motif peut citer, pour l'aide à la saisie. */
export const PATTERN_FIELDS = [
  "artist",
  "albumartist",
  "album",
  "title",
  "year",
  "genre",
  "track",
  "disc",
  "composer",
  "ext",
] as const;
