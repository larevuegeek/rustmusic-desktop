// Découpe d'un tag compositeur, comme service/library/composer.rs : décisions des artistes multiples, mentions écartées, « , ; / ».
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { writable } from "svelte/store";

export type ComposerSplitConfig = { junk: string[]; rules: Record<string, string[]> };

const config = writable<ComposerSplitConfig>({ junk: [], rules: {} });
let initialized = false;

const load = () => invoke<ComposerSplitConfig>("composer_split_config").then(config.set).catch(() => {});

/** Chargée au premier usage, puis après chaque vérification des artistes multiples. */
export function ensureComposerSplit() {
  if (initialized) return;
  initialized = true;
  void load();
  void listen("artist-check-done", () => void load());
}

export const composerSplit = { subscribe: config.subscribe };

export function splitComposer(raw: string | null | undefined, c: ComposerSplitConfig): string[] {
  const nom = (raw ?? "").trim();
  const ecarte = (n: string) => c.junk.includes(n.toLowerCase());
  if (!nom || ecarte(nom)) return [];
  const regle = c.rules[nom.toLowerCase()];
  if (regle) return regle;
  const vus = new Set<string>();
  return nom
    .split(/[,;/\0]/)
    .map((n) => n.trim())
    .filter((n) => n && !ecarte(n) && !vus.has(n.toLowerCase()) && vus.add(n.toLowerCase()));
}
