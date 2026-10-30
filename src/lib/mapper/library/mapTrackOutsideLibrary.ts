import { displayTitle, fileStem } from "#lib/helper/tools/stringTools";
import type { TrackListView } from "#lib/types/ui/library/track/TrackListView";

/** Ce que l'historique ou les J'aime savent d'un morceau, par son cache de tags. */
type Known = {
  path: string;
  title?: string | null;
  artist?: string | null;
  album?: string | null;
  album_artist?: string | null;
  year?: string | null;
  genre?: string | null;
  track_number?: number | null;
  disc_number?: number | null;
  duration?: number | null;
  bitrate?: number | null;
  bits_per_sample?: number | null;
  sample_rate?: number | null;
  channels?: number | null;
  audio_format?: string | null;
  mime_type?: string | null;
  file_size?: number | null;
  thumbnail_path?: string | null;
  play_count?: number | null;
  last_played_at?: string | null;
};

const OUTSIDE = "path:";

/** Ligne fabriquée par `mapTrackOutsideLibrary` : pas de piste en bibliothèque derrière. */
export const isOutsideLibrary = (id: string | number | null | undefined) => typeof id === "string" && id.startsWith(OUTSIDE);

/**
 * Ligne de tableau pour un morceau absent de la bibliothèque (fichier ouvert hors bibliothèque,
 * retiré ou déplacé depuis) : l'accueil le montre, la page « Voir tout » doit le montrer aussi.
 * Sans piste en bibliothèque : ni album ni artiste à ouvrir, `id` tiré du chemin.
 */
export function mapTrackOutsideLibrary(k: Known): TrackListView {
  const filename = k.path.replace(/\\/g, "/").split("/").pop() ?? k.path;
  const dot = filename.lastIndexOf(".");
  return {
    id: `${OUTSIDE}${k.path}`,
    path: k.path,
    filename,
    extension: dot > 0 ? filename.slice(dot + 1).toLowerCase() : "",
    size: k.file_size ?? 0,
    status: "ok",
    is_available: true,
    error_message: null,
    title: displayTitle(k.title, k.path, fileStem(k.path)),
    title_normalized: (k.title ?? fileStem(k.path)).toLowerCase(),
    artist_id: null,
    library_artist_id: null,
    album_id: null,
    artist: k.artist ?? null,
    album: k.album ?? null,
    album_artist: k.album_artist ?? null,
    year: k.year ?? null,
    genre: k.genre ?? null,
    track_number: k.track_number ?? null,
    disc_number: k.disc_number ?? 1,
    duration: k.duration ?? null,
    bitrate: k.bitrate ?? null,
    bits_per_sample: k.bits_per_sample ?? null,
    sample_rate: k.sample_rate ?? null,
    channels: k.channels ?? null,
    audio_format: k.audio_format ?? null,
    mime_type: k.mime_type ?? null,
    file_size: k.file_size ?? null,
    extra_tags: null,
    tags: null,
    thumbnail_path: k.thumbnail_path ?? null,
    last_scanned_at: null,
    play_count: k.play_count ?? 0,
    last_played_at: k.last_played_at ?? null,
    rating: null,
    favorite: false,
    created_at: "",
    updated_at: "",
  };
}
