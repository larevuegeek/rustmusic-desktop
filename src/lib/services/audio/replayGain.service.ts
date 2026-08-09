/**
 * Wrapper for the Replay Gain Tauri commands.
 * Mirrors `src-tauri/src/commands/audio_command.rs`.
 */

import { invoke } from "@tauri-apps/api/core";

/** Which gain reference to apply. */
export type ReplayGainMode = "off" | "track" | "album";

export type ReplayGainSettings = {
  mode: ReplayGainMode;
  /** Pre-amp in dB, clamped to ±15 by the backend. */
  preamp_db: number;
};

export async function getReplayGainSettings(): Promise<ReplayGainSettings> {
  return invoke<ReplayGainSettings>("get_replay_gain_settings");
}

/** Applies and persists. Takes effect on the next track. */
export async function setReplayGainSettings(
  mode: ReplayGainMode,
  preampDb: number,
): Promise<ReplayGainSettings> {
  return invoke<ReplayGainSettings>("set_replay_gain_settings", {
    mode,
    preampDb,
  });
}
