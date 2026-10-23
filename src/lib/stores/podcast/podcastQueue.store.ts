import { writable, get } from "svelte/store";
import type { QueueTrack } from "#lib/types/db/queue/QueueTrack";

/**
 * File des épisodes, à part de la file musique (jamais l'une et l'autre en même temps) :
 * active tant qu'un épisode lancé d'ici joue ; relancer de la musique la quitte, la file
 * musique est restée intacte.
 */
type PodcastQueueState = { tracks: QueueTrack[]; index: number; active: boolean };

const writer = writable<PodcastQueueState>({ tracks: [], index: -1, active: false });

export const podcastQueue = {
  subscribe: writer.subscribe,

  isActive: () => get(writer).active,

  /** Nouvelle file : l'épisode lancé, puis sa suite. */
  start(tracks: QueueTrack[]) {
    writer.set({ tracks, index: 0, active: tracks.length > 0 });
  },

  /** L'épisode suivant, devenu courant ; `null` en fin de file. */
  advance(): QueueTrack | null {
    const s = get(writer);
    if (s.index + 1 >= s.tracks.length) return null;
    writer.set({ ...s, index: s.index + 1 });
    return s.tracks[s.index + 1];
  },

  /** L'épisode précédent (le premier reste le premier). */
  back(): QueueTrack | null {
    const s = get(writer);
    if (s.index < 0 || !s.tracks.length) return null;
    const index = Math.max(0, s.index - 1);
    writer.set({ ...s, index });
    return s.tracks[index];
  },

  /** Saute à un épisode de la file. */
  jump(queueId: string): QueueTrack | null {
    const s = get(writer);
    const index = s.tracks.findIndex((t) => t.queueId === queueId);
    if (index < 0) return null;
    writer.set({ ...s, index });
    return s.tracks[index];
  },

  /** « Lire ensuite » : juste après l'épisode en cours. */
  addNext(track: QueueTrack) {
    writer.update((s) => {
      const tracks = s.tracks.filter((t) => t.path !== track.path);
      const at = Math.min(tracks.length, tracks.findIndex((t) => t.queueId === s.tracks[s.index]?.queueId) + 1);
      tracks.splice(at, 0, track);
      return { ...s, tracks, index: tracks.findIndex((t) => t.queueId === s.tracks[s.index]?.queueId) };
    });
  },

  /** « Ajouter à la file » : à la fin. */
  enqueue(track: QueueTrack) {
    writer.update((s) => (s.tracks.some((t) => t.path === track.path) ? s : { ...s, tracks: [...s.tracks, track] }));
  },

  /** La musique reprend la main : la file des épisodes se met de côté. */
  detach() {
    if (get(writer).active) writer.update((s) => ({ ...s, active: false }));
  },
};
