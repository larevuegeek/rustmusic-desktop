import { writable, get } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { profilSelector } from "#lib/stores/profil/profil.store";
import type { RadioRecent, RadioResult, RadioView, StreamVariant } from "#lib/types/ui/radio/Radio";

type RadioState = {
  radios: RadioView[];
  /** Dernières écoutées, la plus récente d'abord. */
  recent: RadioRecent[];
  loaded: boolean;
  /** Profil des favoris affichés. */
  profilId: number | null;
};

const radioWriter = writable<RadioState>({ radios: [], recent: [], loaded: false, profilId: null });

const currentProfileId = () => get(profilSelector).profilSelected?.id ?? null;

// Seule la dernière demande s'applique (changement de profil rapide).
let loadSeq = 0;
let inflight: { profilId: number; promise: Promise<void> } | null = null;
let ecoute: Promise<unknown> | null = null;

async function load(profilId = currentProfileId()) {
  if (profilId == null) return;
  // Une écoute vient d'être retenue (logo compris) : « Dernières écoutées » se met à jour.
  ecoute ??= listen<number>("radios-recent-updated", (e) => {
    if (e.payload === get(radioWriter).profilId) refreshRecent();
  });
  const seq = ++loadSeq;
  try {
    const [radios, recent] = await Promise.all([
      invoke<RadioView[]>("get_radios", { profilId }),
      invoke<RadioRecent[]>("get_radio_recent", { profilId }),
    ]);
    if (seq === loadSeq) radioWriter.set({ radios, recent, loaded: true, profilId });
  } catch (e) {
    console.error("[radios] chargement :", e);
  }
}

async function refreshRecent() {
  const profilId = get(radioWriter).profilId;
  if (profilId == null) return;
  const recent = await invoke<RadioRecent[]>("get_radio_recent", { profilId }).catch(() => null);
  if (recent && get(radioWriter).profilId === profilId) radioWriter.update((s) => ({ ...s, recent }));
}

/** Charge les favoris du profil s'ils ne sont pas déjà là (ni en route). */
function ensureLoaded(profilId: number) {
  if (get(radioWriter).profilId === profilId || inflight?.profilId === profilId) return;
  const promise = load(profilId).finally(() => {
    if (inflight?.promise === promise) inflight = null;
  });
  inflight = { profilId, promise };
}

// Logos de l'annuaire : une seule demande par adresse.
const logos = new Map<string, Promise<string | null>>();

/** Une favorite (ou une écoute récente) sous la forme d'un résultat d'annuaire : mêmes cartes, même menu. */
export function toResult(r: RadioView | RadioRecent): RadioResult {
  return {
    uuid: r.uuid,
    name: r.name,
    stream_url: r.stream_url,
    homepage: r.homepage,
    favicon: r.favicon_url,
    country_code: r.country_code,
    tags: r.tags ? r.tags.split(",").filter(Boolean) : [],
    codec: r.codec,
    bitrate: r.bitrate,
    reduced_quality: r.reduced_quality,
  };
}

/** La favorite qui correspond à une station de l'annuaire (même fiche ou même flux). */
export function findFavorite(radios: RadioView[], s: RadioResult): RadioView | null {
  return radios.find((r) => (s.uuid && r.uuid === s.uuid) || r.stream_url === s.stream_url) ?? null;
}

export const radioStore = {
  subscribe: radioWriter.subscribe,
  load,
  ensureLoaded,

  search: (term: string, country: string | null, tag: string | null) => invoke<RadioResult[]>("radio_search", { term, country, tag }),

  /** Les plus écoutées d'un pays ; `tag` : un genre. */
  top: (country: string, tag: string | null) => invoke<RadioResult[]>("radio_top", { country, tag }),

  /** Décrit une adresse collée (flux, .m3u, .pls). */
  resolve: (url: string) => invoke<RadioResult>("radio_resolve", { url }),

  logo(url: string | null): Promise<string | null> {
    if (!url) return Promise.resolve(null);
    let p = logos.get(url);
    if (!p) {
      p = invoke<string | null>("radio_logo", { url }).catch(() => null);
      logos.set(url, p);
    }
    return p;
  },

  async add(station: RadioResult): Promise<RadioView> {
    const profilId = currentProfileId();
    if (profilId == null) throw new Error("aucun profil");
    const r = await invoke<RadioView>("radio_add", { profilId, station });
    radioWriter.update((s) => ({ ...s, radios: [...s.radios.filter((x) => x.id !== r.id), r] }));
    return r;
  },

  /** Les versions du flux d'une station, d'après l'annuaire. */
  variants: (name: string, country: string | null) => invoke<StreamVariant[]>("radio_variants", { name, country }),

  /** La favorite passe à une autre version de son flux. */
  async setStream(id: number, variant: StreamVariant) {
    await invoke("radio_set_stream", { radioId: id, variant });
    radioWriter.update((s) => ({ ...s, radios: s.radios.map((x) => (x.id === id ? { ...x, ...variant } : x)) }));
  },

  async remove(id: number) {
    await invoke("radio_remove", { radioId: id });
    radioWriter.update((s) => ({ ...s, radios: s.radios.filter((x) => x.id !== id) }));
  },

  async rename(id: number, name: string) {
    await invoke("radio_rename", { radioId: id, name });
    radioWriter.update((s) => ({ ...s, radios: s.radios.map((x) => (x.id === id ? { ...x, name: name.trim() } : x)) }));
  },

  async reorder(ids: number[]) {
    const profilId = currentProfileId();
    if (profilId == null) return;
    radioWriter.update((s) => ({ ...s, radios: ids.map((id) => s.radios.find((x) => x.id === id)).filter((x): x is RadioView => !!x) }));
    await invoke("reorder_radios", { profilId, ids });
  },
};
