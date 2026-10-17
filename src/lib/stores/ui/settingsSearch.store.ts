import { writable } from "svelte/store";

/** Texte du champ « Chercher un réglage ». */
export const rechercheReglages = writable("");

const fold = (s: string) => s.normalize("NFD").replace(/[̀-ͯ]/g, "").toLowerCase();

/** Vrai si la recherche est vide ou apparaît dans l'un des textes, accents ignorés. */
export function matches(recherche: string, ...textes: (string | undefined)[]): boolean {
  const q = fold(recherche.trim());
  return !q || fold(textes.filter(Boolean).join(" ")).includes(q);
}
