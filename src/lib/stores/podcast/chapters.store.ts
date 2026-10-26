import { writable, get } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";

export type Chapter = { start: number; title: string; url?: string };

type ChaptersState = { path: string | null; list: Chapter[]; open: boolean };

const writer = writable<ChaptersState>({ path: null, list: [], open: false });

/** Chapitres de l'épisode en cours (vide hors épisode ou sans chapitres) ; `open` : liste affichée. */
export const chapters = {
  subscribe: writer.subscribe,

  async load(path: string) {
    writer.set({ path, list: [], open: false });
    const list = await invoke<Chapter[]>("podcast_chapters", { path }).catch(() => []);
    // Un autre morceau a pu démarrer pendant la recherche.
    if (get(writer).path === path) writer.update((s) => ({ ...s, list }));
  },

  clear() {
    writer.set({ path: null, list: [], open: false });
  },

  setOpen(open: boolean) {
    writer.update((s) => ({ ...s, open }));
  },
};

/** Index du chapitre joué à `position` (secondes), -1 avant le premier. */
export function chapterAt(list: Chapter[], position: number): number {
  let i = -1;
  for (let k = 0; k < list.length && list[k].start <= position + 0.25; k++) i = k;
  return i;
}
