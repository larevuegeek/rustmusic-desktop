// Chemins de fichiers : copier dans le presse-papiers, montrer dans l'explorateur.
import { get } from "svelte/store";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { t } from "$lib/i18n";
import { toasts } from "$lib/stores/ui/toast.store";

export async function copierChemin(chemin: string, message = chemin) {
  try {
    await navigator.clipboard.writeText(chemin);
    toasts.push({ type: "success", title: get(t)("album_view.path_copied"), message });
  } catch (e) {
    console.error("[fichier] presse-papiers :", e);
  }
}

/** « Révéler » plutôt qu'ouvrir : marche hors $HOME et ne lance jamais le fichier. */
export async function revelerDansDossier(chemin: string) {
  try {
    await revealItemInDir(chemin);
  } catch (e) {
    console.error("[fichier] explorateur :", e);
  }
}
