// Export automatique des playlists : la passe Rust relancée après un délai quand une playlist change,
// après un scan (playlists intelligentes) et au démarrage (règles « pas écoutée depuis… »).
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { get } from "svelte/store";
import { open } from "@tauri-apps/plugin-dialog";
import { t } from "#lib/i18n";
import { playlistStore } from "#lib/stores/playlist/playlist.store";
import { settingsStore } from "#lib/stores/settings/settings.store";
import { toasts } from "#lib/stores/ui/toast.store";

export type ExportReport = {
  no_dir: boolean;
  playlists: number;
  written: number;
  tracks: number;
  absolute: number;
  skipped: number;
  errors: string[];
};

let timer: ReturnType<typeof setTimeout> | null = null;
let started = false;

/** Une erreur ne s'affiche qu'une fois tant qu'elle ne change pas. */
let lastError = "";
function reportErrors(r: ExportReport) {
  const message = r.errors.join(" · ");
  if (message && message !== lastError) toasts.push({ type: "error", title: get(t)("playlist_export.title"), message });
  lastError = message;
}

export async function runAutoExport(): Promise<ExportReport | null> {
  try {
    const r = await invoke<ExportReport>("playlist_auto_export_run");
    reportErrors(r);
    return r;
  } catch (e) {
    console.error("[export auto]", e);
    return null;
  }
}

/** Relance différée : plusieurs changements rapprochés ne font qu'une passe. */
export function scheduleAutoExport(delay = 3000) {
  if (!get(playlistStore).playlists.some((p) => p.auto_export)) return;
  if (timer) clearTimeout(timer);
  timer = setTimeout(() => void runAutoExport(), delay);
}

export function initAutoExport() {
  if (started) return;
  started = true;
  playlistStore.subscribe(() => scheduleAutoExport());
  void listen("rescan-complete", () => scheduleAutoExport(1000));
  // Au démarrage, toutes les playlists marquées (tous profils), une fois l'interface prête.
  setTimeout(() => void runAutoExport(), 8000);
}

/** Le dossier d'export, choisi s'il ne l'est pas encore ; `null` si l'utilisateur renonce. */
export async function ensureExportDir(): Promise<string | null> {
  const current = get(settingsStore).playlist_export_dir;
  if (current) return current;
  const dir = await open({ directory: true, title: get(t)("playlist_export.choose_dir") }).catch(() => null);
  if (typeof dir !== "string") return null;
  await settingsStore.set("playlist_export_dir", dir);
  return dir;
}

/** Option d'une playlist ; à l'activation, demande le dossier s'il manque. Rend vrai si c'est fait. */
export async function setPlaylistAutoExport(playlistId: number, name: string, enabled: boolean): Promise<boolean> {
  const tr = get(t);
  if (enabled && !(await ensureExportDir())) return false;
  try {
    const r = await invoke<ExportReport>("playlist_set_auto_export", { playlistId, enabled });
    await playlistStore.refresh();
    reportErrors(r);
    toasts.push({ type: "success", title: name, message: tr(enabled ? "playlist_export.enabled" : "playlist_export.disabled") });
    return true;
  } catch (e) {
    toasts.push({ type: "error", title: tr("playlist_export.title"), message: String(e) });
    return false;
  }
}
