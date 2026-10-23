import type { Playlist } from "#lib/types/db/playlist/Playlist";
import { profilSelector } from "#lib/stores/profil/profil.store";
import { writable, get } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import { sourceFilter } from "#lib/stores/source/sourceFilter.store";
import { toasts } from "#lib/stores/ui/toast.store";
import { t } from "#lib/i18n";

export type PlaylistState = {
  playlists: Playlist[],
  isLoading: boolean,
}

const initialState: PlaylistState = {
  playlists: [],
  isLoading: false
}

const playlistWriter = writable<PlaylistState>(initialState);

export const playlistStore = {
  subscribe: playlistWriter.subscribe,
  init: async () => {
    playlistWriter.update(state => ({
      ...state,
      isLoading: true
    }));

    const profilState = get(profilSelector);
    const profilId = profilState.profilSelected?.id;

    if (!profilId) {
      console.warn("No profil selected, skip playlist init");
      playlistWriter.update(state => ({
        ...state,
        isLoading: false
      }));
      return;
    }

    try {
      const playlists = await invoke<Playlist[]>('get_playlists', {
        profilId: profilId,
        sourceId: get(sourceFilter),
      });

      playlistWriter.update(state => ({
        ...state,
        playlists,
        isLoading: false
      }));
    } catch (error) {
      console.error("Failed to load playlists", error);
      playlistWriter.update(state => ({
        ...state,
        isLoading: false
      }));
    }
  },
  refresh: async () => {
    const profilState = get(profilSelector);
    const profilId = profilState.profilSelected?.id;

    if (!profilId) return;

    try {
      const playlists = await invoke<Playlist[]>('get_playlists', {
        profilId: profilId,
        sourceId: get(sourceFilter),
      });

      playlistWriter.update(state => ({
        ...state,
        playlists
      }));
    } catch (error) {
      console.error("Failed to refresh playlists", error);
    }
  },
  /** `serverId` : créée sur ce serveur et synchronisée avec lui. `sourceId` : sa source (défaut : celle affichée). */
  addPlaylist: async (profilId: number, name: string, description: string | null, color: string = '#8b5cf6', icon: string = 'mynaui:music', serverId: number | null = null, sourceId: number | null = null) => {
    const newPlaylist: Playlist = await invoke<Playlist>('create_playlist', {
      payload: {
        profil_id: profilId,
        name: name.trim(),
        description: description?.trim() || null,
        color,
        icon,
        server_id: serverId,
        source_id: serverId ?? sourceId ?? get(sourceFilter) ?? 0,
      }
    });

    playlistWriter.update(state => {
      const playlists = [...state.playlists, newPlaylist];

      return {
        ...state,
        playlists
      };
    });

    return newPlaylist;
  },
  updatePlaylist: async (id: number, name: string, description: string | null, color: string, icon: string) => {
    const updated: Playlist = await invoke<Playlist>('update_playlist', {
      payload: {
        id,
        name: name.trim(),
        description: description?.trim() || null,
        color,
        icon
      }
    });

    playlistWriter.update(state => ({
      ...state,
      playlists: state.playlists.map(p => p.id === id ? updated : p)
    }));
  },
  /** Nouvel ordre des playlists (barre latérale) ; `ids` dans l'ordre voulu, les autres suivent. */
  reorder: async (ids: number[]) => {
    const profilId = get(profilSelector).profilSelected?.id;
    if (!profilId) return;
    const avant = get(playlistWriter).playlists;
    const rang = new Map(ids.map((id, i) => [id, i]));
    const reste = avant.filter((p) => !rang.has(p.id));
    const ordre = [...avant.filter((p) => rang.has(p.id)).sort((a, b) => rang.get(a.id)! - rang.get(b.id)!), ...reste]
      .map((p, i) => ({ ...p, position: i }));
    playlistWriter.update((state) => ({ ...state, playlists: ordre }));
    try {
      await invoke('reorder_playlists', { profilId, ids: ordre.map((p) => p.id) });
    } catch (e) {
      playlistWriter.update((state) => ({ ...state, playlists: avant }));
      toasts.push({ type: 'error', title: get(t)('notify.error'), message: String(e) });
    }
  },
  /** Épingle la playlist dans la barre latérale, ou l'en retire. */
  setPinned: async (id: number, pinned: boolean) => {
    try {
      await invoke('set_playlist_pinned', { playlistId: id, pinned });
    } catch (e) {
      toasts.push({ type: 'error', title: get(t)('notify.error'), message: String(e) });
      return;
    }
    playlistWriter.update(state => ({
      ...state,
      playlists: state.playlists.map(p => p.id === id ? { ...p, pinned } : p)
    }));
  },
  removePlaylist: async (id: number) => {
    await invoke('delete_playlist', {
      playlistId: id
    });

    playlistWriter.update(state => ({
      ...state,
      playlists: state.playlists.filter(p => p.id !== id)
    }));
  },
  clear: () => {
    playlistWriter.set(initialState);
  }
};
