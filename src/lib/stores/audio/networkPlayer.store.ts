/**
 * Lecteurs réseau : recherche et choix de la sortie.
 *
 * UPnP/DLNA (WiiM, Sonos, amplis…) : le moteur Rust confie la lecture à l'appareil.
 * AirPlay : RustMusic décode et lui envoie le son. Dans les deux cas, la file,
 * la position et l'enchaînement fonctionnent comme en local.
 */

import { writable, get } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { player } from "#lib/stores/player/player.store";
import { queueState } from "#lib/stores/queue/queueState.store";
import { playbackPipelineStore } from "#lib/stores/player/playbackPipeline.store";
import { chargerVolume } from "#lib/stores/player/volume.store";
import { toasts } from "#lib/stores/ui/toast.store";
import { t } from "#lib/i18n";

export type NetworkRenderer = {
  /** Identifiant stable d'une session à l'autre (UDN, ou « airplay:… »). */
  id: string;
  name: string;
  manufacturer: string;
  model: string;
  icon_url: string | null;
  ip: string;
  kind: "upnp" | "airplay";
};

/** Icône de l'appareil selon sa famille. */
export function networkIcon(r: Pick<NetworkRenderer, "kind">): string {
  return r.kind === "airplay" ? "material-symbols-light:airplay-rounded" : "material-symbols-light:cast-rounded";
}

type State = {
  renderers: NetworkRenderer[];
  selected: NetworkRenderer | null;
  searching: boolean;
  /** Une recherche a déjà eu lieu : « aucun lecteur » a un sens. */
  searched: boolean;
};

function createStore() {
  const { subscribe, update } = writable<State>({ renderers: [], selected: null, searching: false, searched: false });
  let listening = false;

  async function init(): Promise<void> {
    try {
      const selected = await invoke<NetworkRenderer | null>("network_player_selected");
      update((s) => ({ ...s, selected, renderers: selected && !s.renderers.some((r) => r.id === selected.id) ? [...s.renderers, selected] : s.renderers }));
    } catch (e) {
      console.error("[réseau] lecteur choisi :", e);
    }
    if (listening) return;
    listening = true;
    // Lecteur éteint ou réseau coupé : la lecture s'arrête, l'utilisateur le sait.
    listen<string>("network-player-lost", (e) => {
      toasts.push({ type: "error", title: get(t)("network_player.lost"), message: e.payload });
    });
    listen<string>("network-player-error", (e) => {
      player.update({ status: "ended", isPreparing: false });
      toasts.push({ type: "error", title: get(t)("network_player.error"), message: e.payload });
    });
  }

  async function discover(): Promise<void> {
    update((s) => ({ ...s, searching: true }));
    try {
      const renderers = await invoke<NetworkRenderer[]>("network_players_discover");
      update((s) => ({ ...s, renderers, searched: true }));
    } catch (e) {
      console.error("[réseau] recherche :", e);
    } finally {
      update((s) => ({ ...s, searching: false }));
    }
  }

  /** Change de sortie ; le morceau en cours reprend sur la nouvelle, au même endroit. */
  async function select(id: string | null): Promise<void> {
    await switchOutput(async () => {
      const selected = await invoke<NetworkRenderer | null>("network_player_select", { id });
      update((s) => ({ ...s, selected }));
    });
  }

  /** Réglage d'une famille (UPnP/DLNA ou AirPlay). Coupée pendant qu'elle joue : retour en local. */
  async function setEnabled(kind: NetworkRenderer["kind"], on: boolean): Promise<void> {
    const command = kind === "airplay" ? "set_airplay_enabled" : "set_network_players_enabled";
    if (on) {
      await invoke(command, { enabled: true });
      update((s) => ({ ...s, searched: false }));
      return;
    }
    await switchOutput(async () => {
      await invoke(command, { enabled: false });
      update((s) => ({
        ...s,
        selected: s.selected?.kind === kind ? null : s.selected,
        renderers: s.renderers.filter((r) => r.kind !== kind),
        searched: false,
      }));
    }, get({ subscribe }).selected?.kind === kind);
  }

  /** Bascule de sortie ; si un morceau jouait, il reprend sur la nouvelle au même endroit. */
  async function switchOutput(change: () => Promise<void>, resume = true): Promise<void> {
    const before = get(player);
    const enCours = resume && before.status === "playing" && before.trackId;
    const position = before.jsPosition ?? 0;
    await change();
    // La chaîne locale ne décrit plus la sortie.
    playbackPipelineStore.set(null);
    await chargerVolume();

    if (!enCours) return;
    const qs = get(queueState);
    const track = qs.tracks[qs.currentIndex];
    if (!track) return;
    const { playerService } = await import("#lib/services/player/player.service");
    await playerService.playFile(track);
    if (position > 2 && (await playerService.attendrePret(10000))) await playerService.seekTo(position);
  }

  return { subscribe, init, discover, select, setEnabled };
}

export const networkPlayerStore = createStore();
