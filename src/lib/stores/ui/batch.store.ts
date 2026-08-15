/**
 * État du lot en cours.
 *
 * Un seul lot à la fois, volontairement : deux traitements de masse
 * concurrents sur la même bibliothèque se marcheraient dessus, et l'utilisateur
 * n'aurait aucun moyen de savoir lequel a touché quoi.
 */

import { writable, get } from "svelte/store";
import { listen } from "@tauri-apps/api/event";
import type { BatchProgress, BatchReport } from "$lib/services/batch/batch.service";

export type BatchState = {
  jobId: string | null;
  /** Ce qu'on traite — sert à titrer l'écran de suivi. */
  label: string;
  progress: BatchProgress | null;
  /** Renseigné une fois le lot terminé, y compris s'il a été annulé. */
  report: BatchReport | null;
  running: boolean;
  /** Vrai entre la demande d'annulation et le compte rendu. */
  cancelling: boolean;
  /**
   * Relance le lot sur les seuls fichiers en échec.
   *
   * Fournie par l'appelant plutôt que reconstruite ici : le panneau ne sait
   * pas ce qu'il exécute — écriture de tags aujourd'hui, renommage demain — et
   * ne doit pas avoir à le savoir pour rester réutilisable.
   */
  retry: ((paths: string[]) => Promise<void>) | null;
};

const initialState: BatchState = {
  jobId: null,
  label: "",
  progress: null,
  report: null,
  running: false,
  cancelling: false,
  retry: null,
};

const store = writable<BatchState>(initialState);

let listenersReady = false;

export const batchStore = {
  subscribe: store.subscribe,

  /** Déclare un lot lancé. Le suivi arrive ensuite par les événements. */
  begin: (
    jobId: string,
    label: string,
    retry: ((paths: string[]) => Promise<void>) | null = null,
  ) => store.set({ ...initialState, jobId, label, running: true, retry }),

  markCancelling: () => store.update((s) => ({ ...s, cancelling: true })),

  /** Ferme l'écran de suivi. Sans effet sur un lot encore en cours. */
  dismiss: () => store.set(initialState),

  /** Un lot est-il en cours ? Sert à refuser d'en lancer un second. */
  isRunning: () => get(store).running,
};

/**
 * Branche les deux événements du moteur de lot.
 *
 * Appelé une fois au démarrage, comme les autres écouteurs. Le garde-fou
 * évite un double abonnement en développement, où le module est rechargé à
 * chaud sans que l'application redémarre.
 */
export async function initBatchListeners() {
  if (listenersReady) return;
  listenersReady = true;

  await listen<BatchProgress>("batch-progress", (event) => {
    store.update((s) =>
      // Un événement d'un lot précédent ne doit pas ressusciter l'écran.
      s.jobId === event.payload.job_id ? { ...s, progress: event.payload } : s,
    );
  });

  await listen<BatchReport>("batch-done", (event) => {
    store.update((s) =>
      s.jobId === event.payload.job_id
        ? { ...s, report: event.payload, running: false, cancelling: false }
        : s,
    );
  });
}
