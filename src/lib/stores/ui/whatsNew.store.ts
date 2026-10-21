import { writable } from "svelte/store";
import { readLocal, writeLocal } from "#lib/helper/tools/storage";
import { slidesFor } from "#lib/config/whatsNew";

// Valeur = version concernée : une nouvelle version remet tout à zéro.
const VIEWED_KEY = "rm-whatsnew-viewed";
const HIDDEN_KEY = "rm-whatsnew-hidden";
const version = __APP_VERSION__;

type WhatsNewState = {
  /** Diapos de cette version déjà ouvertes : plus de pastille sur le mégaphone. */
  viewed: boolean;
  /** « Ne plus afficher » coché : plus d'ouverture au lancement pour cette version. */
  hidden: boolean;
  open: boolean;
};

const state = writable<WhatsNewState>({
  viewed: readLocal(VIEWED_KEY, null) === version,
  hidden: readLocal(HIDDEN_KEY, null) === version,
  open: false,
});

export const whatsNew = {
  subscribe: state.subscribe,
  version,
  hasSlides: slidesFor(version).length > 0,
  markViewed() {
    writeLocal(VIEWED_KEY, version);
    state.update((s) => ({ ...s, viewed: true }));
  },
  setHidden(hidden: boolean) {
    writeLocal(HIDDEN_KEY, hidden ? version : "");
    state.update((s) => ({ ...s, hidden }));
  },
  setOpen: (open: boolean) => state.update((s) => ({ ...s, open })),
};
