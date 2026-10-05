import { writable } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import type { RecentFileListView } from "#lib/types/ui/recent/RecentFileListView";

function createRecentStore() {
  const { subscribe, set, update } = writable<RecentFileListView[]>([]);

  let loading = false;
  const totalTracks = writable(0);

  async function refreshRecent() {
    if (loading) return;
        loading = true;
    try {

      const files = await invoke<RecentFileListView[]>('get_recent_files');
      
      set(files);
      totalTracks.set(files.length);

    } finally {
      loading = false;
    }
  }

  async function clearRecent(): Promise<void> {
      set([])
      totalTracks.set(0);

      await invoke<void>('clear_recent_files');
  }

  return { subscribe, refreshRecent, totalTracks, clearRecent };
}

export const recent = createRecentStore();
export const recentCount = recent.totalTracks;
