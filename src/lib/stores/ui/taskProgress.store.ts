import { writable, derived, get } from "svelte/store";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { t, currentLocale } from "$lib/i18n";
import { tailleLisible } from "$lib/helper/tools/sizeTools";

// ============================================================================
// STORE GLOBAL DE TÂCHES EN COURS
// Centralise toutes les progressions (import, images artistes, rescan, etc.)
// Affiché dans la StatusBar en bas à droite
// ============================================================================

export type Task = {
  id: string;
  label: string;
  icon: string;
  current: number;
  total: number;
  percent: number;
  detail: string;        // ex: nom du fichier en cours
  active: boolean;
  completedAt?: number;  // timestamp de fin pour auto-dismiss
};

type TaskMap = Record<string, Task>;

const taskWriter = writable<TaskMap>({});

let unlisteners: UnlistenFn[] = [];

const defaultTask: Task = {
  id: "", label: "", icon: "", current: 0, total: 0,
  percent: 0, detail: "", active: true,
};

function upsertTask(id: string, update: Partial<Task>) {
  taskWriter.update(tasks => {
    const existing = tasks[id] ?? { ...defaultTask, id };
    return {
      ...tasks,
      [id]: { ...existing, ...update }
    };
  });
}

function completeTask(id: string) {
  taskWriter.update(tasks => {
    if (!tasks[id]) return tasks;
    return {
      ...tasks,
      [id]: { ...tasks[id], active: false, percent: 100, completedAt: Date.now() }
    };
  });

  // Auto-dismiss après 4s
  setTimeout(() => {
    taskWriter.update(tasks => {
      const { [id]: _, ...rest } = tasks;
      return rest;
    });
  }, 4000);
}

export const taskProgressStore = {
  subscribe: taskWriter.subscribe,

  init: async () => {
    // ─── Import de fichiers ───
    const u1 = await listen<{ current: number; total: number; percent: number; file_name: string }>(
      'import-progress', (e) => {
        upsertTask('import', {
          label: get(t)("system.task_importing"),
          icon: "lucide:hard-drive-download",
          current: e.payload.current,
          total: e.payload.total,
          percent: e.payload.percent,
          detail: e.payload.file_name,
        });
      }
    );

    const u2 = await listen<{ total: number; duration_ms: number }>(
      'import-complete', (e) => {
        upsertTask('import', {
          label: get(t)(e.payload.total === 1 ? "system.task_import_done_one" : "system.task_import_done_n").replace("{n}", String(e.payload.total)),
          percent: 100,
          current: e.payload.total,
          total: e.payload.total,
          detail: "",
        });
        completeTask('import');
      }
    );

    // ─── Images artistes ───
    const u3 = await listen<{ current: number; total: number; artist_name: string }>(
      'artist-image-progress', (e) => {
        const pct = e.payload.total > 0 ? Math.round((e.payload.current * 100) / e.payload.total) : 0;
        upsertTask('artist-images', {
          label: get(t)("common.artist_images"),
          icon: "lucide:image-down",
          current: e.payload.current,
          total: e.payload.total,
          percent: pct,
          detail: e.payload.artist_name,
        });

        // Auto-complete quand current == total
        if (e.payload.current >= e.payload.total) {
          completeTask('artist-images');
        }
      }
    );

    // ─── Rescan bibliothèque ───
    const u4 = await listen<{ library_name: string }>(
      'rescan-start', (e) => {
        upsertTask('rescan', {
          label: get(t)("system.task_rescan").replace("{name}", e.payload.library_name),
          icon: "lucide:refresh-cw",
          percent: 0,
          detail: "",
        });
      }
    );

    const u5 = await listen<{ library_name: string }>(
      'rescan-complete', (_e) => {
        completeTask('rescan');
      }
    );

    const u5b = await listen<{ library_id: number; current: number; total: number }>(
      'rescan-progress', (e) => {
        const { current, total } = e.payload;
        upsertTask('rescan', { current, total, percent: total > 0 ? Math.round(current * 100 / total) : 0 });
      }
    );

    // ─── Migration miniatures (covers + artistes, même event) ───
    const u6 = await listen<{ current: number; total: number; percent: number; file_name: string }>(
      'migration-progress', (e) => {
        upsertTask('migration', {
          label: get(t)("system.task_thumbnails"),
          icon: "lucide:folder-sync",
          current: e.payload.current,
          total: e.payload.total,
          percent: e.payload.percent,
          detail: e.payload.file_name,
        });

        if (e.payload.current >= e.payload.total) {
          completeTask('migration');
        }
      }
    );

    // ─── Pré-décodage du morceau (profil Minimal) ───
    const u7 = await listen<boolean>('playback-preparing', (e) => {
      if (e.payload) {
        upsertTask('playback-preparing', {
          label: get(t)("player.preparing"),
          icon: "lucide:loader-circle",
          percent: 0,
          total: 0,
          detail: "",
          active: true,
        });
      } else {
        // Auto-dismiss : le morceau est prêt à jouer.
        completeTask('playback-preparing');
      }
    });

    // Progression du pré-décodage (octets décodés / total estimé).
    const u8 = await listen<{ decoded_bytes: number; total_bytes: number }>(
      'playback-preparing-progress',
      (e) => {
        const { decoded_bytes, total_bytes } = e.payload;
        const locale = get(currentLocale);
        const lu = tailleLisible(decoded_bytes, locale) || "0";
        const percent = total_bytes > 0
          ? Math.min(99, Math.round((decoded_bytes * 100) / total_bytes))
          : 0;
        upsertTask('playback-preparing', {
          label: get(t)("player.preparing"),
          icon: "lucide:loader-circle",
          current: decoded_bytes,
          total: total_bytes,
          percent,
          detail: total_bytes > 0
            ? `${lu} / ${tailleLisible(total_bytes, locale)}`
            : lu,
          active: true,
        });
      },
    );

    unlisteners = [u1, u2, u3, u4, u5, u5b, u6, u7, u8];
  },

  cancel: (id: string) => {
    taskWriter.update(tasks => {
      const { [id]: _, ...rest } = tasks;
      return rest;
    });
  },

  destroy: () => {
    unlisteners.forEach(u => u());
    unlisteners = [];
  },
};

// Derived : liste des tâches visibles (actives + récemment terminées)
export const activeTasks = derived(taskWriter, ($tasks) =>
  Object.values($tasks).sort((a, b) => {
    // Actives d'abord, puis complétées récemment
    if (a.active !== b.active) return a.active ? -1 : 1;
    return 0;
  })
);

// Derived : y a-t-il au moins une tâche en cours ?
export const hasActiveTasks = derived(taskWriter, ($tasks) =>
  Object.values($tasks).some(t => t.active)
);
