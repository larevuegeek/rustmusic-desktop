// Dossiers de bibliothèque introuvables au scan (partage réseau non monté, ou monté
// sous un autre nom). Le backend les saute sans toucher à leurs totaux et l'annonce
// ici : on propose de suivre le partage s'il a été retrouvé, sinon on prévient.

import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { get } from "svelte/store";
import { t } from "#lib/i18n";
import { popinStore } from "#lib/stores/ui/popin.store";
import { toasts } from "#lib/stores/ui/toast.store";
import { libraryStore } from "#lib/stores/library/library.store";
import { libraryContentStore } from "#lib/stores/library/libraryContent.store";
import { forgetLocations } from "#lib/helper/library/trackLocation";
import Dialog from "#lib/components/ui/dialog/Dialog.svelte";

type Remount = { old_root: string; new_root: string };
type UnavailableDir = { dir_id: string; path: string; remount: Remount | null };
type Payload = { library_id: number; dirs: UnavailableDir[] };

/** Nom lisible d'un montage GVFS : la valeur de `server=` (`10.0.0.170`, `nas.local`). */
const serveur = (racine: string) => /server=([^,/]+)/.exec(racine)?.[1] ?? racine;

/** Dernier dossier du chemin, pour le message. */
const nomDossier = (chemin: string) => chemin.split("/").filter(Boolean).pop() ?? chemin;

async function relocate(libraryId: number, remount: Remount) {
  const tr = get(t);
  try {
    await invoke("relocate_library_root", { oldRoot: remount.old_root, newRoot: remount.new_root });
  } catch (e) {
    console.error("[bibliothèque] relocalisation impossible :", e);
    toasts.push({ type: "error", title: tr("notify.error"), message: tr("library_unavailable.relocate_failed") });
    return;
  }
  // Le scan suit sans retenir la fenêtre : sa progression s'affiche comme d'habitude.
  invoke("rescan_library", { libraryId })
    .then(() => {
      forgetLocations();
      libraryContentStore.refresh();
      libraryStore.refresh();
    })
    .catch((e) => console.error("[bibliothèque] rescan après relocalisation :", e));
}

export async function initUnavailableDirsListener() {
  return listen<Payload>("library-dirs-unavailable", ({ payload }) => {
    const tr = get(t);
    const retrouve = payload.dirs.find((d) => d.remount);

    if (retrouve?.remount) {
      const remount = retrouve.remount;
      popinStore.open(
        tr("library_unavailable.title"),
        Dialog,
        {
          message: tr("library_unavailable.remount_found")
            .replace("{folder}", nomDossier(retrouve.path))
            .replace("{old}", serveur(remount.old_root))
            .replace("{new}", serveur(remount.new_root)),
          confirmText: tr("library_unavailable.relocate"),
          cancelText: tr("updater.later"),
          onConfirm: () => relocate(payload.library_id, remount),
        },
        { icon: "material-symbols:folder-off-outline-rounded" },
      );
      return;
    }

    const message = payload.dirs.length === 1
      ? tr("library_unavailable.one").replace("{folder}", nomDossier(payload.dirs[0].path))
      : tr("library_unavailable.many").replace("{n}", String(payload.dirs.length));
    toasts.push({ type: "warning", title: tr("library_unavailable.title"), message, duration: 9000 });
  });
}
