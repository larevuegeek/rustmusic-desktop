import { invoke } from "@tauri-apps/api/core";
import { queueState } from "$lib/stores/queue/queueState.store";
import { toasts } from "$lib/stores/ui/toast.store";
import type { TrackListView } from "$lib/types/ui/library/track/TrackListView";
import { get } from "svelte/store";
import { loadTracksByAlbum } from "$lib/services/library/library.service";
import { libraryStore } from "$lib/stores/library/library.store";
import { toQueueTracks } from "$lib/helper/tools/queueTools";
import { playerService } from "$lib/services/player/player.service";
import { echantillonAleatoire } from "$lib/helper/tools/randomTools";
import { t, currentLocale } from "$lib/i18n";

/** Texte au singulier ou au pluriel, nombre au format de la langue. */
function compte(n: number, one: string, many: string): string {
  return get(t)(n === 1 ? one : many).replace("{n}", n.toLocaleString(get(currentLocale)));
}

export async function handleAlbumEnqueue(libraryAlbumId: string) {
  const state = get(libraryStore);
  const libraryId = state.librarySelected?.id;
  if (!libraryId) return;

  try {
    const result = await loadTracksByAlbum(libraryId, libraryAlbumId);
    if (!result || result.length === 0) return;

    const queueTracks = toQueueTracks(result);
    await queueState.loadTracks(queueTracks);
    playerService.playFile(queueTracks[0]);

    toasts.push({
      type: "success",
      title: get(t)("notify.album_playing"),
      message: compte(queueTracks.length, "notify.queue_loaded_one", "notify.queue_loaded_n")
    });
  } catch (e) {
    console.error("Impossible de charger l'album", e);
  }
}

/** Ajoute tout l'album à la fin de la file, sans interrompre la lecture. */
export async function handleAlbumAddToQueue(libraryAlbumId: string) {
  const libraryId = get(libraryStore).librarySelected?.id;
  if (!libraryId) return;
  try {
    const pistes = await loadTracksByAlbum(libraryId, libraryAlbumId);
    if (!pistes?.length) return;
    for (const qt of toQueueTracks(pistes)) await queueState.enqueue(qt);
    toasts.push({ type: "success", title: get(t)("notify.queued"), message: compte(pistes.length, "notify.queued_one", "notify.queued_n") });
  } catch (e) {
    console.error("Impossible d'ajouter l'album à la file", e);
  }
}

/** Lit tous les titres d'un artiste (remplace la file). */
export async function handleArtistPlay(libraryId: number, artistId: string) {
  try {
    const pistes = await invoke<TrackListView[]>("get_tracks_by_artist", { libraryId, artistId });
    if (!pistes?.length) return;
    const file = toQueueTracks(pistes);
    await queueState.loadTracks(file);
    playerService.playFile(file[0]);
  } catch (e) {
    console.error("Impossible de lire l'artiste", e);
  }
}

/** Ajoute tous les titres d'un artiste à la fin de la file. */
export async function handleArtistAddToQueue(libraryId: number, artistId: string) {
  try {
    const pistes = await invoke<TrackListView[]>("get_tracks_by_artist", { libraryId, artistId });
    if (!pistes?.length) return;
    for (const qt of toQueueTracks(pistes)) await queueState.enqueue(qt);
    toasts.push({ type: "success", title: get(t)("notify.queued"), message: compte(pistes.length, "notify.queued_one", "notify.queued_n") });
  } catch (e) {
    console.error("Impossible d'ajouter l'artiste à la file", e);
  }
}

/** Lit un genre, dans l'ordre de la bibliothèque ou mélangé. */
export async function handleGenrePlay(libraryId: number, genre: string, melanger = false) {
  try {
    const pistes = await invoke<TrackListView[]>("get_tracks_by_genre", { libraryId, genre });
    if (!pistes?.length) return;
    const ordre = melanger ? [...pistes].sort(() => Math.random() - 0.5) : pistes;
    const file = toQueueTracks(ordre);
    await queueState.loadTracks(file);
    playerService.playFile(file[0]);
  } catch (e) {
    console.error("Impossible de lire le genre", e);
  }
}

/** Lit des titres déjà chargés (une page d'album), dans l'ordre ou mélangés. */
export async function handleTracksPlay(pistes: TrackListView[], melanger = false) {
  if (!pistes.length) return;
  try {
    const file = toQueueTracks(melanger ? echantillonAleatoire(pistes, pistes.length) : pistes);
    await queueState.loadTracks(file);
    playerService.playFile(file[0]);
  } catch (e) {
    console.error("Impossible de lancer la lecture", e);
  }
}

async function tirerEtJouer(args: Record<string, unknown>) {
  try {
    await handleTracksPlay((await invoke<TrackListView[]>("get_mix_tracks", { ...args, limit: 100 })) ?? []);
  } catch (e) {
    console.error("Impossible de lancer le mix", e);
  }
}

/** Un mix d'époque : une décennie entière, ou une seule année. */
export function handlePeriodeMix(libraryId: number, decennie: number, annee: number | null = null) {
  return tirerEtJouer(annee == null ? { libraryId, kind: "decennie", decennie } : { libraryId, kind: "annee", annee });
}

/** Cent titres tirés au hasard dans la bibliothèque. */
export function handleRandomMix(libraryId: number) {
  return tirerEtJouer({ libraryId, kind: "hasard" });
}
