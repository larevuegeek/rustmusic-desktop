import { writable, get } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { t } from "#lib/i18n";
import { toasts } from "#lib/stores/ui/toast.store";
import { profilSelector } from "#lib/stores/profil/profil.store";
import { libraryStore } from "#lib/stores/library/library.store";
import { queueState } from "#lib/stores/queue/queueState.store";
import { player } from "#lib/stores/player/player.store";
import { liked } from "#lib/stores/playlist/like.store";
import { playlistStore } from "#lib/stores/playlist/playlist.store";
import { offlineStore } from "#lib/stores/mediaServer/offline.store";
import { recent } from "#lib/stores/recent/recent.store";
import type { DlnaFolder, DlnaFound, MusicFolder, ServerEntry, ServerKind, ServerProbe } from "#lib/types/ui/mediaServer/MediaServer";
import { remoteServerId } from "#lib/types/db/library/Library";

type ServerState = {
  servers: ServerEntry[];
  loaded: boolean;
  profilId: number | null;
  /** Serveur joignable ou non, par id ; absent tant que ce n'est pas vérifié. */
  online: Record<number, boolean>;
};

const serverWriter = writable<ServerState>({ servers: [], loaded: false, profilId: null, online: {} });

const currentProfileId = () => get(profilSelector).profilSelected?.id ?? null;

async function load(profilId = currentProfileId()) {
  if (profilId == null) return;
  try {
    const servers = await invoke<ServerEntry[]>("get_servers", { profilId });
    serverWriter.update((s) => ({ ...s, servers, loaded: true, profilId, online: s.profilId === profilId ? s.online : {} }));
  } catch (e) {
    console.error("[serveurs] chargement :", e);
  }
}

/** Vérifie que les serveurs activés répondent (pastilles verte ou rouge). */
async function check() {
  const profilId = currentProfileId();
  if (profilId == null) return;
  const res = await invoke<[number, boolean][]>("servers_check", { profilId }).catch(() => null);
  if (res && currentProfileId() === profilId) serverWriter.update((s) => ({ ...s, online: Object.fromEntries(res) }));
}

let watching: ReturnType<typeof setInterval> | null = null;

/** Le sélecteur de bibliothèque suit : ajout, retrait, activation. */
async function afterChange() {
  await load();
  await libraryStore.refresh();
  void check();
}

export type ServerCredentials = { kind: ServerKind; url: string; username: string; password: string | null; apiKey: string | null };

export const serverStore = {
  subscribe: serverWriter.subscribe,
  load,
  check,

  /** Au lancement : l'état des serveurs, puis toutes les deux minutes. */
  watch() {
    if (watching) return;
    void listen<string>("server-play-error", (e) =>
      toasts.push({ type: "error", title: get(t)("servers.error_play"), message: e.payload }));
    // Synchronisation d'une distante finie : les favoris venus du serveur.
    void listen<{ library_id: number }>("import-complete", (e) => {
      if (!get(libraryStore).libraries.some((l) => l.id === e.payload.library_id && l.server_id)) return;
      void liked.refresh();
      void playlistStore.refresh();
    });
    void load();
    void check();
    offlineStore.init();
    watching = setInterval(() => void check(), 120_000);
  },

  /** Morceau d'un serveur injoignable ou désactivé, sauf gardé hors ligne : la lecture le saute. */
  unavailable(path: string): boolean {
    const id = remoteServerId(path);
    if (id == null || offlineStore.isReady(path)) return false;
    const s = get(serverWriter);
    return s.online[id] === false || s.servers.some((x) => x.id === id && !x.enabled);
  },

  probe: (c: ServerCredentials) =>
    invoke<ServerProbe>("server_probe", { kind: c.kind, url: c.url, username: c.username, password: c.password, apiKey: c.apiKey }),

  async add(c: ServerCredentials, name: string | null, folderIds: string[]) {
    const profilId = currentProfileId();
    if (profilId == null) throw new Error("aucun profil");
    const s = await invoke<ServerEntry>("server_add", {
      profilId, kind: c.kind, url: c.url, username: c.username, password: c.password, apiKey: c.apiKey, name, folderIds,
    });
    await afterChange();
    return s;
  },

  folders: (serverId: number) => invoke<MusicFolder[]>("server_folders", { serverId }),

  /** Serveurs DLNA du réseau. */
  discover: () => invoke<DlnaFound[]>("server_discover"),
  /** Sous-dossiers d'un serveur DLNA (`0` : la racine). */
  browse: (url: string, objectId: string) => invoke<DlnaFolder[]>("server_browse", { url, objectId }),

  async addLibraries(serverId: number, folderIds: string[]) {
    await invoke("server_add_libraries", { serverId, folderIds });
    await afterChange();
  },

  async setEnabled(serverId: number, enabled: boolean) {
    serverWriter.update((s) => ({ ...s, servers: s.servers.map((x) => (x.id === serverId ? { ...x, enabled } : x)) }));
    await invoke("server_set_enabled", { serverId, enabled });
    await afterChange();
  },

  async setTranscode(serverId: number, format: string | null, bitrate: number | null) {
    await invoke("server_set_transcode", { serverId, format, bitrate });
    await load();
  },

  async setPinPlaylists(serverId: number, pin: boolean) {
    serverWriter.update((s) => ({ ...s, servers: s.servers.map((x) => (x.id === serverId ? { ...x, pin_playlists: pin } : x)) }));
    await invoke("server_set_pin_playlists", { serverId, pin });
    await playlistStore.refresh();
  },

  async rename(serverId: number, name: string) {
    await invoke("server_rename", { serverId, name });
    await afterChange();
  },

  async remove(serverId: number) {
    // Lecture d'un de ses morceaux : arrêtée d'abord (son fichier du cache va être effacé).
    const fromServer = (path: string | null | undefined) => remoteServerId(path) === serverId;
    if (fromServer(get(player).pathFile)) await invoke("stop_play").catch(() => {});
    await invoke("server_remove", { serverId });
    // Ses morceaux quittent aussi la file, le lecteur, les titres aimés et les récents.
    queueState.dropTracks(fromServer);
    if (fromServer(get(player).pathFile)) player.clear();
    await Promise.all([afterChange(), liked.refresh(), recent.refreshRecent()]);
  },
};
