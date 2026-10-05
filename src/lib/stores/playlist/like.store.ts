import { get, writable } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import { profilSelector } from "../profil/profil.store";
import type { TrackLikedView } from "#lib/types/ui/like/trackLikedView";

type LikedState = {
  paths: Set<string>;
  loading: boolean;
};

function createLikedStore() {
  const { subscribe, set, update } = writable<LikedState>({
    paths: new Set(),
    loading: false
  });

  const likedCount = writable(0);

  async function refresh() {

    let profil = get(profilSelector);
    if(!profil.profilSelected) return;

    update((s) => ({ ...s, loading: true }));

    try {
      const rows = await invoke<TrackLikedView[]>('get_tracks_liked', { profilId: profil.profilSelected.id });
      const next = new Set(rows.map((r) => r.path));
      set({ paths: next, loading: false });
      likedCount.set(next.size);
    } catch (e) {
      console.error('[likes] rechargement :', e);
      update((s) => ({ ...s, loading: false }));
    }
  }

  async function like(path: string) {

    let profil = get(profilSelector);

    if(!profil.profilSelected) return;

    // si déjà présent, on évite de compter deux fois
    let already = false;
    update((s) => {
      already = s.paths.has(path);
      if (already) return s;

      const next = new Set(s.paths);
      next.add(path);
      likedCount.update((n) => n + 1);
      return { ...s, paths: next };
    });
    if (already) return;

    try {
      
      await invoke('add_track_liked', { path: path, profilId: profil.profilSelected.id});

    } catch (e) {
      // rollback
      update((s) => {
        const next = new Set(s.paths);
        if (next.delete(path)) likedCount.update((n) => Math.max(0, n - 1));
        return { ...s, paths: next };
      });
      throw e;
    }
  }

  async function unlike(path: string) {

    let profil = get(profilSelector);

    if(!profil.profilSelected) return;

    let existed = false;
    update((s) => {
      existed = s.paths.has(path);
      if (!existed) return s;

      const next = new Set(s.paths);
      next.delete(path);
      likedCount.update((n) => Math.max(0, n - 1));
      return { ...s, paths: next };
    });
    if (!existed) return;

    try {

      await invoke('remove_track_liked', { path: path, profilId: profil.profilSelected.id});
      
    } catch (e) {
      // rollback
      update((s) => {
        const next = new Set(s.paths);
        next.add(path);
        likedCount.update((n) => n + 1);
        return { ...s, paths: next };
      });
      throw e;
    }
  }

  async function toggle(path: string) {
    let isLikedNow = false;
    update((s) => {
      isLikedNow = s.paths.has(path);
      return s;
    });
    return isLikedNow ? unlike(path) : like(path);
  }

  /** Aime (ou retire) toute une liste : un album, un artiste. */
  async function toggleAll(paths: string[], aimer: boolean) {
    const deja = get({ subscribe }).paths;
    await Promise.all(paths.map((p) => (aimer ? (deja.has(p) ? null : like(p)) : unlike(p))));
  }

  return { subscribe, likedCount, refresh, like, unlike, toggle, toggleAll };
}

export const liked = createLikedStore();
export const likedCount = liked.likedCount;
