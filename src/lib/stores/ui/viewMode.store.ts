import { writable } from "svelte/store";
import { lireLocal, ecrireLocal } from "$lib/helper/tools/stockage";

export type ViewMode = "grid" | "list";

const viewModeWriter = writable<ViewMode>(lireLocal("viewMode", "grid") === "list" ? "list" : "grid");

viewModeWriter.subscribe((value) => ecrireLocal("viewMode", value));

export const viewMode = {
  subscribe: viewModeWriter.subscribe,
  toggle: () => viewModeWriter.update(v => v === "grid" ? "list" : "grid"),
  set: (mode: ViewMode) => viewModeWriter.set(mode),
};
