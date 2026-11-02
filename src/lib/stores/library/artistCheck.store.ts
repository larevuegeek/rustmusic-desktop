// Artistes multiples : état en direct de la vérification ; la fenêtre de bilan s'y abonne.
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { get, writable } from "svelte/store";
import { t } from "#lib/i18n";
import { popinStore } from "#lib/stores/ui/popin.store";
import { settingsStore } from "#lib/stores/settings/settings.store";
import { libraryContentStore } from "#lib/stores/library/libraryContent.store";
import ArtistCheckPopin from "#lib/components/library/common/popin/ArtistCheckPopin.svelte";

export type LiveItem = { name: string; decision: "split" | "whole" | "unknown"; parts: string[] };
type State = {
  running: boolean;
  done: number;
  total: number;
  current: string;
  /** Les noms déjà tranchés, dans l'ordre. */
  live: LiveItem[];
  /** Photos connues des artistes vus pendant la vérification (clé : nom en minuscules). */
  images: Record<string, string>;
  /** Début de la vérification (ms), pour estimer le temps restant. */
  startedAt: number;
  /** Le bilan à afficher une fois terminé. */
  runId: number | null;
  error: string | null;
};

const vierge = (o: Partial<State> = {}): State => ({ running: true, done: 0, total: 0, current: "", live: [], images: {}, startedAt: Date.now(), runId: null, error: null, ...o });
const state = writable<State>(vierge({ running: false }));
let initialized = false;

export type RuleView = { name: string; parts: string[]; source: string };
export type CheckReport = {
  run: { id: number; started_at: string; finished_at: string | null; checked: number; error: string | null } | null;
  split: RuleView[];
  whole: RuleView[];
  unknown: RuleView[];
  run_unknown: number;
  images: Record<string, string>;
};

const popinOuverte = () => get(popinStore).component === ArtistCheckPopin;

/** La fenêtre : en direct pendant la vérification, puis le bilan. `one` : un seul nom à vérifier. */
export function openArtistReport(props: { one?: string } = {}) {
  popinStore.open(get(t)("artist_split.report_title"), ArtistCheckPopin, props, { size: "lg", flush: true, icon: "material-symbols:group-outline-rounded" });
}

async function finished(runId: number | null, error: string | null, silent: boolean) {
  state.update((s) => ({ ...s, running: false, current: "", runId, error }));
  libraryContentStore.refresh();
  if (popinOuverte() || !silent) return;
  // Après un scan : la fenêtre ne s'ouvre que s'il y a du neuf.
  if (get(state).live.some((x) => x.decision !== "whole")) openArtistReport();
}

let silencieux = false;

export const artistCheck = {
  subscribe: state.subscribe,

  async init() {
    if (initialized) return;
    initialized = true;
    if (await invoke<boolean>("artist_check_running").catch(() => false)) state.update((s) => ({ ...s, running: true }));
    await listen<{ done: number; total: number; name: string; decision: string; parts: string[]; images?: Record<string, string> }>("artist-check-progress", (e) => {
      const p = e.payload;
      state.update((s) => ({
        ...s,
        running: true,
        done: p.done,
        total: p.total,
        current: p.decision ? s.current : p.name,
        live: p.decision ? [...s.live, { name: p.name, decision: p.decision as LiveItem["decision"], parts: p.parts }] : s.live,
        images: p.images && Object.keys(p.images).length ? { ...s.images, ...p.images } : s.images,
      }));
    });
    await listen<{ run_id: number | null; error: string | null }>("artist-check-done", (e) => void finished(e.payload.run_id, e.payload.error, silencieux));
    await listen("rescan-complete", () => {
      if (get(settingsStore).artist_check_auto === "true") void artistCheck.start(true);
    });
  },

  /** Toute la bibliothèque. La fenêtre s'ouvre tout de suite, sauf en mode silencieux (après un scan). */
  async start(quiet = false) {
    if (!quiet) openArtistReport();
    if (get(state).running) return;
    silencieux = quiet;
    state.set(vierge());
    try {
      await invoke("artist_check_start");
    } catch (e) {
      if (String(e) !== "busy") state.update((s) => ({ ...s, running: false, error: String(e) }));
    }
  },

  /** Un seul nom (menu « … » d'un artiste) : la fenêtre l'affiche en cours, puis son bilan. */
  async checkOne(name: string) {
    if (get(state).running) return openArtistReport();
    silencieux = false;
    state.set(vierge({ total: 1, current: name }));
    openArtistReport({ one: name });
    try {
      const runId = await invoke<number>("artist_check_name", { name });
      await finished(runId, null, false);
    } catch (e) {
      await finished(null, String(e), false);
    }
  },
};
