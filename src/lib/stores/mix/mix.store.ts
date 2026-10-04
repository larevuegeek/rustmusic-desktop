import { invoke } from "@tauri-apps/api/core";
import { writable } from "svelte/store";
import type { TrackListView } from "$lib/types/ui/library/track/TrackListView";
import { dureeEcoute } from "$lib/helper/tools/dateTools";

/** D'où vient un mix : un tirage de la bibliothèque, ou un mix créé (playlist auto « mix »). */
export type SourceMix =
  | { kind: "genre"; genre: string }
  | { kind: "decennie"; decennie: number }
  | { kind: "oublies" }
  | { kind: "hires" }
  | { playlistId: number };

export const TAILLE_MIX = 50;

// Un mix « du jour » ne se retire pas à chaque retour : l'accueil et la page Mix
// montrent le même tirage jusqu'au lendemain (ou jusqu'à « Nouveau tirage »).
const cache = new Map<string, Promise<TrackListView[]>>();

/** Change à chaque nouveau tirage : les cartes concernées se rechargent. */
export const generationMix = writable(0);

const jour = () => new Date().toDateString();

export function tirer(libraryId: number, cle: string, source: SourceMix): Promise<TrackListView[]> {
  const j = jour();
  // Les tirages d'hier ne resserviront plus.
  for (const k of [...cache.keys()]) if (!k.endsWith(`|${j}`)) cache.delete(k);
  const k = `${libraryId}|${cle}|${j}`;
  let tirage = cache.get(k);
  if (!tirage) {
    tirage = ("playlistId" in source
      ? invoke<TrackListView[]>("get_playlist_tracks_view", { playlistId: source.playlistId })
      : invoke<TrackListView[]>("get_mix_tracks", { libraryId, ...source, limit: TAILLE_MIX })
    ).catch(() => []);
    cache.set(k, tirage);
  }
  return tirage;
}

/** Oublie le tirage d'un mix dans toutes les bibliothèques (ses règles ont changé). */
export function oublier(cle: string) {
  for (const k of [...cache.keys()]) if (k.split("|")[1] === cle) cache.delete(k);
  generationMix.update((n) => n + 1);
}

/** Nouveau tirage : d'un mix (`cle`), ou de tous ceux de la bibliothèque. */
export function renouveler(libraryId: number, cle?: string) {
  const prefixe = cle ? `${libraryId}|${cle}|` : `${libraryId}|`;
  for (const k of [...cache.keys()]) if (k.startsWith(prefixe)) cache.delete(k);
  generationMix.update((n) => n + 1);
}

export type Apercu = { pochettes: string[]; description: string; meta: string };

/** Quatre pochettes, les artistes phares, le nombre de titres et la durée. */
export function apercu(liste: TrackListView[], t: (cle: string) => string): Apercu {
  const pochettes = [...new Set(liste.map((p) => p.thumbnail_path).filter(Boolean) as string[])].slice(0, 4);

  const parArtiste = new Map<string, number>();
  for (const p of liste) if (p.artist) parArtiste.set(p.artist, (parArtiste.get(p.artist) ?? 0) + 1);
  const tete = [...parArtiste.entries()].sort((a, b) => b[1] - a[1]).map(([a]) => a);
  const noms = tete.slice(0, 3).join(", ");
  const description = tete.length > 3 ? t("home.mix_artists_more").replace("{a}", noms) : noms;

  const secondes = liste.reduce((s, p) => s + (p.duration ?? 0), 0);
  const meta = t("home.mix_meta").replace("{n}", String(liste.length)).replace("{d}", dureeEcoute(secondes));

  return { pochettes, description, meta };
}
