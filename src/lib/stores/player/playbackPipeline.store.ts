/**
 * Reactive snapshot of the current playback pipeline (source → resampler → output).
 *
 * Populated by listening to the Tauri event `"playback-pipeline"` which the
 * Rust audio threads emit at the start of each track. Lets the player UI
 * display things like "DSD64 → 88.2 kHz" with a visible arrow when a
 * conversion is in flight.
 */

import { writable } from "svelte/store";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type PlaybackPipelineInfo = {
  /** "DSD64", "FLAC", "MP3", ... */
  source_format: string;
  source_sample_rate: number;
  source_bits: number;
  source_channels: number;

  /** Only set for DSD playback (intermediate PCM rate after DSD2PCM filter). */
  intermediate_pcm_rate: number | null;
  /** DSD filter taps (e.g. 2048 / 1024 / 512). Only set for DSD. */
  dsd_filter_taps: number | null;
  /** DSD decimation factor (e.g. 32 for DSD64 → 88.2 kHz). Only set for DSD. */
  dsd_decimation: number | null;

  /** Device sample rate (what CPAL outputs). */
  output_sample_rate: number;
  output_channels: number;
  /** Human name of the active output device. */
  device_name: string;

  /** True when a resampler is in the chain (source rate ≠ device rate). */
  resampler_active: boolean;
  /** Active quality profile : "high" | "medium" | "low". */
  quality_profile: string;
  /** Effective audio backend : "CPAL shared" | "WASAPI exclusive". */
  backend: string;
  /** Chaîne intacte à volume 100 % (exclusif, rate et canaux source, sans perte, Replay Gain neutre). */
  bit_perfect: boolean;
  /** Sortie exclusive ou DoP demandés mais non obtenus : le motif. */
  repli?: Repli | null;
};

export type Repli = "format_refuse" | "appareil_occupe" | "appareil_introuvable" | "indisponible" | "dop_refuse";

/** L'état de la chaîne pour la pastille du lecteur (voir `CHAINE`). */
export type PipelineMode = "dop" | "bit-perfect" | "exclusive" | "resampled" | "dsd" | "shared";

export function pipelineMode(info: PlaybackPipelineInfo | null, volume = 100): PipelineMode | null {
  if (!info) return null;
  // DoP : DSD natif au DAC (« WASAPI DoP », « ALSA DoP », « CoreAudio DoP »).
  if (info.backend.endsWith("DoP")) return "dop";
  if (info.intermediate_pcm_rate != null) return "dsd";
  if (info.resampler_active) return "resampled";
  // Le volume logiciel recalcule chaque échantillon : bit-perfect à 100 % seulement.
  if (info.bit_perfect && volume >= 100) return "bit-perfect";
  if (info.backend.includes("exclusive")) return "exclusive";
  return "shared";
}

export const playbackPipelineStore = writable<PlaybackPipelineInfo | null>(null);

let unlisten: UnlistenFn | null = null;

/**
 * Subscribe to the backend event. Idempotent — calling twice doesn't double-subscribe.
 * Should be called once at app boot from `+layout.svelte`.
 */
export async function initPlaybackPipelineListener(): Promise<void> {
  if (unlisten !== null) return;
  unlisten = await listen<PlaybackPipelineInfo>("playback-pipeline", (event) => {
    playbackPipelineStore.set(event.payload);
  });
}

/** Stop listening — only useful for hot-reload / cleanup in dev. */
export async function disposePlaybackPipelineListener(): Promise<void> {
  if (unlisten) {
    unlisten();
    unlisten = null;
  }
}
