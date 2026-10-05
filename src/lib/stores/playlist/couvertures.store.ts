import { derived, get } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import { playlistStore } from "./playlist.store";
import { likedCount } from "./like.store";
import { profilSelector } from "#lib/stores/profil/profil.store";
import { recentCount } from "#lib/stores/recent/recent.store";

// Relu quand une playlist, les titres aimés ou les récents changent.
const empreinte = derived([profilSelector, playlistStore, likedCount, recentCount], ([$p, $pl, $l, $r]) =>
  [$p.profilSelected?.id ?? "", $pl.playlists.map((p) => `${p.id}:${p.track_count}:${p.updated_at}`).join(","), $l, $r].join("|"));

/** Pochettes des playlists, de Titres aimés et de Récents : un seul chargement pour la barre, l'accueil et la page. */
export const couverturesPlaylists = derived(empreinte, (_, set: (v: Record<string, string[]>) => void) => {
  const profilId = get(profilSelector).profilSelected?.id;
  if (profilId == null) return;
  let perime = false;
  invoke<{ key: string; covers: string[] }[]>("get_sidebar_covers", { profilId })
    .then((l) => { if (!perime) set(Object.fromEntries(l.map((c) => [c.key, c.covers]))); })
    .catch((e) => console.error("Pochettes des playlists :", e));
  return () => { perime = true; };
}, {} as Record<string, string[]>);

/** Les playlists qui reçoivent des titres : une auto (ou un mix) calcule son contenu. */
export const playlistsRangees = derived(playlistStore, ($pl) => $pl.playlists.filter((p) => !p.is_smart));
