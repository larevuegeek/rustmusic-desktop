import { writable } from 'svelte/store';

/** Bouton principal fourni par une page (« Tout lire », « Mix aléatoire ») à la place d'« Ajouter un dossier ». */
export type ActionEntete = { cle: string; icone: string; lancer: () => Promise<void> };
/** Un chiffre de l'en-tête : la valeur et les clés i18n du mot (singulier, pluriel). */
export type ChiffreEntete = { n: number; un: string; plusieurs: string };

/** Ce qu'une page ajoute à l'en-tête de bibliothèque ; à remettre à `null` en la quittant. */
export const libraryHeader = writable<{
  action?: ActionEntete | null;
  /** Chiffres propres à la page ; sinon ceux de la bibliothèque. */
  chiffres?: ChiffreEntete[] | null;
}>({});
