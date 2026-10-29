/**
 * Récupération de métadonnées depuis une source en ligne (Deezer ou MusicBrainz).
 * Miroir de `src-tauri/src/commands/tags/metadata_command.rs`.
 *
 * Ces appels **n'écrivent rien**. Ils proposent : l'appariement rend un score
 * et un niveau de confiance par piste, et c'est l'interface qui décide, champ
 * par champ, ce qui est appliqué.
 */

import { invoke } from "@tauri-apps/api/core";

/** Base interrogée. */
export type MetadataSource = "deezer" | "musicbrainz";

/** Un album trouvé par la recherche. */
export type AlbumHit = {
  /** Propre à la source : nombre Deezer, UUID MusicBrainz. */
  id: string;
  title: string;
  artist: string;
  cover: string | null;
  /** Nombre de pistes annoncé — le premier indice qu'on tient le bon album. */
  track_count: number;
  /** Ce qui distingue deux éditions : « 1979 · GB · 2×CD ». */
  detail: string | null;
};

export type AlbumTrack = {
  /** Rang dans tout l'album : unique même quand les disques se suivent. */
  key: number;
  /** Numéro sur son disque. */
  position: number;
  disc: number;
  title: string;
  artist: string;
  /** En secondes. */
  duration: number;
  /** Compositeurs, quand la source les connaît (MusicBrainz). */
  composer: string | null;
  id: string;
};

export type AlbumDetail = {
  id: string;
  title: string;
  artist: string;
  year: string | null;
  /** Genre au niveau de l'album. */
  genre: string | null;
  cover: string | null;
  tracks: AlbumTrack[];
};

/** À quel point on peut se fier à un appariement. */
export type Confidence = "sure" | "doubtful" | "rejected";

export type TrackMatch = {
  local_index: number;
  /** Clé de la piste distante retenue, `null` si aucune ne convient. */
  remote_key: number | null;
  /** Entre 0 et 1. */
  score: number;
  confidence: Confidence;
};

export type MatchProposal = {
  album: AlbumDetail;
  /** Un résultat par piste locale, dans l'ordre envoyé. */
  matches: TrackMatch[];
};

/** Une piste locale telle que l'atelier la décrit. */
export type LocalTrackPayload = {
  index: number;
  title: string;
  track_number: number | null;
  duration: number | null;
};

/** Une piste trouvée par la recherche. */
export type TrackHit = {
  id: string;
  title: string;
  artist: string;
  album: string;
  cover: string | null;
  /** En secondes — le meilleur discriminant entre deux versions d'un titre. */
  duration: number;
};

/**
 * Tout ce que la source sait d'une piste, prêt à remplir un formulaire.
 *
 * Les champs absents valent `null` et non `""` : proposer une chaîne vide
 * reviendrait à proposer d'effacer ce qui est déjà écrit dans le fichier.
 */
export type TrackValues = {
  title: string;
  artist: string;
  album: string;
  album_artist: string | null;
  year: string | null;
  genre: string | null;
  track_number: number | null;
  total_tracks: number | null;
  disc_number: number | null;
  composer: string | null;
  cover: string | null;
};

export async function searchAlbums(source: MetadataSource, query: string, limit = 8): Promise<AlbumHit[]> {
  return invoke<AlbumHit[]>("metadata_search_albums", { source, query, limit });
}

/**
 * Cherche des pistes.
 *
 * Le pendant à l'unité de `searchAlbums` : pour corriger **un** fichier,
 * désigner son album puis y retrouver sa piste fait deux décisions là où le
 * titre suffit.
 */
export async function searchTracks(source: MetadataSource, query: string, limit = 8): Promise<TrackHit[]> {
  return invoke<TrackHit[]>("metadata_search_tracks", { source, query, limit });
}

export async function trackValues(source: MetadataSource, trackId: string): Promise<TrackValues> {
  return invoke<TrackValues>("metadata_track_values", { source, trackId });
}

/**
 * Récupère un album et l'apparie au jeu de travail.
 *
 * Les deux vont ensemble : proposer un album sans dire quelle piste correspond
 * à quel fichier laisserait tout le travail d'identification à l'utilisateur.
 */
export async function matchAlbum(
  source: MetadataSource,
  albumId: string,
  tracks: LocalTrackPayload[],
): Promise<MatchProposal> {
  return invoke<MatchProposal>("metadata_match_album", { source, albumId, tracks });
}

/** Les champs que la source sait renseigner. */
export const SOURCE_FIELDS = [
  "title",
  "artist",
  "album",
  "album_artist",
  "year",
  "genre",
  "track_number",
  "total_tracks",
  "disc_number",
  "composer",
] as const;

export type SourceField = (typeof SOURCE_FIELDS)[number];

/**
 * Ce que la source propose pour une piste appariée.
 *
 * Un champ que la source ignore (compositeur chez Deezer) reste vide, donc
 * « non fourni » : mieux vaut ne rien proposer que du vide qui effacerait l'existant.
 */
export function proposedValues(
  album: AlbumDetail,
  track: AlbumTrack,
): Partial<Record<SourceField, string>> {
  // Total et numéro de disque comptés par disque, comme l'écrivent les lecteurs.
  const discs = new Set(album.tracks.map((tk) => tk.disc)).size;
  return {
    title: track.title,
    artist: track.artist,
    album: album.title,
    album_artist: album.artist,
    year: album.year ?? "",
    genre: album.genre ?? "",
    track_number: String(track.position),
    total_tracks: String(album.tracks.filter((tk) => tk.disc === track.disc).length),
    // Un seul disque : rien à dire, sinon « 1 » serait proposé partout.
    disc_number: discs > 1 ? String(track.disc) : "",
    composer: track.composer ?? "",
  };
}

/** Durée en minutes:secondes, pour comparer d'un coup d'œil. */
export function formatDuration(seconds: number): string {
  const m = Math.floor(seconds / 60);
  const s = seconds % 60;
  return `${m}:${String(s).padStart(2, "0")}`;
}
