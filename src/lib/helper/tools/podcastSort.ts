// Tri des podcasts suivis : partagé par l'accueil et « Mes podcasts », retenu d'une visite à l'autre.
import { sortKey, comparerNaturel } from "#lib/helper/library/sortKey";
import { readLocal, writeLocal } from "#lib/helper/tools/storage";
import type { DirectoryResult, PodcastView } from "#lib/types/ui/podcast/Podcast";

export type PodcastSort = "recent" | "name" | "added" | "new";
const KEY = "podcasts:tri";
const SORTS: PodcastSort[] = ["recent", "name", "added", "new"];

export function readPodcastSort(): PodcastSort {
  const v = readLocal(KEY, "recent");
  return SORTS.includes(v as PodcastSort) ? (v as PodcastSort) : "recent";
}

export const writePodcastSort = (s: PodcastSort) => writeLocal(KEY, s);

const byName = (x: PodcastView, y: PodcastView) => comparerNaturel.compare(sortKey(x.title), sortKey(y.title));
const byLatest = (x: PodcastView, y: PodcastView) => (y.latest_at ?? "").localeCompare(x.latest_at ?? "");

const COMPARE: Record<PodcastSort, (x: PodcastView, y: PodcastView) => number> = {
  recent: (x, y) => byLatest(x, y) || byName(x, y),
  name: byName,
  added: (x, y) => y.created_at.localeCompare(x.created_at) || byName(x, y),
  new: (x, y) => y.new_count - x.new_count || byLatest(x, y),
};

/** Les favoris d'abord, puis l'ordre choisi. */
export const sortPodcasts = (list: PodcastView[], s: PodcastSort) =>
  [...list].sort((x, y) => Number(y.pinned) - Number(x.pinned) || COMPARE[s](x, y));

/** Options du menu de tri (libellés traduits par l'appelant). */
export function podcastSortOptions(t: (k: string) => string) {
  return [
    { value: "recent", label: t("podcasts.sort_recent"), icon: "material-symbols:schedule-outline-rounded" },
    { value: "name", label: t("podcasts.sort_name"), icon: "material-symbols:sort-by-alpha-rounded" },
    { value: "added", label: t("podcasts.sort_added"), icon: "material-symbols:bookmark-added-outline-rounded" },
    { value: "new", label: t("podcasts.sort_new"), icon: "material-symbols:fiber-new-outline-rounded" },
  ];
}

// ─── Les plus écoutés : classement Apple, nom ou dernier épisode ───
export type PopularSort = "rank" | "name" | "date";
const POPULAR_KEY = "podcasts:tri-populaires";
const POPULAR_SORTS: PopularSort[] = ["rank", "name", "date"];

export function readPopularSort(): PopularSort {
  const v = readLocal(POPULAR_KEY, "rank");
  return POPULAR_SORTS.includes(v as PopularSort) ? (v as PopularSort) : "rank";
}

export const writePopularSort = (s: PopularSort) => writeLocal(POPULAR_KEY, s);

/** « rank » garde l'ordre reçu. */
export function sortDirectory(list: DirectoryResult[], s: PopularSort): DirectoryResult[] {
  if (s === "rank") return list;
  const byTitle = (x: DirectoryResult, y: DirectoryResult) => comparerNaturel.compare(sortKey(x.title), sortKey(y.title));
  return [...list].sort(s === "name" ? byTitle : (x, y) => (y.released_at ?? "").localeCompare(x.released_at ?? "") || byTitle(x, y));
}

export function popularSortOptions(t: (k: string) => string) {
  return [
    { value: "rank", label: t("podcasts.sort_rank"), icon: "material-symbols:trophy-outline-rounded" },
    { value: "name", label: t("podcasts.sort_name"), icon: "material-symbols:sort-by-alpha-rounded" },
    { value: "date", label: t("podcasts.sort_recent"), icon: "material-symbols:schedule-outline-rounded" },
  ];
}
