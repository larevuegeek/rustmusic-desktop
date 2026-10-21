import { get } from "svelte/store";
import { popinStore } from "#lib/stores/ui/popin.store";
import { whatsNew } from "#lib/stores/ui/whatsNew.store";
import { libraryStore } from "#lib/stores/library/library.store";
import { miniPlayerActive } from "#lib/stores/ui/miniPlayer.store";
import WhatsNewPopin from "#lib/components/whatsnew/WhatsNewPopin.svelte";

/** Ouvre les nouveautés ; `open` retombe à la fermeture (la carte de soutien attend). */
export function openWhatsNew() {
  if (!whatsNew.hasSlides) return;
  whatsNew.markViewed();
  whatsNew.setOpen(true);
  popinStore.open(`RustMusic ${whatsNew.version}`, WhatsNewPopin, {}, { size: "lg", bare: true });
  const unsubscribe = popinStore.subscribe((s) => {
    if (s.component === WhatsNewPopin) return;
    whatsNew.setOpen(false);
    queueMicrotask(() => unsubscribe());
  });
}

/** Au lancement : une fois les bibliothèques chargées, s'il y a du neuf et qu'on ne l'a pas masqué. */
export function showWhatsNewIfDue() {
  if (!whatsNew.hasSlides || get(whatsNew).hidden) return;
  // Sans bibliothèque (première installation), l'accueil guide déjà : rien de « nouveau ».
  if (get(libraryStore).libraries.length === 0) return;
  setTimeout(() => {
    if (!get(popinStore).isOpen && !get(miniPlayerActive)) openWhatsNew();
  }, 1200);
}
