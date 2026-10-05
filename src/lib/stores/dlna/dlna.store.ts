/**
 * Reactive DLNA server status. Refreshed at app launch and after each
 * lifecycle command (start/stop/update_settings).
 */

import { writable } from "svelte/store";
import {
  dlnaGetStatus,
  type DlnaStatus,
} from "#lib/services/dlna/dlna.service";

export const dlnaStatusStore = writable<DlnaStatus | null>(null);

/** Fetch latest status from the backend and update the store. */
export async function refreshDlnaStatus(): Promise<DlnaStatus | null> {
  try {
    const status = await dlnaGetStatus();
    dlnaStatusStore.set(status);
    return status;
  } catch (e) {
    console.error("[dlna] refresh status failed:", e);
    return null;
  }
}
