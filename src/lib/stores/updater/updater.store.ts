import { writable } from "svelte/store";

// ============================================================================
// STORE D'ÉTAT DE L'AUTO-UPDATER
// Pilote l'affichage du UpdateBanner et le cycle de vie du téléchargement.
// ============================================================================

export type UpdateState =
  | { kind: "idle" }
  | { kind: "checking" }
  | { kind: "available"; version: string; notes?: string; date?: string }
  | { kind: "downloading"; version: string; downloaded: number; total: number | null }
  | { kind: "installing"; version: string }
  | { kind: "ready"; version: string } // installé, attend le restart
  | { kind: "error"; message: string };

export const updaterState = writable<UpdateState>({ kind: "idle" });

/** Issue de la dernière vérification, même silencieuse : « À jour » ne se dit qu'après un succès. */
export const derniereVerification = writable<"jamais" | "a-jour" | "maj" | "echec">("jamais");
