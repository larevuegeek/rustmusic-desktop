import { writable } from "svelte/store";
import type { Player } from "#lib/types/db/player/Player";
import { queueState } from "../queue/queueState.store";

const initial_state: Player =  {
  pathFile: null,
  audioFile: null,
  duration: 0,
  status: "ready",
  rustPosition: 0,
  jsPosition: 0,
  lastUpdateTime: 0,
  trackId: null,
  isPreparing: false,
}

const playerWritable = writable<Player>(initial_state);

export const player = {
  subscribe: playerWritable.subscribe,

  setPlayer: (p: Player) => playerWritable.set(p),
  clearTrack: (trackId: string) => {
    
    playerWritable.update((prev) => (prev ? {
      ...prev,
      ...initial_state,
    } : prev));

    queueState.removeTrack(trackId);

  },
  reset: () =>
    playerWritable.update((prev) => (prev ? { ...prev, rustPosition: 0, jsPosition: 0, status: "ended" } : prev)),
  update: (updates: Partial<Player>) =>
    playerWritable.update((prev) => (prev ? { ...prev, ...updates } : prev)),
  clear: () => playerWritable.set(initial_state),
};