import { get } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import { podcastQueue } from "#lib/stores/podcast/podcastQueue.store";
import { profilSelector } from "#lib/stores/profil/profil.store";
import { playerService } from "#lib/services/player/player.service";
import type { QueueTrack } from "#lib/types/db/queue/QueueTrack";
import type { EpisodeView, PodcastView } from "#lib/types/ui/podcast/Podcast";

/** Épisodes non écoutés mis à la suite de celui qu'on lance. */
const MAX_SUITE = 50;

function toTrack(podcast: PodcastView, episode: EpisodeView, position = 0): QueueTrack {
  return {
    queueId: crypto.randomUUID(),
    profilId: get(profilSelector).profilSelected?.id ?? 1,
    path: episode.path,
    title: episode.title,
    artist: podcast.title,
    duration: episode.duration ?? undefined,
    cover: podcast.cover_path ?? undefined,
    position,
  };
}

/**
 * Écoute un épisode, là où on l'avait laissé, dans la file des podcasts (la file musique
 * reste intacte) : cet épisode, puis les non écoutés qui le suivent dans la liste (`suite`).
 */
export async function playEpisode(podcast: PodcastView, episode: EpisodeView, suite: EpisodeView[] = []) {
  const tracks = [episode, ...suite.filter((e) => !e.played && e.id !== episode.id).slice(0, MAX_SUITE)].map((e, i) =>
    toTrack(podcast, e, i),
  );
  try {
    podcastQueue.start(tracks);
    await playerService.playFile(tracks[0], true);
  } catch (e) {
    console.error("[podcasts] lecture :", e);
  }
}

/** « Lire ensuite » : juste après l'épisode en cours (file des podcasts). */
export const playEpisodeNext = (podcast: PodcastView, episode: EpisodeView) => podcastQueue.addNext(toTrack(podcast, episode));

/** « Ajouter à la file » : à la fin de la file des podcasts. */
export const enqueueEpisode = (podcast: PodcastView, episode: EpisodeView) => podcastQueue.enqueue(toTrack(podcast, episode));

export const downloadEpisode = (id: number) => invoke("podcast_download", { episodeId: id });
export const deleteEpisodeDownload = (id: number) => invoke<number>("podcast_delete_download", { episodeId: id });
