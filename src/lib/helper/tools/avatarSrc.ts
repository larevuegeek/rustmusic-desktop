// Image de profil : un fichier du dossier de l'appli (servi par le protocole asset), ou une adresse déjà prête.
import { convertFileSrc } from "@tauri-apps/api/core";

export function avatarSrc(path: string | null | undefined): string | undefined {
  if (!path) return undefined;
  if (/^(https?:|data:|blob:|asset:)/.test(path)) return path;
  return convertFileSrc(path);
}
