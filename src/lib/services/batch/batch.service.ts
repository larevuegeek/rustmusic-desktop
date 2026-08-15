/**
 * Traitements par lot.
 *
 * Miroir de `src-tauri/src/core/batch/runner.rs` et de
 * `src-tauri/src/commands/batch_command.rs`.
 *
 * Générique par construction : l'écriture de tags est le premier usage, le
 * renommage et le déplacement suivront la même mécanique — on lance, on suit
 * la progression, on rend compte.
 */

import { invoke } from "@tauri-apps/api/core";
import type { TagEditPayload } from "$lib/services/tags/tagEditor.service";

/** Émis après chaque fichier. */
export type BatchProgress = {
  job_id: string;
  done: number;
  total: number;
  failed: number;
  /** Le fichier qui vient d'être traité. */
  current: string;
};

/** Un fichier qui n'a pas pu être traité, et pourquoi. */
export type BatchFailure = {
  path: string;
  error: string;
};

/** Émis une fois, à la fin, quelle qu'en soit la cause. */
export type BatchReport = {
  job_id: string;
  total: number;
  succeeded: number;
  /** Seuls les échecs sont listés : c'est sur eux qu'on agit ensuite. */
  failures: BatchFailure[];
  cancelled: boolean;
};

/** Fichiers jamais atteints — non nul seulement après une annulation. */
export function skipped(report: BatchReport): number {
  return report.total - report.succeeded - report.failures.length;
}

/**
 * Applique le même jeu de modifications à plusieurs fichiers.
 *
 * Rend l'identifiant du lot **immédiatement** : le traitement se poursuit en
 * fond. Attendre la fin bloquerait l'interface pendant des minutes sur une
 * bibliothèque réseau.
 */
export async function writeTagsBatch(
  paths: string[],
  edit: TagEditPayload,
): Promise<string> {
  return invoke<string>("write_tags_batch", { paths, edit });
}

/** Un fichier et les modifications qui lui sont propres. */
export type TagEditItem = {
  path: string;
  edit: TagEditPayload;
};

/**
 * Applique à chaque fichier des modifications **différentes**.
 *
 * Complète `writeTagsBatch`, qui applique le même jeu à tous. Numéroter les
 * pistes d'un album de 1 à N donne par définition une valeur par fichier.
 *
 * Chaque élément prépare ses propres images : sans conséquence pour une
 * numérotation, qui n'en comporte pas, mais ce n'est pas la commande à choisir
 * pour poser une pochette sur des centaines de fichiers.
 */
export async function writeTagsEach(items: TagEditItem[]): Promise<string> {
  return invoke<string>("write_tags_each", { items });
}

/**
 * Demande l'annulation d'un lot.
 *
 * Faux si le lot est inconnu ou déjà terminé. L'arrêt n'est pas immédiat : il
 * est constaté entre deux fichiers, jamais pendant une écriture. Le compte
 * rendu final arrive normalement.
 */
export async function cancelBatch(jobId: string): Promise<boolean> {
  return invoke<boolean>("cancel_batch", { jobId });
}

/** Nom de fichier seul, pour l'affichage — les chemins complets sont illisibles. */
export function fileName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}
