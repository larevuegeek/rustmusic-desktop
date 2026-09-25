import type { Playlist } from "$lib/types/db/playlist/Playlist";
import { profilSelector } from "$lib/stores/profil/profil.store";
import { writable, get } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";

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
      });

      playlistWriter.update(state => ({
        ...state,
        playlists
      }));
    } catch (error) {
      console.error("Failed to refresh playlists", error);
    }
  },
  addPlaylist: async (profilId: number, name: string, description: string | null, color: string = '#8b5cf6', icon: string = 'mynaui:music') => {
    const newPlaylist: Playlist = await invoke<Playlist>('create_playlist', {
      payload: {
        profil_id: profilId,
        name: name.trim(),
        description: description?.trim() || null,
        color,
        icon
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
