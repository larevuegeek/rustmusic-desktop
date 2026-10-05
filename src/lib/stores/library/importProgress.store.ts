import { writable, derived, get } from "svelte/store";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { dataCache } from "#lib/stores/cache/dataCache.store";
import { libraryContentStore } from "#lib/stores/library/libraryContent.store";
import { libraryStore } from "#lib/stores/library/library.store";
import { toasts } from "#lib/stores/ui/toast.store";
import { t, currentLocale } from "#lib/i18n";
import type { Library } from "#lib/types/db/library/Library";

export type ImportProgressState = {
  active: boolean;
  libraryId: number | null;
  directory: string;
  /** `listing` : recherche des fichiers ; `importing` : analyse. */
  phase: 'listing' | 'importing';
  found: number;
  current: number;
  total: number;
  percent: number;
  fileName: string;
  elapsedMs: number;
  /** Arrêt demandé, en attente de la fin du lot en cours. */
  stopping: boolean;
  cancelled: boolean;
  /** Nouveaux morceaux pas encore comptés par la bibliothèque. */
  added: number;
};

type Cible = { library_id: number; directory: string };

export type ImportCompletePayload = Cible & {
  total: number;
  duration_ms: number;
  cancelled: boolean;
};

export type ImportProgressPayload = Cible & {
  added: number;
  current: number;
  total: number;
  percent: number;
  file_name: string;
};

const initialState: ImportProgressState = {
  active: false,
  libraryId: null,
  directory: '',
  phase: 'listing',
  found: 0,
  current: 0,
  total: 0,
  percent: 0,
  fileName: '',
  elapsedMs: 0,
  stopping: false,
  cancelled: false,
  added: 0,
};

const importWriter = writable<ImportProgressState>(initialState);

let unlisteners: UnlistenFn[] = [];
let startTime = 0;
let reset: ReturnType<typeof setTimeout> | undefined;

/** Incrémenté à chaque import validé : les pages qui chargent leurs listes elles-mêmes rechargent. */
export const importsTermines = writable(0);

/** La bibliothèque avant l'import, pour le bilan d'un arrêt. */
let avant: Library | null = null;

function bilanPartiel(libraryId: number) {
  const apres = get(libraryStore).libraries.find(l => l.id === libraryId);
  const tr = get(t);
  const parties = ([
    ['total_tracks', 'tracks'],
    ['total_albums', 'albums'],
    ['total_artists', 'artists'],
  ] as const).flatMap(([cle, nom]) => {
    const n = Math.max(0, (apres?.[cle] ?? 0) - (avant?.[cle] ?? 0));
    return n ? [`${n.toLocaleString(get(currentLocale))} ${tr(`library_head.${nom}_${n === 1 ? 'one' : 'n'}`)}`] : [];
  });
  toasts.push({
    type: "warning",
    title: tr("notify.import_partial"),
    message: parties.length ? parties.join(' · ') : tr("notify.import_partial_none"),
    duration: 6000,
  });
}

function suivre(libraryId: number) {
  clearTimeout(reset);
  if (avant?.id !== libraryId) avant = get(libraryStore).libraries.find(l => l.id === libraryId) ?? null;
}

/** Fin d'import ou de rescan : l'état reste affiché le temps du bilan. */
function terminer() {
  importWriter.update(state => ({ ...state, active: false, stopping: false }));
  avant = null;
  clearTimeout(reset);
  reset = setTimeout(() => {
    importWriter.set(initialState);
    startTime = 0;
  }, 3000);
}

export const importProgressStore = {
  subscribe: importWriter.subscribe,

  init: async () => {
    const u1 = await listen<Cible>('import-start', (event) => {
      suivre(event.payload.library_id);
      startTime = Date.now();
      importWriter.set({
        ...initialState,
        active: true,
        libraryId: event.payload.library_id,
        directory: event.payload.directory,
      });
    });

    const u2 = await listen<Cible & { found: number }>('import-listing', (event) => {
      suivre(event.payload.library_id);
      importWriter.update(state => ({
        ...state,
        active: true,
        cancelled: false,
        libraryId: event.payload.library_id,
        directory: event.payload.directory,
        phase: 'listing',
        found: event.payload.found,
      }));
    });

    const u3 = await listen<ImportProgressPayload>('import-progress', (event) => {
      if (startTime === 0) startTime = Date.now();
      suivre(event.payload.library_id);
      importWriter.update(state => ({
        ...state,
        active: true,
        cancelled: false,
        added: event.payload.added,
        libraryId: event.payload.library_id,
        directory: event.payload.directory,
        phase: 'importing',
        current: event.payload.current,
        total: event.payload.total,
        percent: event.payload.percent,
        fileName: event.payload.file_name,
        elapsedMs: Date.now() - startTime,
      }));
    });

    const u4 = await listen<ImportCompletePayload>('import-complete', async (event) => {
      const { library_id, cancelled } = event.payload;
      importWriter.update(state => ({
        ...state,
        cancelled,
        percent: 100,
        elapsedMs: event.payload.duration_ms,
      }));

      dataCache.invalidateAll();
      libraryContentStore.refresh();
      // Les ajouts en attente ne s'effacent qu'une fois la bibliothèque relue.
      await libraryStore.refresh();
      importWriter.update(state => ({ ...state, added: 0 }));
      importsTermines.update(n => n + 1);
      if (cancelled) bilanPartiel(library_id);
      terminer();
    });

    // Un rescan arrêté pendant le listage n'émet pas `import-complete`.
    const u5 = await listen('rescan-complete', terminer);

    unlisteners = [u1, u2, u3, u4, u5];
  },

  start: () => {
    startTime = Date.now();
    importWriter.set({ ...initialState, active: true });
  },

  /** Arrête l'import ou le rescan en cours ; ce qui est importé reste. */
  stop: async () => {
    importWriter.update(state => ({ ...state, stopping: state.active }));
    try {
      await invoke('cancel_task', { taskId: 'import' });
    } catch (e) {
      console.error('[import] stop failed:', e);
    }
  },

  destroy: () => {
    unlisteners.forEach(u => u());
    unlisteners = [];
  }
};

/** Progression à la place de l'état vide : ajout en cours, ou import dans une bibliothèque encore vide. */
export const importAffiche = derived([libraryStore, importWriter], ([$lib, $imp]) =>
  $lib.isImporting
  || ($imp.libraryId !== null && $lib.libraries.some(l => l.id === $imp.libraryId && l.total_tracks === 0))
);

/** Pistes d'une bibliothèque, avec celles que l'import en cours a déjà ajoutées. */
export const pistesAvecImport = derived(importWriter, ($imp) =>
  (lib: Pick<Library, 'id' | 'total_tracks'> | null | undefined): number =>
    (lib?.total_tracks ?? 0) + (lib && lib.id === $imp.libraryId ? $imp.added : 0)
);
