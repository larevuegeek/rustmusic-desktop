import { writable, get } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

/** Un morceau d'un serveur gardé hors ligne. */
export type OfflineItem = { path: string; server_id: number; ready: boolean; size: number | null };

type OfflineState = {
  /** Téléchargés : jouables serveur éteint. */
  ready: Set<string>;
  /** En attente de téléchargement. */
  pending: Set<string>;
  /** Par serveur : morceaux prêts, en attente, octets. */
  byServer: Record<number, { ready: number; pending: number; bytes: number }>;
};

const writer = writable<OfflineState>({ ready: new Set(), pending: new Set(), byServer: {} });

async function load() {
  const items = await invoke<OfflineItem[]>("offline_list").catch(() => null);
  if (!items) return;
  const state: OfflineState = { ready: new Set(), pending: new Set(), byServer: {} };
  for (const i of items) {
    (i.ready ? state.ready : state.pending).add(i.path);
    const s = (state.byServer[i.server_id] ??= { ready: 0, pending: 0, bytes: 0 });
    if (i.ready) s.ready++;
    else s.pending++;
    s.bytes += i.size ?? 0;
  }
  writer.set(state);
}

let started = false;
// Plusieurs morceaux arrivent d'affilée : un seul rechargement par rafale.
let reload: ReturnType<typeof setTimeout> | null = null;

export const offlineStore = {
  subscribe: writer.subscribe,
  load,

  /** Au lancement : l'état, puis ses changements (téléchargements, ajouts, retraits). */
  init() {
    if (started) return;
    started = true;
    void load();
    void listen("offline-changed", () => {
      if (reload) clearTimeout(reload);
      reload = setTimeout(() => void load(), 250);
    });
  },

  isReady: (path: string | null | undefined) => !!path && get(writer).ready.has(path),

  /** Garde ces morceaux hors ligne ; rend le nombre de nouveaux. */
  add: (paths: string[]) => invoke<number>("offline_add", { paths }),

  remove: (paths: string[]) => invoke<void>("offline_remove", { paths }),
};
