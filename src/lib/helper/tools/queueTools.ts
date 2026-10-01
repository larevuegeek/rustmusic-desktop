import type { QueueTrack } from "$lib/types/db/queue/QueueTrack";
import { profilSelector } from "$lib/stores/profil/profil.store";
import { get } from "svelte/store";
import { t } from "$lib/i18n";

/**
 * Interface minimale qu'un objet track doit satisfaire
 * pour être converti en QueueTrack.
 * Tous les types de tracks (TrackListView, PlaylistTrackView,
 * RecentFile, TrackLikedView) peuvent être adaptés à cette interface.
 */
export type TrackLike = {
  /** Chaîne : id de piste de bibliothèque ; nombre : id d'historique ou de favori. */
  id?: string | number | null;
  path: string | null;
  title?: string | null;
  artist?: string | null;
  artist_name?: string | null;
  duration?: number | null;
  thumbnail_path?: string | null;
  cover?: string | null;
};

/**
 * Convertit n'importe quel objet "track-like" en QueueTrack.
 * Gère automatiquement :
 * - Le profil courant
 * - La cover (chemin brut, résolu async via coverHelper)
 * - Les valeurs null/undefined
 */
export function toQueueTrack(track: TrackLike): QueueTrack {
  const profil = get(profilSelector);
  const profilId = profil.profilSelected?.id ?? 1;

  const cover = track.thumbnail_path ?? track.cover ?? "/images/no-cd.png";

  return {
    queueId: crypto.randomUUID(),
    profilId,
    path: track.path ?? "",
    title: track.title ?? get(t)("common.unknown_title"),
    artist: track.artist ?? track.artist_name ?? get(t)("common.unknown_artist"),
    duration: track.duration ?? 0,
    cover,
    position: 0,
  };
}

/**
 * Convertit un tableau de tracks en QueueTracks.
 */
export function toQueueTracks(tracks: TrackLike[]): QueueTrack[] {
  return tracks.map(toQueueTrack);
}
