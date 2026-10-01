/**
 * Couleur du morceau en cours, lue sur sa pochette : `h` teinte OKLCH, `s` intensité (0 gris → 1 franc).
 * Partagée par le lecteur et le mini-lecteur ; ne se recalcule qu'au changement de pochette.
 */
import { derived } from "svelte/store";
import { player } from "$lib/stores/player/player.store";
import { queueState } from "$lib/stores/queue/queueState.store";
import { resolveCoverSrc } from "$lib/helper/tools/coverHelper";
import { couleurPochette, type CouleurPochette } from "$lib/helper/tools/couleurPochette";

// La vignette sur disque, sinon l'image du tag (data:) ; `null` sans morceau.
const pochette = derived([player, queueState], ([$p, $q]) => {
  if (!$p?.pathFile) return null;
  const cover = $q.tracks[$q.currentIndex]?.cover;
  if (cover && cover !== "/images/no-cd.png" && !cover.startsWith("data:")) return { disque: cover };
  const tag = $p.audioFile?.tags?.attached_images?.[0]?.image_src;
  return tag?.startsWith("data:") ? { tag } : null;
}, null as { disque?: string; tag?: string } | null);

let derniere = "";
let teinte = 150;

// Sans pochette lisible, la teinte reste et l'intensité tombe : le lecteur passe au gris.
export const couleurMorceau = derived(pochette, ($pochette, set) => {
  const cle = $pochette?.disque ?? $pochette?.tag ?? "";
  if (cle === derniere) return;
  derniere = cle;
  if (!cle) {
    set({ h: teinte, s: 0 });
    return;
  }
  (async () => {
    const src = $pochette?.disque ? await resolveCoverSrc($pochette.disque, "1x") : $pochette?.tag;
    const c = src ? await couleurPochette(src) : null;
    if (derniere !== cle) return;
    if (c) teinte = c.h;
    set({ h: teinte, s: c?.s ?? 0 });
  })();
}, { h: 150, s: 0 } as CouleurPochette);
