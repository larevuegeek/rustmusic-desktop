import type { AudioFile } from "#lib/types/db/audioFile/AudioFile";
import type { LibraryCacheCreate } from "#lib/types/db/library/LibraryCache";

export function toLibraryCacheCreate(
  selectedPath: string,
  audioFile: AudioFile,
  thumbnailPath?: string | null
): LibraryCacheCreate {
  const tags = audioFile.tags ?? null;

  return {
    path: selectedPath,

    title: tags?.title ?? null,
    artist: tags?.artist ?? null,
    album: tags?.album ?? null,
    album_artist: tags?.album_artist ?? null,

    year: tags?.year ?? null,
    genre: tags?.genre ?? null,
    track_number: tags?.track_number ?? null,
    disc_number: tags?.disc_number ?? null,

    duration: audioFile.duration ?? null,
    bitrate: audioFile.bitrate ?? null,
    bits_per_sample: audioFile.bits_per_sample ?? null,
    sample_rate: audioFile.sample_rate ?? null,

    channels: audioFile.channel ?? null,

    audio_format: audioFile.audio_format ?? null,
    mime_type: audioFile.mime_type ?? null,

    file_size: audioFile.file_size ?? null,

    // JSON sérialisé (si tags absent, on envoie "{}" ou null selon ton choix)
    extra_tags: tags ? JSON.stringify(tags) : JSON.stringify({}),

    thumbnail_path: thumbnailPath ?? null
  };
}