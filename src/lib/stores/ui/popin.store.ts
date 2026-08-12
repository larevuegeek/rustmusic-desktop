import type { Component } from "svelte"
import { writable } from "svelte/store";

// eslint-disable-next-line @typescript-eslint/no-explicit-any
type AnyComponent = Component<any, any, any>;

/** Largeur du panneau. Un formulaire court et un outil d'édition n'ont pas
 *  les mêmes besoins : forcer les deux dans la même colonne étroite oblige
 *  l'un à défiler sans fin. */
export type PopinSize = "md" | "lg" | "xl";

export type PopinOptions = {
  /** Défaut `md` — la largeur historique, adaptée aux formulaires courts. */
  size?: PopinSize;
  /** Nom d'icône Iconify affiché devant le titre (`lucide:tags`…). */
  icon?: string;
  /**
   * Retire le rembourrage et le défilement gérés par la popin : le composant
   * reçoit une boîte vide et contrôle entièrement sa mise en page.
   *
   * Indispensable dès qu'on veut un pied de page qui ne défile pas ou deux
   * colonnes qui défilent indépendamment — impossible si c'est la popin qui
   * possède l'unique zone de défilement.
   */
  flush?: boolean;
};

export type PopinState = {
    isOpen: boolean,
    title: string,
    component: AnyComponent | null
    props?: Record<string, any>;
    size: PopinSize;
    flush: boolean;
    icon: string | null;
}

const initialState: PopinState = {
  isOpen: false,
  title: "",
  component: null,
  props: {},
  size: "md",
  flush: false,
  icon: null
};

const popinWriter = writable<PopinState>(initialState);

/**
 * Veto de fermeture posé par le contenu.
 *
 * Sans lui, un formulaire d'édition perd la saisie en cours au moindre Échap
 * ou clic à côté, sans rien dire. Le contenu est le seul à savoir s'il a
 * quelque chose à perdre : c'est donc à lui de répondre.
 */
let closeGuard: (() => boolean) | null = null;

function forceClose() {
  closeGuard = null;
  popinWriter.set(initialState);
}

export const popinStore = {
  subscribe: popinWriter.subscribe,
  open: (
    title: string,
    component: AnyComponent,
    props: Record<string, any> = {},
    options: PopinOptions = {}
  ) => {
    closeGuard = null;
    popinWriter.set({
      isOpen: true,
      title,
      component,
      props,
      size: options.size ?? "md",
      flush: options.flush ?? false,
      icon: options.icon ?? null
    });
  },

  /**
   * Le contenu déclare un veto. `fn` renvoie `false` pour refuser la fermeture
   * — à charge pour elle d'afficher pourquoi.
   * Retourne la fonction de désinscription, à rendre depuis un `$effect`.
   */
  guard: (fn: () => boolean) => {
    closeGuard = fn;
    return () => {
      if (closeGuard === fn) closeGuard = null;
    };
  },

  /** Fermeture **demandée par l'utilisateur** : le contenu peut la refuser. */
  requestClose: () => {
    if (closeGuard && !closeGuard()) return;
    forceClose();
  },

  /** Fermeture **décidée par le code** (enregistrement réussi…) : sans veto. */
  close: forceClose
};
