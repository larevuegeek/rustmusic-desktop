import { readable } from "svelte/store";
import { player } from "./player.store";
import type { PlayerStatus } from "#lib/types/db/player/Player";

export type Lecture = { path: string | null; status: PlayerStatus };

// Le lecteur publie sa position plusieurs fois par seconde ; les listes n'ont besoin
// que du morceau et de l'état : ce store ne prévient que quand l'un d'eux change.
export const lecture = readable<Lecture>({ path: null, status: "ready" }, (set) => {
  let dernier: Lecture | null = null;
  return player.subscribe((p) => {
    const path = p?.pathFile ?? null;
    const status = p?.status ?? "ready";
    if (dernier && dernier.path === path && dernier.status === status) return;
    set((dernier = { path, status }));
  });
});
