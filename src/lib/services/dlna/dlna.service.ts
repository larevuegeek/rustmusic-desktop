/**
 * Wrapper for the Tauri DLNA commands. Mirror of `commands/network/dlna_command.rs`.
 */

import { invoke } from "@tauri-apps/api/core";

export type DlnaStatus = {
  running: boolean;
  friendly_name: string;
  port: number;
  /** "http://192.168.1.10:8200" when running, null otherwise. */
  url: string | null;
};

export type DlnaSettings = {
  enabled: boolean;
  friendly_name: string;
  port: number;
  uuid: string;
};

export async function dlnaGetSettings(): Promise<DlnaSettings> {
  return invoke<DlnaSettings>("dlna_get_settings");
}

export async function dlnaGetStatus(): Promise<DlnaStatus> {
  return invoke<DlnaStatus>("dlna_status");
}

export async function dlnaStart(): Promise<DlnaStatus> {
  return invoke<DlnaStatus>("dlna_start");
}

export async function dlnaStop(): Promise<DlnaStatus> {
  return invoke<DlnaStatus>("dlna_stop");
}

/**
 * Update friendly name and/or port. If the server was running, it is
 * automatically restarted to apply the new settings.
 */
export async function dlnaUpdateSettings(
  friendlyName?: string,
  port?: number,
): Promise<DlnaStatus> {
  return invoke<DlnaStatus>("dlna_update_settings", {
    friendlyName,
    port,
  });
}
