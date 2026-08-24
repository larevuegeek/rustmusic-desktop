import type { Group, Limit } from "./types";

/**
 * Des playlists intelligentes toutes faites.
 *
 * Composer « les plus écoutés » à la main demande de savoir qu'il faut une
 * condition sur le nombre d'écoutes, un tri décroissant et une coupe — trois
 * choix pour une idée qui en est une seule. Ces recettes les posent d'un clic,
 * et restent modifiables ensuite : ce sont des points de départ, pas des
 * playlists d'un genre à part.
 */

export type Preset = {
  key: string;
  name: string;
  icon: string;
  color: string;
  /** Ce que la recette fait, dit en une ligne. */
  hint: string;
  rules: Group;
  limit: Limit;
};

export const PRESETS: Preset[] = [
  {
    key: "most_played",
    name: "Les plus écoutés",
    icon: "mynaui:fire",
    color: "#f97316",
    hint: "Les 100 morceaux que tu as le plus joués",
    rules: {
      match: "all",
      rules: [{ field: "play_count", op: "gt", value: 0 }],
    },
    limit: { count: 100, sort: "play_count", desc: true },
  },
  {
    key: "top_genres",
    name: "Mes styles favoris",
    icon: "mynaui:star",
    color: "#8b5cf6",
    // Le classement se recalcule à chaque ouverture : cette playlist suit les
    // goûts, elle ne fige pas une liste de genres décidée un jour.
    hint: "Au hasard dans les 5 genres que tu écoutes le plus",
    rules: {
      match: "all",
      rules: [{ field: "genre", op: "in_top_played", value: 5 }],
    },
    limit: { count: 100, sort: "random", desc: false },
  },
  {
    key: "top_artists",
    name: "Mes artistes favoris",
    icon: "mynaui:heart",
    color: "#ec4899",
    hint: "Au hasard chez les 10 artistes que tu écoutes le plus",
    rules: {
      match: "all",
      rules: [{ field: "artist", op: "in_top_played", value: 10 }],
    },
    limit: { count: 100, sort: "random", desc: false },
  },
  {
    key: "best_rated",
    name: "Mes préférés",
    icon: "mynaui:star",
    color: "#22c55e",
    hint: "Tout ce que tu as noté 4 étoiles ou plus",
    rules: {
      match: "all",
      rules: [{ field: "rating", op: "gte", value: 4 }],
    },
    limit: { count: null, sort: "rating", desc: true },
  },
  {
    key: "never_played",
    name: "Jamais écoutés",
    icon: "mynaui:moon",
    color: "#0ea5e9",
    hint: "Ce qui dort dans la bibliothèque depuis son import",
    rules: {
      match: "all",
      rules: [{ field: "play_count", op: "eq", value: 0 }],
    },
    limit: { count: 200, sort: "random", desc: false },
  },
  {
    key: "forgotten",
    name: "Oubliés",
    icon: "mynaui:cloud",
    color: "#f59e0b",
    // Deux conditions, et la première compte : sans elle, la playlist se
    // remplirait de morceaux jamais écoutés, qui ne sont pas « oubliés ».
    hint: "Déjà aimés, mais plus joués depuis six mois",
    rules: {
      match: "all",
      rules: [
        { field: "play_count", op: "gt", value: 0 },
        { field: "last_played_at", op: "not_in_last", value: 180 },
      ],
    },
    limit: { count: 100, sort: "random", desc: false },
  },
  {
    key: "recent",
    name: "Ajouts récents",
    icon: "mynaui:sun",
    color: "#14b8a6",
    hint: "Entrés dans la bibliothèque ce dernier mois",
    rules: {
      match: "all",
      rules: [{ field: "created_at", op: "in_last", value: 30 }],
    },
    limit: { count: null, sort: "created_at", desc: true },
  },
  {
    key: "hires",
    name: "Haute résolution",
    icon: "mynaui:lightning",
    color: "#6366f1",
    hint: "24 bits ou plus — de quoi tirer parti du bit-perfect",
    rules: {
      match: "all",
      rules: [{ field: "bits_per_sample", op: "gte", value: 24 }],
    },
    limit: { count: null, sort: "sample_rate", desc: true },
  },
];
