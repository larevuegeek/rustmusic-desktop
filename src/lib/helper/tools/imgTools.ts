import type { AudioFile } from "$lib/types/db/audioFile/AudioFile";
import { invoke } from "@tauri-apps/api/core";
import { appDataDir, resolve } from "@tauri-apps/api/path";

export default async function getCoverUrl(relativePath: string): Promise<string> {
  // 1) nettoie le chemin venant de la DB
  const clean = (relativePath ?? "")
    .trim()                       // vire espaces / \n / \r
    .replace(/^[\/\\]+/, "")       // vire / ou \ au début
    .replace(/\\/g, "/");          // normalise windows -> /

  const base = await appDataDir();

  const native = await resolve(base, clean);

  return native;
}

/** Rust relit le fichier et ecrit la vignette : les octets ne transitent pas. */
export async function thumbnail_getter(selectedPath: string, audioFile: AudioFile): Promise<string | undefined | null> {
  if (!audioFile.tags?.attached_images?.length) return null;
  return await invoke<string | null>("save_thumbnail_from_file", { path: selectedPath });
}
