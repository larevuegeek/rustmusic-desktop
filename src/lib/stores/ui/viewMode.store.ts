import { writable } from "svelte/store";
import { readLocal, writeLocal } from "#lib/helper/tools/storage";

export type ViewMode = "grid" | "list";

const viewModeWriter = writable<ViewMode>(readLocal("viewMode", "grid") === "list" ? "list" : "grid");

viewModeWriter.subscribe((value) => writeLocal("viewMode", value));

export const viewMode = {
  subscribe: viewModeWriter.subscribe,
  toggle: () => viewModeWriter.update(v => v === "grid" ? "list" : "grid"),
  set: (mode: ViewMode) => viewModeWriter.set(mode),
};
