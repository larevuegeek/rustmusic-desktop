import { writable, get } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import { t } from "#lib/i18n";
import { player } from "#lib/stores/player/player.store";
import { toasts } from "#lib/stores/ui/toast.store";
import { radioStore, toResult } from "#lib/stores/radio/radio.store";
import { profilSelector } from "#lib/stores/profil/profil.store";
import type { RadioResult } from "#lib/types/ui/radio/Radio";

type RadioStatus = "connecting" | "playing" | "paused" | "error";

type RadioPlayerState = {
  /** Station en cours ; `null` hors mode radio. */
  station: RadioResult | null;
  logoPath: string | null;
  radioId: number | null;
  status: RadioStatus;
  /** Titre annoncé par la station pour ce qu'on entend. */
  title: string | null;
  /** Retard sur le direct (pause en différé), en secondes. */
  behindSecs: number;
  bitrate: number | null;
};

const idle: RadioPlayerState = { station: null, logoPath: null, radioId: null, status: "connecting", title: null, behindSecs: 0, bitrate: null };
const state = writable<RadioPlayerState>(idle);

type LiveStatus = { title: string | null; bitrate: number; behind_secs: number; paused: boolean };

// Relu chaque seconde : titre, retard (le moteur voit toutes les pauses : bouton, clavier, touches média).
let poll: ReturnType<typeof setInterval> | null = null;
function startPolling() {
  stopPolling();
  poll = setInterval(async () => {
    const s = await invoke<LiveStatus | null>("radio_status").catch(() => null);
    if (!s) return;
    state.update((x) => {
      if (!x.station) return x;
      const status = x.status === "playing" || x.status === "paused" ? (s.paused ? "paused" : "playing") : x.status;
      return { ...x, status, title: s.title ?? x.title, behindSecs: s.behind_secs, bitrate: s.bitrate || x.bitrate };
    });
  }, 1000);
}
function stopPolling() {
  if (poll) clearInterval(poll);
  poll = null;
}

// Une demande de lecture plus récente l'emporte sur celle en cours de connexion.
let playSeq = 0;

/** Les favorites, pour Précédent / Suivant. */
function neighbour(step: 1 | -1): { station: RadioResult; logoPath: string | null; radioId: number } | null {
  const favoris = get(radioStore).radios;
  if (favoris.length === 0) return null;
  const cur = get(state).station;
  const i = favoris.findIndex((r) => cur && (r.stream_url === cur.stream_url || (cur.uuid && r.uuid === cur.uuid)));
  const next = favoris[(i + step + favoris.length) % favoris.length];
  return { station: toResult(next), logoPath: next.logo_path, radioId: next.id };
}

export const radioPlayer = {
  subscribe: state.subscribe,

  isActive: () => get(state).station !== null,

  /** Lance une station ; la musique s'arrête, sa file reste intacte. */
  async play(station: RadioResult, logoPath: string | null = null, radioId: number | null = null) {
    const seq = ++playSeq;
    const favorite = get(radioStore).radios.find((r) => r.stream_url === station.stream_url || (station.uuid && r.uuid === station.uuid));
    state.set({
      ...idle,
      station,
      logoPath: logoPath ?? favorite?.logo_path ?? null,
      radioId: radioId ?? favorite?.id ?? null,
      bitrate: station.bitrate,
    });
    player.update({ status: "idle", isPreparing: false, jsPosition: 0, rustPosition: 0 });
    try {
      const profilId = get(profilSelector).profilSelected?.id;
      if (profilId == null) throw new Error("aucun profil");
      await invoke("radio_play", { profilId, station, radioId: radioId ?? favorite?.id ?? null });
      if (seq !== playSeq) return;
      state.update((x) => ({ ...x, status: "playing" }));
      startPolling();
    } catch (err) {
      if (seq !== playSeq) return;
      state.update((x) => ({ ...x, status: "error" }));
      toasts.push({ type: "error", title: get(t)("radios.error_play"), message: String(err) });
    }
  },

  async toggle() {
    const s = get(state);
    if (!s.station) return;
    if (s.status === "error") return radioPlayer.play(s.station, s.logoPath, s.radioId);
    if (s.status === "connecting") return;
    await invoke("pause_play").catch(() => {});
    state.update((x) => ({ ...x, status: x.status === "playing" ? "paused" : "playing" }));
  },

  /** Abandonne le différé et repart du direct. */
  async goLive() {
    await invoke("radio_go_live").catch((e) => toasts.push({ type: "error", title: get(t)("radios.error_play"), message: String(e) }));
    state.update((x) => ({ ...x, status: "playing", behindSecs: 0 }));
  },

  async stop() {
    playSeq++;
    stopPolling();
    state.set(idle);
    await invoke("radio_stop").catch(() => {});
  },

  /** La musique reprend la main (le backend a déjà coupé la radio). */
  detach() {
    if (!get(state).station) return;
    playSeq++;
    stopPolling();
    state.set(idle);
  },

  /** Le flux s'est arrêté de lui-même (radio perdue après les reconnexions). */
  onEnded() {
    stopPolling();
    state.update((x) => ({ ...x, status: "error" }));
    toasts.push({ type: "error", title: get(t)("radios.lost") });
  },

  next: () => {
    const n = neighbour(1);
    return n ? radioPlayer.play(n.station, n.logoPath, n.radioId) : undefined;
  },
  prev: () => {
    const n = neighbour(-1);
    return n ? radioPlayer.play(n.station, n.logoPath, n.radioId) : undefined;
  },
};
