// Source affichée : « Cet ordinateur » (0), un serveur (son id) ou toutes les sources.
import { derived, get } from "svelte/store";
import { settingsStore } from "#lib/stores/settings/settings.store";
import { serverStore } from "#lib/stores/mediaServer/mediaServer.store";
import { libraryStore } from "#lib/stores/library/library.store";
import { liked } from "#lib/stores/playlist/like.store";
import { playlistStore } from "#lib/stores/playlist/playlist.store";
import { recent } from "#lib/stores/recent/recent.store";
import { sourceFilter as filtre } from "./sourceFilter.store";
import type { Library } from "#lib/types/db/library/Library";

export const LOCAL = 0;
export type SourceKey = number | "all";

const parse = (v: string | undefined): SourceKey => (v === "all" ? "all" : Number.isInteger(Number(v)) ? Number(v) : LOCAL);

/** Serveurs activés : sans eux, il n'y a pas de choix de source. */
export const activeServers = derived(serverStore, (s) => s.servers.filter((x) => x.enabled));

/** La source choisie ; celle d'un serveur retiré ou désactivé redevient « Cet ordinateur ». */
export const currentSource = derived([settingsStore, serverStore], ([$s, $srv]): SourceKey => {
  const k = parse($s.current_source);
  // Serveurs pas encore lus : on garde le choix tel quel.
  if (!$srv.loaded || k === LOCAL) return k;
  const actifs = $srv.servers.filter((x) => x.enabled);
  if (k === "all") return actifs.length ? "all" : LOCAL;
  return actifs.some((x) => x.id === k) ? k : LOCAL;
});

/** Filtre des commandes : `null` = toutes (aussi sans serveur : rien à séparer). */
export const sourceFilter = derived([currentSource, activeServers], ([$c, $a]): number | null =>
  $c === "all" || $a.length === 0 ? null : $c,
);

// Recopié pour les magasins, qui se rechargent quand il change (pas au premier passage : ils s'initialisent seuls).
let premier = true;
sourceFilter.subscribe((v) => {
  if (v === get(filtre) && !premier) return;
  filtre.set(v);
  if (premier) {
    premier = false;
    return;
  }
  void liked.refresh();
  void playlistStore.refresh();
  void recent.refreshRecent();
});

export const sourceOfLibrary = (l: Pick<Library, "server_id">): number => l.server_id ?? LOCAL;

/** Bibliothèques de la source affichée. */
export const sourceLibraries = derived([libraryStore, currentSource], ([$l, $c]) =>
  $c === "all" ? $l.libraries : $l.libraries.filter((l) => sourceOfLibrary(l) === $c),
);

/** Garde la bibliothèque ouverte dans la source : sinon, celle par défaut de la source (ou la première). */
export function keepLibraryInSource() {
  const libs = get(sourceLibraries);
  const sel = get(libraryStore).librarySelected;
  if (libs.length && (!sel || !libs.some((l) => l.id === sel.id))) {
    libraryStore.selectLibrary(libs.find((l) => l.is_default) ?? libs[0]);
  }
}

export async function selectSource(key: SourceKey) {
  await settingsStore.set("current_source", String(key));
  keepLibraryInSource();
}
