// Ce que permet l'installation : Flatpak (mises à jour par Flathub, portails) et sortie exclusive
// (carte son accessible ou non). Lu une fois au lancement.
import { get, writable } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";

export type PlatformFeatures = { flatpak: boolean; exclusive: boolean };

export const platformFeatures = writable<PlatformFeatures>({ flatpak: false, exclusive: true });

let lecture: Promise<PlatformFeatures> | null = null;

export function loadPlatformFeatures(): Promise<PlatformFeatures> {
  lecture ??= invoke<PlatformFeatures>("get_platform_features")
    .then((f) => {
      platformFeatures.set(f);
      return f;
    })
    .catch(() => get(platformFeatures));
  return lecture;
}
