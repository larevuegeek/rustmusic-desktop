/**
 * Récupération de métadonnées depuis une source en ligne.
 * Miroir de `src-tauri/src/commands/metadata_command.rs`.
 *
 * Ces appels **n'écrivent rien**. Ils proposent : l'appariement rend un score
 * et un niveau de confiance par piste, et c'est l'interface qui décide, champ
 * par champ, ce qui est appliqué.
 */

import { invoke } from "@tauri-apps/api/core";

/** Un album trouvé par la recherche. */
export type AlbumHit = {
  id: number;
  title: string;
  artist: string;
  cover: string | null;
  /** Nombre de pistes annoncé — le premier indice qu'on tient le bon album. */
  track_count: number;
};

export type AlbumTrack = {
  position: number;
  disc: number;
  title: string;
  artist: string;
  /** En secondes. */
  duration: number;
};

export type AlbumDetail = {
  id: number;
  title: string;
  artist: string;
  year: string | null;
  /** Genre au niveau de l'album — Deezer n'en donne pas par piste. */
  genre: string | null;
  cover: string | null;
  tracks: AlbumTrack[];
};

/** À quel point on peut se fier à un appariement. */
export type Confidence = "sure" | "doubtful" | "rejected";

export type TrackMatch = {
  local_index: number;
  /** Position de la piste distante retenue, `null` si aucune ne convient. */
  remote_position: number | null;
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
  id: number;
  title: string;
  artist: string;
  album: string;
  cover: string | null;
  /** En secondes — le meilleur discriminant entre deux versions d'un titre. */
  duration: number;
};

/**
 * Tout ce que Deezer sait d'une piste, prêt à remplir un formulaire.
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
  cover: string | null;
};

export async function searchAlbums(query: string, limit = 8): Promise<AlbumHit[]> {
  return invoke<AlbumHit[]>("metadata_search_albums", { query, limit });
}

/**
 * Cherche des pistes.
 *
 * Le pendant à l'unité de `searchAlbums` : pour corriger **un** fichier,
 * désigner son album puis y retrouver sa piste fait deux décisions là où le
 * titre suffit.
 */
export async function searchTracks(query: string, limit = 8): Promise<TrackHit[]> {
  return invoke<TrackHit[]>("metadata_search_tracks", { query, limit });
}

export async function trackValues(trackId: number): Promise<TrackValues> {
  return invoke<TrackValues>("metadata_track_values", { trackId });
}

/**
 * Récupère un album et l'apparie au jeu de travail.
 *
 * Les deux vont ensemble : proposer un album sans dire quelle piste correspond
 * à quel fichier laisserait tout le travail d'identification à l'utilisateur.
 */
export async function matchAlbum(
  albumId: number,
  tracks: LocalTrackPayload[],
): Promise<MatchProposal> {
  return invoke<MatchProposal>("metadata_match_album", { albumId, tracks });
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
] as const;

export type SourceField = (typeof SOURCE_FIELDS)[number];

/**
 * Ce que la source propose pour une piste appariée.
 *
 * Le compositeur et le parolier n'y figurent pas : Deezer ne les donne pas.
 * Mieux vaut ne rien proposer que proposer du vide qui effacerait l'existant.
 */
export function proposedValues(
  album: AlbumDetail,
  track: AlbumTrack,
): Partial<Record<SourceField, string>> {
  return {
    title: track.title,
    artist: track.artist,
    album: album.title,
    album_artist: album.artist,
    year: album.year ?? "",
    genre: album.genre ?? "",
    track_number: String(track.position),
    total_tracks: String(album.tracks.length),
  };
}

/** Durée en minutes:secondes, pour comparer d'un coup d'œil. */
export function formatDuration(seconds: number): string {
  const m = Math.floor(seconds / 60);
  const s = seconds % 60;
  return `${m}:${String(s).padStart(2, "0")}`;
}
