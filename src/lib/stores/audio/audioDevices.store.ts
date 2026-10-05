/**
 * Périphériques de sortie audio enrichis (capacités du DAC).
 *
 * Le backend Rust énumère CPAL, agrège les `supported_output_configs` par
 * device et renvoie fréquences supportées, formats d'échantillon, canaux
 * max, marqueur défaut système. Chargé à la demande — les capacités CPAL
 * ne changent qu'au branchement/débranchement d'un device.
 */

import { writable, get } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";

export type AudioDeviceInfo = {
  /** Nom brut CPAL — sert d'identifiant pour `set_device`. */
  name: string;
  /** Nom formaté "Nom (Fabricant)" pour l'UI. */
  displayName: string;
  manufacturer: string | null;
  driver: string | null;
  isDefault: boolean;
  /** Fréquences supportées, triées ascendant (Hz). */
  sampleRates: number[];
  /** Libellés des formats : "16-bit int" / "24-bit int" / "32-bit float"… */
  sampleFormats: string[];
  /** Nombre max de canaux exposés. */
  maxChannels: number;
  /** Taille min de buffer exposée (frames). `null` si driver = "Unknown". */
  minBufferFrames: number | null;
  /** Taille max de buffer exposée (frames). */
  maxBufferFrames: number | null;
  /** Vrai si ≥ 24-bit ET ≥ 88.2 kHz — badge "Hi-Res" du dropdown. */
  isHires: boolean;
  /** ID WASAPI pour probing détaillé (Windows uniquement). */
  wasapiId: string | null;
  /** Bus de l'adaptateur : « USB », « HDAUDIO », « BTHENUM »… (Windows uniquement). */
  bus: string | null;
  /** Sortie qu'utilise le lecteur : celle choisie et enregistrée, restaurée au lancement. */
  isSelected: boolean;
};

/** Résultat du probing WASAPI par device (Windows uniquement). */
export type WasapiDeviceCapabilities = {
  exclusiveRates: number[];
  exclusiveBitDepths: number[];
  sharedRates: number[];
  mixRate: number | null;
  mixBitDepth: number | null;
  mixChannels: number | null;
};

type State = {
  loaded: boolean;
  loading: boolean;
  devices: AudioDeviceInfo[];
  error: string | null;
  /**
   * displayName (unique) du device actif. On track par displayName parce
   * que le raw `name` CPAL peut collisionner entre plusieurs endpoints
   * distincts (ex. « Haut-parleurs (Realtek) » et « Haut-parleurs (Fosi) »
   * partagent le raw name « Haut-parleurs »). Le raw name reste envoyé
   * à `set_device` pour l'identification côté CPAL.
   */
  activeDisplayName: string | null;
  /** Capacités réelles en exclusif par displayName (Windows) : CPAL y donne la même table partout. */
  exclusive: Record<string, WasapiDeviceCapabilities>;
};

const initial: State = {
  loaded: false,
  loading: false,
  devices: [],
  error: null,
  activeDisplayName: null,
  exclusive: {},
};

function createStore() {
  const { subscribe, set, update } = writable<State>({ ...initial });

  async function refresh(): Promise<void> {
    update((s) => ({ ...s, loading: true, error: null }));
    try {
      const devices = await invoke<AudioDeviceInfo[]>("get_output_devices");
      update((s) => {
        // Le lecteur fait foi : sa sortie restaurée, puis la dernière choisie ici
        // si elle existe encore, puis celle du système.
        const encorePresente = devices.some((d) => d.displayName === s.activeDisplayName);
        const active =
          devices.find((d) => d.isSelected)?.displayName
          ?? (encorePresente ? s.activeDisplayName : null)
          ?? devices.find((d) => d.isDefault)?.displayName
          ?? null;
        return { ...s, loaded: true, loading: false, devices, activeDisplayName: active, exclusive: {} };
      });
      sonder(devices);
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      update((s) => ({ ...s, loading: false, error: msg }));
    }
  }

  /** Interroge le pilote de chaque sortie (formats acceptés, sans ouvrir de flux), l'une après l'autre. */
  async function sonder(devices: AudioDeviceInfo[]): Promise<void> {
    for (const d of devices) {
      if (!d.wasapiId) continue;
      try {
        const caps = await invoke<WasapiDeviceCapabilities>("wasapi_probe_device_capabilities", { deviceId: d.wasapiId });
        update((s) => ({ ...s, exclusive: { ...s.exclusive, [d.displayName]: caps } }));
      } catch {
        // Sortie débranchée entre-temps : on garde les valeurs CPAL.
      }
    }
  }

  async function ensureLoaded(): Promise<void> {
    const state = get({ subscribe });
    if (state.loaded || state.loading) return;
    await refresh();
  }

  function setActive(displayName: string): void {
    update((s) => ({ ...s, activeDisplayName: displayName }));
  }

  return { subscribe, refresh, ensureLoaded, setActive };
}

export const audioDevicesStore = createStore();

/** Formate une fréquence en libellé compact : "44.1 kHz", "192 kHz", "2.8 MHz". */
export function formatSampleRate(hz: number): string {
  if (hz >= 1_000_000) return `${(hz / 1_000_000).toFixed(2)} MHz`;
  if (hz >= 1_000)
    return `${(hz / 1_000).toFixed(hz % 1000 === 0 ? 0 : 1)} kHz`;
  return `${hz} Hz`;
}

/** Le format le plus "profond" du device : "24-bit" > "16-bit" > "32-bit float". */
export function bestFormatLabel(formats: string[]): string | null {
  if (formats.includes("32-bit float")) return "32-bit float";
  if (formats.includes("32-bit int")) return "32-bit";
  if (formats.includes("24-bit int")) return "24-bit";
  if (formats.includes("16-bit int")) return "16-bit";
  return formats[0] ?? null;
}

/** Fréquence maximum supportée (utile pour le badge du dropdown). */
export function maxSampleRate(rates: number[]): number | null {
  if (rates.length === 0) return null;
  return rates[rates.length - 1];
}
