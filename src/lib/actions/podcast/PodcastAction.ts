import { get } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import { queueState } from "#lib/stores/queue/queueState.store";
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
 * Écoute un épisode, là où on l'avait laissé. La file devient cet épisode, puis les
 * épisodes non écoutés qui le suivent dans la liste affichée (`suite`).
 */
export async function playEpisode(podcast: PodcastView, episode: EpisodeView, suite: EpisodeView[] = []) {
  const tracks = [episode, ...suite.filter((e) => !e.played && e.id !== episode.id).slice(0, MAX_SUITE)].map((e, i) =>
    toTrack(podcast, e, i),
  );
  try {
    // Position relue en base : celle de la liste affichée peut dater.
    const from = (await invoke<number | null>("podcast_resume_position", { path: episode.path }).catch(() => null)) ?? 0;
    // La file change pour lancer ce morceau-ci : le synchroniseur ne doit pas le lancer une seconde fois.
    playerService.expectExplicitAction();
    await queueState.loadTracks(tracks);
    await playerService.playFile(tracks[0]);
    if (from > 5 && (await playerService.waitUntilReady(15000))) await playerService.seekTo(from);
  } catch (e) {
    console.error("[podcasts] lecture :", e);
  }
}

/** Ce que la file attend pour un ajout : elle attribue elle-même id, profil et place. */
function toQueueInput(podcast: PodcastView, episode: EpisodeView) {
  const { queueId: _, profilId: __, position: ___, ...track } = toTrack(podcast, episode);
  return track;
}

/** « Lire ensuite » : juste après le morceau en cours. */
export const playEpisodeNext = (podcast: PodcastView, episode: EpisodeView) => queueState.addTrack(toQueueInput(podcast, episode));

/** « Ajouter à la file » : à la fin. */
export const enqueueEpisode = (podcast: PodcastView, episode: EpisodeView) => queueState.enqueue(toQueueInput(podcast, episode));

export const downloadEpisode = (id: number) => invoke("podcast_download", { episodeId: id });
export const deleteEpisodeDownload = (id: number) => invoke<number>("podcast_delete_download", { episodeId: id });
