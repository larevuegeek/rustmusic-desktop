/**
 * Tire au hasard `taille` éléments parmi `items`, sans répétition ni toucher à
 * l'original. Fisher-Yates partiel : chances égales, contrairement à un tri par
 * `Math.random() - 0.5`.
 */
export function echantillonAleatoire<T>(items: T[], taille: number): T[] {
  const copie = [...items];
  const combien = Math.min(taille, copie.length);

  for (let i = 0; i < combien; i++) {
    const j = i + Math.floor(Math.random() * (copie.length - i));
    [copie[i], copie[j]] = [copie[j], copie[i]];
  }

  return copie.slice(0, combien);
}
