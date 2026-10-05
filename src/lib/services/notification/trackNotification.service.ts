import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";
import { invoke } from "@tauri-apps/api/core";
import { queueState } from "#lib/stores/queue/queueState.store";
import { settingsStore } from "#lib/stores/settings/settings.store";
import type { QueueTrack } from "#lib/types/db/queue/QueueTrack";

// ============================================================================
// SERVICE DE NOTIFICATION OS — CHANGEMENT DE MORCEAU
// Affiche une notification native (Windows action center, macOS, Linux libnotify)
// chaque fois que le morceau courant change, avec cover + titre + artiste.
// Activable via Réglages → Notifications.
// ============================================================================

let unsubscribe: (() => void) | null = null;

/** Dernière queueId notifiée — évite de re-notifier sur un même morceau. */
let lastNotifiedQueueId = "";

/**
 * Skip de la TOUTE PREMIÈRE émission du store. Au démarrage de l'app, la file
 * d'attente est restaurée depuis la BDD ; ça met `currentIndex` à 0+ sans que
 * l'utilisateur ait demandé à jouer quoi que ce soit — il ne faut pas le
 * notifier d'un morceau qu'il n'a pas lancé.
 */
let firstEmission = true;

/** Cache du résultat de la demande de permission OS (1 seul check par session). */
let permissionState: "unknown" | "granted" | "denied" = "unknown";

async function ensurePermission(): Promise<boolean> {
  if (permissionState === "granted") return true;
  if (permissionState === "denied") return false;

  try {
    let granted = await isPermissionGranted();
    if (!granted) {
      const result = await requestPermission();
      granted = result === "granted";
    }
    permissionState = granted ? "granted" : "denied";
    return granted;
  } catch (e) {
    console.warn("[notif] permission check failed:", e);
    permissionState = "denied";
    return false;
  }
}

/**
 * Convertit le chemin de cover stocké en BDD (path full ou null) en un chemin
 * fichier absolu que l'OS peut lire pour l'icône de la notif. On utilise la
 * miniature 2x (taille suffisante pour la zone d'icône sans surcoût mémoire).
 * Retourne undefined si pas de cover dispo ou erreur de résolution.
 */
async function resolveIconPath(coverPath: string | undefined): Promise<string | undefined> {
  if (!coverPath || coverPath === "/images/no-cd.png") return undefined;
  // Si c'est déjà une URL distante on ne peut pas l'utiliser comme icône OS.
  if (coverPath.startsWith("http") || coverPath.startsWith("data:")) return undefined;

  try {
    const thumbnailPath = coverPath.replace(/[\/\\]full[\/\\]/, "/2x/");
    return await invoke<string>("resolve_cover_thumbnail", { path: thumbnailPath });
  } catch {
    return undefined;
  }
}

async function fireNotification(track: QueueTrack) {
  // Réglage utilisateur OFF → on ne notifie pas, mais on continue de tracer la
  // queueId courante pour ne pas spammer si l'utilisateur réactive le réglage.
  if (settingsStore.get("show_notifications") !== "true") return;

  const ok = await ensurePermission();
  if (!ok) return;

  const icon = await resolveIconPath(track.cover);
  const title = track.title || "RustMusic";
  const body = track.artist ?? "";

  try {
    sendNotification({ title, body, icon });
  } catch (e) {
    console.warn("[notif] sendNotification failed:", e);
  }
}

export const trackNotificationService = {
  init() {
    if (unsubscribe) return; // déjà initialisé

    unsubscribe = queueState.subscribe((qs) => {
      if (qs.currentIndex < 0) return;
      const track = qs.tracks[qs.currentIndex];
      if (!track) return;

      const key = track.queueId;
      if (key === lastNotifiedQueueId) return;
      lastNotifiedQueueId = key;

      // Première émission post-init : on ne notifie pas (restore de la queue
      // au démarrage de l'app, pas une vraie action utilisateur).
      if (firstEmission) {
        firstEmission = false;
        return;
      }

      fireNotification(track);
    });
  },

  destroy() {
    if (unsubscribe) {
      unsubscribe();
      unsubscribe = null;
    }
    lastNotifiedQueueId = "";
    firstEmission = true;
    permissionState = "unknown";
  },
};
