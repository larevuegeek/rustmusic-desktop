import { writable, get } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";

export type PinKind = "album" | "artist";

/** Un album ou un artiste épinglé dans la barre latérale. */
export type LibraryPin = {
  kind: PinKind;
  id: string;
  title: string;
  subtitle: string | null;
  cover: string | null;
};

type PinsState = { libraryId: number | null; pins: LibraryPin[] };

const pinsWriter = writable<PinsState>({ libraryId: null, pins: [] });

let demande = 0;

async function charger(libraryId: number | null) {
  const n = ++demande;
  if (libraryId == null) {
    pinsWriter.set({ libraryId: null, pins: [] });
    return;
  }
  try {
    const pins = await invoke<LibraryPin[]>("get_library_pins", { libraryId });
    // Une réponse doublée par un changement de bibliothèque est jetée.
    if (n === demande) pinsWriter.set({ libraryId, pins: pins ?? [] });
  } catch (e) {
    console.error("Épingles :", e);
  }
}

export const pinsStore = {
  subscribe: pinsWriter.subscribe,
  load: charger,

  async setPinned(kind: PinKind, libraryId: number, id: string, pinned: boolean) {
    const commande = `${pinned ? "pin" : "unpin"}_${kind}`;
    await invoke(commande, kind === "album" ? { libraryId, albumId: id } : { libraryId, artistId: id });
    if (get(pinsWriter).libraryId === libraryId) await charger(libraryId);
  },
};

export function isPinned(state: PinsState, kind: PinKind, id: string | null | undefined): boolean {
  return !!id && state.pins.some((p) => p.kind === kind && p.id === id);
}
