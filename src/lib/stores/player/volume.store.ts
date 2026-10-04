import { writable } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";

/** Volume du lecteur (0–100), partagé : glissière, raccourcis et pastille bit-perfect. */
export const volume = writable(80);

let minuteur: ReturnType<typeof setTimeout> | null = null;

/** Relit le volume du moteur (au démarrage, après une sourdine par raccourci). */
export async function chargerVolume(): Promise<void> {
  try {
    volume.set(await invoke<number>("get_volume"));
  } catch (e) {
    console.error("[volume] lecture :", e);
  }
}

/** Tout de suite à l'écran, au plus toutes les 50 ms vers le moteur. */
export function reglerVolume(v: number): void {
  const borne = Math.max(0, Math.min(100, Math.round(v)));
  volume.set(borne);
  if (minuteur) clearTimeout(minuteur);
  minuteur = setTimeout(() => {
    invoke("set_volume", { volume: borne }).catch((e) => console.error("[volume] réglage :", e));
  }, 50);
}
