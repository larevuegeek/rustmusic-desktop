/**
 * Tauri commands for the WebKit/GDK rendering mode override (Linux only).
 * Changing this setting requires an app restart to take effect (env vars
 * are read by WebKitGTK at process init).
 */

import { invoke } from "@tauri-apps/api/core";

export type RenderMode = "auto" | "force-gpu" | "force-software";

export type RenderModeStatus = {
  /** Active persisted setting. */
  mode: RenderMode;
  /** Detected virt kind on Linux (`"kvm" | "vmware" | …`), `null` on bare metal / non-Linux. */
  virt_kind: string | null;
  /** Réglage DMA-BUF enregistré (Linux). */
  dmabuf_mode: DmabufMode;
  /** DMA-BUF actif pour la session en cours (un changement attend le redémarrage). */
  dmabuf_active: boolean;
  /** Pilote propriétaire NVIDIA détecté : `auto` y coupe DMA-BUF. */
  nvidia: boolean;
};

export type DmabufMode = "auto" | "on" | "off";

export async function getRenderMode(): Promise<RenderModeStatus> {
  return invoke<RenderModeStatus>("get_render_mode");
}

export async function setRenderMode(value: RenderMode): Promise<RenderModeStatus> {
  return invoke<RenderModeStatus>("set_render_mode", { value });
}

export async function setDmabufMode(value: DmabufMode): Promise<RenderModeStatus> {
  return invoke<RenderModeStatus>("set_dmabuf_mode", { value });
}
