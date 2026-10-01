import { writable } from "svelte/store";

/** Texte du champ « Chercher un réglage ». */
export const rechercheReglages = writable("");

const plier = (s: string) => s.normalize("NFD").replace(/[̀-ͯ]/g, "").toLowerCase();

/** Vrai si la recherche est vide ou apparaît dans l'un des textes, accents ignorés. */
export function correspond(recherche: string, ...textes: (string | undefined)[]): boolean {
  const q = plier(recherche.trim());
  return !q || plier(textes.filter(Boolean).join(" ")).includes(q);
}
