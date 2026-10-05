import { recent } from "#lib/stores/recent/recent.store";
import { toasts } from "#lib/stores/ui/toast.store";
import type { RecentFile } from "#lib/types/db/recent/RecentFile";
import { invoke } from "@tauri-apps/api/core";
import { get } from "svelte/store";
import { t } from "#lib/i18n";

export async function handleRemoveRecentItem(recentFile: RecentFile) {

    await invoke<void>('remove_recent_file', { path : recentFile.path });

    recent.refreshRecent();

    //AddToast
    toasts.push({
      type: "info",
      title: get(t)("notify.recent_removed"),
      message: get(t)("notify.recent_removed_desc")
    });
}