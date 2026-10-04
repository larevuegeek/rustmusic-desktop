import { invoke } from "@tauri-apps/api/core";
import { get } from "svelte/store";
import { t, currentLocale } from "$lib/i18n";
import { toasts } from "$lib/stores/ui/toast.store";
import { libraryContentStore } from "$lib/stores/library/libraryContent.store";
import { tailleLisible } from "$lib/helper/tools/sizeTools";

type Bilan = { found: number; errors: number; cancelled: boolean };

/** Une seule passe de chaque sorte à la fois : la seconde est signalée, pas relancée. */
async function lancer(titre: string, cleFin: string, passe: () => Promise<Bilan>): Promise<void> {
  const tr = get(t);
  try {
    const b = await passe();
    libraryContentStore.refresh();
    let message = tr(b.cancelled ? "library_head.fetch_stopped" : cleFin).replace("{n}", String(b.found));
    if (b.errors > 0) message += tr("library_head.fetch_retry").replace("{n}", String(b.errors));
    toasts.push({ type: b.cancelled ? "info" : "success", title: titre, message });
  } catch (e) {
    const deja = String(e).includes("deja_en_cours");
    toasts.push({ type: deja ? "info" : "error", title: titre, message: tr(deja ? "library_head.fetch_running" : "notify.error") });
  }
}

/** Portraits manquants ; `relancer` : y compris les artistes restés sans résultat. */
export function recupererPortraits(relancer = false): Promise<void> {
  return lancer(get(t)("library_head.artist_images"), "library_head.artist_images_done", () =>
    invoke<Bilan>("fetch_all_artist_images", { force: relancer }));
}

/** Pochettes manquantes, bibliothèque après bibliothèque. */
export function recupererPochettes(libraryIds: number[]): Promise<void> {
  return lancer(get(t)("library_head.covers"), "library_head.covers_done", async () => {
    const total: Bilan = { found: 0, errors: 0, cancelled: false };
    for (const libraryId of libraryIds) {
      const b = await invoke<Bilan>("fetch_all_album_covers", { libraryId });
      total.found += b.found;
      total.errors += b.errors;
      if (b.cancelled) return { ...total, cancelled: true };
    }
    return total;
  });
}

/** Supprime les images que plus rien n'utilise et allège les plus lourdes. */
export async function nettoyerImages(): Promise<void> {
  const tr = get(t);
  try {
    const b = await invoke<{ supprimes: number; alleges: number; liberes: number }>("clean_image_cache");
    const message = b.supprimes + b.alleges === 0
      ? tr("settings.clean_images_none")
      : tr("settings.clean_images_done")
          .replace("{n}", String(b.supprimes))
          .replace("{m}", String(b.alleges))
          .replace("{size}", tailleLisible(b.liberes, get(currentLocale)));
    toasts.push({ type: "success", title: tr("settings.clean_images"), message });
  } catch (e) {
    console.error("[images] nettoyage :", e);
    toasts.push({ type: "error", title: tr("settings.clean_images"), message: tr("notify.error") });
  }
}
