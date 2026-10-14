/**
 * Télécommande d'ampli : l'ampli piloté, ses capacités et son état, poussés par le moteur
 * (`amp-remote-state`) à chaque changement, y compris depuis la télécommande de l'ampli.
 */

import { writable } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { toasts } from "#lib/stores/ui/toast.store";

export type AmpBrand = "onkyo" | "denon" | "yamaha";
export type AmpZone = "main" | "zone2";

export type Amp = { id: string; name: string; brand: AmpBrand; model: string; ip: string; port: number };
export type AmpChoice = { id: string; name: string };
export type AmpZoneState = { power: boolean; volume: number | null; muted: boolean; input: string | null };
export type AmpNetLine = { text: string; kind: string };
export type AmpNet = {
  title: string;
  count: number;
  cursor: number | null;
  lines: AmpNetLine[];
  playback: boolean;
  track: string;
  artist: string;
  album: string;
  time: string;
  status: "play" | "pause" | "stop" | "";
  cover_rev: number;
};
export type AmpNavKey = "up" | "down" | "left" | "right" | "enter" | "back" | "home" | "menu";
export type AmpTransport = "play_pause" | "stop" | "next" | "previous" | "repeat" | "shuffle";

export type AmpView = {
  amp: Amp;
  caps: {
    inputs: AmpChoice[];
    modes: AmpChoice[];
    volume_max: number;
    volume_step: number;
    tone: { min: number; max: number; step: number } | null;
    zone2: boolean;
  };
  state: {
    connected: boolean;
    main: AmpZoneState;
    zone2: AmpZoneState | null;
    mode: AmpChoice | null;
    bass: number | null;
    treble: number | null;
    net: AmpNet;
  };
};

export type AmpCommand =
  | { type: "power"; zone: AmpZone; on: boolean }
  | { type: "volume"; zone: AmpZone; value: number }
  | { type: "volume_step"; zone: AmpZone; up: boolean }
  | { type: "mute"; zone: AmpZone; on: boolean }
  | { type: "input"; zone: AmpZone; id: string }
  | { type: "mode"; id: string }
  | { type: "bass"; value: number }
  | { type: "treble"; value: number }
  | { type: "nav"; key: AmpNavKey }
  | { type: "net_select"; line: number }
  | { type: "transport"; action: AmpTransport };

type State = {
  view: AmpView | null;
  /** Pochette de l'écran réseau (adresse `data:`). */
  cover: string | null;
  found: Amp[];
  searching: boolean;
  searched: boolean;
};

/** Marques prises en charge, pour l'affichage. */
export const AMP_BRANDS: Record<AmpBrand, string> = { onkyo: "Onkyo · Pioneer · Integra", denon: "Denon · Marantz", yamaha: "Yamaha" };

function createStore() {
  const { subscribe, update } = writable<State>({ view: null, cover: null, found: [], searching: false, searched: false });
  let listening = false;

  async function init(): Promise<void> {
    try {
      const view = await invoke<AmpView | null>("amp_remote_status");
      const cover = await invoke<string | null>("amp_remote_cover");
      update((s) => ({ ...s, view, cover }));
    } catch (e) {
      console.error("[télécommande] état :", e);
    }
    if (listening) return;
    listening = true;
    listen<AmpView | null>("amp-remote-state", (e) => update((s) => ({ ...s, view: e.payload })));
    listen<string | null>("amp-remote-cover", (e) => update((s) => ({ ...s, cover: e.payload })));
  }

  async function discover(): Promise<void> {
    update((s) => ({ ...s, searching: true }));
    try {
      const found = await invoke<Amp[]>("amp_remote_discover");
      update((s) => ({ ...s, found, searched: true }));
    } catch (e) {
      console.error("[télécommande] recherche :", e);
    } finally {
      update((s) => ({ ...s, searching: false }));
    }
  }

  async function select(amp: Amp | null): Promise<void> {
    const view = await invoke<AmpView | null>("amp_remote_select", { amp });
    update((s) => ({ ...s, view }));
  }

  /** Commande envoyée à l'ampli ; l'état revient par l'ampli lui-même. */
  async function send(command: AmpCommand): Promise<void> {
    try {
      await invoke("amp_remote_send", { command });
    } catch (e) {
      toasts.push({ type: "error", title: String(e) });
    }
  }

  async function setEnabled(on: boolean): Promise<void> {
    await invoke("set_amp_remote_enabled", { enabled: on });
    const view = on ? await invoke<AmpView | null>("amp_remote_status") : null;
    update((s) => ({ ...s, view, searched: false, found: on ? s.found : [] }));
  }

  return { subscribe, init, discover, select, send, setEnabled };
}

export const ampRemoteStore = createStore();
