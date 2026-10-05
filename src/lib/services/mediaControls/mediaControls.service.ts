import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { get } from "svelte/store";

import { player } from "#lib/stores/player/player.store";
import { queueState } from "#lib/stores/queue/queueState.store";
import { settingsStore } from "#lib/stores/settings/settings.store";
import { playerService } from "#lib/services/player/player.service";

// ============================================================================
// SERVICE FRONTEND POUR SMTC (System Media Transport Controls)
//
// Côté backend, `souvlaki` parle aux 3 OS (Windows SMTC, macOS Now Playing,
// Linux MPRIS). Ici on fait juste la plomberie :
//   1. Push les métadonnées du morceau courant chaque fois qu'il change
//   2. Push l'état de lecture (playing / paused / stopped) au fil du status
//   3. Écoute les events "smtc-command" émis par le Rust quand l'OS demande
//      play/pause/next/prev (touches média clavier, mini-player Windows, etc.)
//
// L'enable/disable est piloté par le réglage `system_media_controls`. Quand
// désactivé, on coupe complètement l'intégration côté Rust (rien n'apparaît
// dans l'OS) → utile sur Linux sans D-Bus ou Windows N.
// ============================================================================

let unsubPlayer: (() => void) | null = null;
let unlistenCommand: UnlistenFn | null = null;
let enabled = false;

/** Dernier trackId pushé en métadonnées — évite de re-pousser pour rien. */
let lastTrackId: string | null = null;
/** Dernier statut pushé — pareil, anti-spam. */
let lastStatus = "";

function pushCurrentMetadata() {
  const qs = get(queueState);
  const idx = qs.currentIndex;
  if (idx < 0) return;
  const track = qs.tracks[idx];
  if (!track) return;

  // La cover sur le QueueTrack est typiquement un chemin fichier full ou une
  // miniature. On passe au Rust qui se charge de convertir en file:// URI.
  // Si pas de cover ou cover par défaut, on n'envoie rien (l'OS affichera
  // l'icône de l'app à la place).
  const coverPath =
    track.cover && track.cover !== "/images/no-cd.png"
      ? track.cover
      : undefined;

  invoke("update_media_metadata", {
    payload: {
      title: track.title || "RustMusic",
      artist: track.artist,
      album: undefined, // QueueTrack n'a pas d'album, on le laisse vide
      cover_path: coverPath,
      duration_secs: track.duration,
    },
  }).catch((e) => console.warn("[smtc] metadata push failed:", e));
}

function pushPlaybackState(status: string, positionSecs: number | undefined) {
  let state: "playing" | "paused" | "stopped";
  if (status === "playing") state = "playing";
  else if (status === "paused") state = "paused";
  else state = "stopped"; // idle, ready, ended → stopped pour l'OS

  invoke("update_media_playback", {
    payload: {
      state,
      position_secs:
        positionSecs && positionSecs > 0 && Number.isFinite(positionSecs)
          ? positionSecs
          : undefined,
    },
  }).catch((e) => console.warn("[smtc] playback push failed:", e));
}

async function attachListeners() {
  // Listener Tauri : commandes OS → playerService
  unlistenCommand = await listen<string>("smtc-command", (event) => {
    const cmd = event.payload;
    switch (cmd) {
      // Un bureau peut envoyer « pause » au verrouillage ou à la veille : une bascule relançait alors la musique.
      case "play":
        if (get(player).status !== "playing") playerService.handleTogglePlay();
        break;
      case "pause":
        if (get(player).status === "playing") playerService.pauseFile();
        break;
      case "toggle":
        playerService.handleTogglePlay();
        break;
      case "next":
        playerService.nextTrack();
        break;
      case "previous":
        playerService.prevTrack();
        break;
      case "stop":
        playerService.stopPlay();
        break;
      // "quit" / "raise" → ignorés (on ne ferme/refocus pas l'app depuis SMTC)
    }
  });

  // Abonnement player store : track change → metadata, status change → state
  unsubPlayer = player.subscribe((p) => {
    if (!enabled) return;

    if (p.trackId && p.trackId !== lastTrackId) {
      lastTrackId = p.trackId;
      pushCurrentMetadata();
    }

    if (p.status !== lastStatus) {
      lastStatus = p.status;
      pushPlaybackState(p.status, p.jsPosition);
    }
  });
}

function detachListeners() {
  if (unlistenCommand) {
    unlistenCommand();
    unlistenCommand = null;
  }
  if (unsubPlayer) {
    unsubPlayer();
    unsubPlayer = null;
  }
  lastTrackId = null;
  lastStatus = "";
}

export const mediaControlsService = {
  /**
   * Active SMTC côté backend + branche les listeners frontend.
   * Idempotent.
   */
  async enable() {
    if (enabled) return;
    try {
      await invoke("enable_media_controls");
    } catch (e) {
      console.warn("[smtc] enable failed:", e);
      return;
    }
    enabled = true;
    await attachListeners();

    // Si un morceau est déjà chargé au moment de l'activation, on pousse
    // tout de suite pour ne pas attendre le prochain changement.
    const p = get(player);
    if (p.trackId) {
      lastTrackId = p.trackId;
      lastStatus = p.status;
      pushCurrentMetadata();
      pushPlaybackState(p.status, p.jsPosition);
    }
  },

  /** Désactive SMTC + détache les listeners. Idempotent. */
  async disable() {
    if (!enabled) return;
    enabled = false;
    detachListeners();
    try {
      await invoke("disable_media_controls");
    } catch (e) {
      console.warn("[smtc] disable failed:", e);
    }
  },

  /**
   * Synchronise l'état runtime avec le réglage utilisateur.
   * À appeler au démarrage et après chaque toggle dans les réglages.
   */
  async sync() {
    const wanted = settingsStore.get("system_media_controls") === "true";
    if (wanted && !enabled) {
      await this.enable();
    } else if (!wanted && enabled) {
      await this.disable();
    }
  },

  isEnabled() {
    return enabled;
  },
};
