import { writable, get } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { profilSelector } from "#lib/stores/profil/profil.store";
import { toasts } from "#lib/stores/ui/toast.store";
import { t } from "#lib/i18n";
import type { DirectoryResult, LatestEpisode, PodcastPreview, PodcastView } from "#lib/types/ui/podcast/Podcast";

type PodcastState = {
  podcasts: PodcastView[];
  loaded: boolean;
  /** Profil des abonnements affichés. */
  profilId: number | null;
};

const podcastWriter = writable<PodcastState>({ podcasts: [], loaded: false, profilId: null });

const currentProfileId = () => get(profilSelector).profilSelected?.id ?? null;

let ecoute: Promise<unknown> | null = null;
// Seule la dernière demande s'applique (changement de profil rapide).
let loadSeq = 0;
let inflight: { profilId: number; promise: Promise<void> } | null = null;

async function load(profilId = currentProfileId()) {
  if (profilId == null) return;
  const seq = ++loadSeq;
  // La vérification de fond prévient quand des épisodes paraissent ; la lecture, quand un épisode ne vient pas.
  ecoute ??= Promise.all([
    listen("podcasts-updated", () => load()),
    listen<string>("podcast-error", (e) => toasts.push({ type: "error", title: get(t)("podcasts.error_play"), message: e.payload })),
  ]);
  try {
    const podcasts = await invoke<PodcastView[]>("get_podcasts", { profilId });
    if (seq === loadSeq) podcastWriter.set({ podcasts, loaded: true, profilId });
  } catch (e) {
    console.error("[podcasts] chargement :", e);
  }
}

/** Charge les abonnements du profil s'ils ne sont pas déjà là (ni en route). */
function ensureLoaded(profilId: number) {
  if (get(podcastWriter).profilId === profilId || inflight?.profilId === profilId) return;
  const promise = load(profilId).finally(() => {
    if (inflight?.promise === promise) inflight = null;
  });
  inflight = { profilId, promise };
}

/** Remplace un podcast dans la liste après une action sur lui. */
async function reloadOne(id: number) {
  const p = await invoke<PodcastView | null>("get_podcast", { podcastId: id });
  podcastWriter.update((s) => ({
    ...s,
    podcasts: p ? s.podcasts.map((x) => (x.id === id ? p : x)) : s.podcasts.filter((x) => x.id !== id),
  }));
  return p;
}

export const podcastStore = {
  subscribe: podcastWriter.subscribe,
  load,
  ensureLoaded,
  reloadOne,

  search: (term: string, country: string) => invoke<DirectoryResult[]>("podcast_search", { term, country }),

  /** Les plus écoutés du moment ; `genre` : catégorie Apple. */
  top: (country: string, genre: number | null) => invoke<DirectoryResult[]>("podcast_top", { country, genre }),

  /** Épisodes à écouter du profil courant : commencés, puis nouveaux. */
  latest: (limit = 20) => {
    const profilId = currentProfileId();
    return profilId == null ? Promise.resolve([]) : invoke<LatestEpisode[]>("get_podcast_latest", { profilId, limit });
  },

  /** Abonne le profil courant ; renvoie le podcast (déjà suivi : le même). */
  async follow(url: string): Promise<PodcastView> {
    const profilId = currentProfileId();
    if (profilId == null) throw new Error("aucun profil");
    const p = await invoke<PodcastView>("podcast_subscribe", { profilId, url });
    podcastWriter.update((s) => ({ ...s, podcasts: [p, ...s.podcasts.filter((x) => x.id !== p.id)] }));
    return p;
  },

  /** Favori : épinglé dans la barre latérale et en tête de « Mes podcasts ». */
  async setPinned(id: number, pinned: boolean) {
    podcastWriter.update((s) => ({ ...s, podcasts: s.podcasts.map((p) => (p.id === id ? { ...p, pinned } : p)) }));
    try {
      await invoke("set_podcast_pinned", { podcastId: id, pinned });
    } catch (e) {
      podcastWriter.update((s) => ({ ...s, podcasts: s.podcasts.map((p) => (p.id === id ? { ...p, pinned: !pinned } : p)) }));
      throw e;
    }
  },

  preview: (url: string) => invoke<PodcastPreview>("podcast_preview", { url }),

  async unfollow(id: number) {
    await invoke("podcast_unsubscribe", { podcastId: id });
    podcastWriter.update((s) => ({ ...s, podcasts: s.podcasts.filter((x) => x.id !== id) }));
  },

  /** Relit un flux ; renvoie le nombre d'épisodes parus. */
  async refresh(id: number): Promise<number> {
    const added = await invoke<number>("podcast_refresh", { podcastId: id });
    await reloadOne(id);
    return added;
  },

  async refreshAll(): Promise<number> {
    const added = await invoke<number>("podcast_refresh", { podcastId: null });
    await load();
    return added;
  },
};
