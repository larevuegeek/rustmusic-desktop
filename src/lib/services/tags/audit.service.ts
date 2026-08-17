/**
 * Inventaire de ce qu'il y a à corriger dans une bibliothèque.
 * Miroir de `src-tauri/src/commands/audit_command.rs`.
 *
 * C'est le point d'entrée du travail par lot : sans lui, l'atelier n'a de
 * sens que si l'on sait déjà quel album ouvrir.
 */

import { invoke } from "@tauri-apps/api/core";

/**
 * Ce qu'un constat demande comme geste, et donc où il se range à l'écran.
 *
 * `blocked` est à part : il n'y a rien à corriger, le format n'accepte pas de
 * réécriture. On le montre quand même — un fichier qu'on croit corrigeable et
 * qui ne l'est pas est une perte de temps qu'on ne veut découvrir qu'une fois.
 */
export type Severity =
  | "essential"
  | "incomplete"
  | "coherence"
  | "duplicate"
  | "blocked";

export type AuditGroup = {
  /** Identifiant stable, clé de traduction (`audit.kind.<kind>`). */
  kind: string;
  severity: Severity;
  count: number;
  /** Les fichiers concernés, à verser tels quels dans l'atelier. */
  paths: string[];
  /** Quelques noms, pour que la ligne dise de quoi elle parle. */
  samples: string[];
};

export type AuditReport = {
  /** Nombre de morceaux passés en revue — le dénominateur de tout le reste. */
  scanned: number;
  groups: AuditGroup[];
};

export async function auditLibrary(libraryId: number): Promise<AuditReport> {
  return invoke<AuditReport>("audit_library", { libraryId });
}

/** Ordre d'affichage : ce qui empêche de retrouver un morceau d'abord. */
export const SEVERITY_ORDER: Severity[] = [
  "essential",
  "coherence",
  "duplicate",
  "incomplete",
  "blocked",
];
