import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { get } from "svelte/store";
import { t } from "$lib/i18n";

import { derniereVerification, updaterState } from "$lib/stores/updater/updater.store";

// ============================================================================
// SERVICE D'AUTO-UPDATE
//
// Workflow standard Tauri 2 :
//   1. `check()` interroge l'endpoint `rustmusic.dev/api/releases/{target}/{current_version}`
//      → renvoie `Update | null`
//   2. Si non-null, on met le store en "available" → UpdateBanner s'affiche
//   3. L'utilisateur clique "Installer" → `downloadAndInstall()` + onProgress
//   4. Une fois fini, `relaunch()` redémarre l'app
//
// L'utilisateur peut dismiss le banner sans installer (state revient à "idle").
// ============================================================================

/** Référence vers l'objet Update courant (pour pouvoir le downloadAndInstall plus tard). */
let pendingUpdate: Update | null = null;

/**
 * Lance une vérification de mise à jour côté serveur.
 * `silent: true` (défaut) → ne change pas l'UI si erreur ou pas d'update.
 * `silent: false` (check manuel depuis les réglages) → affiche aussi les
 * cas "pas d'update" et "erreur" pour donner du feedback.
 */
export async function checkForUpdate(silent = true): Promise<void> {
  // Si déjà en train de télécharger, on ne relance pas une nouvelle check
  const current = get(updaterState);
  if (current.kind === "downloading" || current.kind === "installing") {
    return;
  }

  updaterState.set({ kind: "checking" });
  try {
    const update = await check();
    derniereVerification.set(update ? "maj" : "a-jour");
    if (update) {
      pendingUpdate = update;
      updaterState.set({
        kind: "available",
        version: update.version,
        notes: update.body ?? undefined,
        date: update.date ?? undefined,
      });
    } else {
      if (!silent) {
        updaterState.set({ kind: "idle" });
      } else {
        updaterState.set({ kind: "idle" });
      }
    }
  } catch (e) {
    const message = String((e as Error)?.message ?? e ?? get(t)("updater.check_failed"));
    derniereVerification.set("echec");
    if (silent) {
      // Échec silencieux : pas de réseau, serveur down, signature invalide…
      // On log mais on n'embête pas l'utilisateur.
      console.warn("[updater] check failed:", message);
      updaterState.set({ kind: "idle" });
    } else {
      updaterState.set({ kind: "error", message });
    }
  }
}

/**
 * Télécharge + installe la mise à jour disponible, puis relance l'app.
 * Doit être appelé après un `checkForUpdate()` ayant retourné une update.
 */
export async function downloadAndInstall(): Promise<void> {
  if (!pendingUpdate) {
    updaterState.set({ kind: "error", message: get(t)("updater.nothing_to_install") });
    return;
  }

  const version = pendingUpdate.version;
  updaterState.set({ kind: "downloading", version, downloaded: 0, total: null });

  try {
    let totalBytes: number | null = null;
    let downloadedBytes = 0;

    await pendingUpdate.downloadAndInstall((event) => {
      switch (event.event) {
        case "Started":
          totalBytes = event.data.contentLength ?? null;
          updaterState.set({ kind: "downloading", version, downloaded: 0, total: totalBytes });
          break;
        case "Progress":
          downloadedBytes += event.data.chunkLength;
          updaterState.set({
            kind: "downloading",
            version,
            downloaded: downloadedBytes,
            total: totalBytes,
          });
          break;
        case "Finished":
          updaterState.set({ kind: "installing", version });
          break;
      }
    });

    updaterState.set({ kind: "ready", version });
    // Restart automatique 1.5s plus tard pour laisser le user voir le message
    setTimeout(() => {
      relaunch().catch((e) => console.error("[updater] relaunch failed:", e));
    }, 1500);
  } catch (e) {
    const message = String((e as Error)?.message ?? e ?? get(t)("updater.install_failed"));
    updaterState.set({ kind: "error", message });
  }
}

/** Ferme le banner sans installer (l'utilisateur veut le faire plus tard). */
export function dismissUpdate(): void {
  pendingUpdate = null;
  updaterState.set({ kind: "idle" });
}

/**
 * Init : à appeler au démarrage de l'app, avec un délai pour ne pas ralentir
 * le cold start. Vérifie silencieusement et met le store à jour si update dispo.
 */
export function initUpdaterAutoCheck(delayMs = 5000): void {
  setTimeout(() => {
    checkForUpdate(true).catch((e) => console.warn("[updater] auto-check failed:", e));
  }, delayMs);
}
