/**
 * Wrapper for the audio quality profile Tauri commands.
 * Mirrors `src-tauri/src/commands/audio/audio_command.rs`.
 */

import { invoke } from "@tauri-apps/api/core";

/** User-facing setting persisted in the DB. */
export type AudioQualitySetting = "auto" | "high" | "medium" | "low" | "minimal";

/** Concrete preset applied to the audio pipeline. `Auto` resolves to one of these. */
export type AudioQualityProfile = "high" | "medium" | "low" | "minimal";

export type AudioQualityStatus = {
  /** What the user picked (auto/high/medium/low). */
  setting: AudioQualitySetting;
  /** Resolved profile actually driving the pipeline (high/medium/low). */
  resolved: AudioQualityProfile;
  /** "kvm" / "vmware" / ... when virtualised, null on bare metal. */
  virt_kind: string | null;
  /** Logical core count detected on the host. */
  cpu_cores: number;
};

export async function getAudioQualityStatus(): Promise<AudioQualityStatus> {
  return invoke<AudioQualityStatus>("get_audio_quality_status");
}

export async function setAudioQualitySetting(
  value: AudioQualitySetting,
): Promise<AudioQualityStatus> {
  return invoke<AudioQualityStatus>("set_audio_quality_setting", { value });
}
