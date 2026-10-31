// Compteurs d'écoutes à jour, par chemin : les listes déjà ouvertes s'en servent sans rechargement.
// Valeurs absolues rendues par le moteur : rien n'est compté deux fois quand la page recharge ses données.
import { writable } from "svelte/store";

export type PlayStats = { play_count: number; last_played_at: string | null };

const stats = writable<Map<string, PlayStats>>(new Map());

export const playStats = {
  subscribe: stats.subscribe,
  set(path: string, s: PlayStats) {
    stats.update((m) => new Map(m).set(path, s));
  },
};

/** Le morceau avec son compteur le plus récent, s'il a été écouté depuis le chargement de la page. */
export function withPlayStats<T extends { path?: string | null; play_count?: number | null; last_played_at?: string | null }>(
  track: T,
  map: Map<string, PlayStats>,
): T {
  const s = track.path ? map.get(track.path) : undefined;
  return s ? { ...track, play_count: s.play_count, last_played_at: s.last_played_at } : track;
}
