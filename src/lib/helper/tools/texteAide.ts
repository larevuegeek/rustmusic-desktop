import { estTerme, type TermeId } from "$lib/config/glossaire";

// Balisage des traductions : « [[dac|convertisseur]] » marque un terme expliqué.
const BALISE = /\[\[([a-z_]+)\|([^\]]+)\]\]/g;

export type Morceau = { texte: string; terme?: TermeId };

/** Découpe un texte traduit en morceaux simples et termes expliqués. */
export function decouper(texte: string): Morceau[] {
  const morceaux: Morceau[] = [];
  let fin = 0;
  for (const m of texte.matchAll(BALISE)) {
    if (m.index > fin) morceaux.push({ texte: texte.slice(fin, m.index) });
    morceaux.push(estTerme(m[1]) ? { texte: m[2], terme: m[1] } : { texte: m[2] });
    fin = m.index + m[0].length;
  }
  if (fin < texte.length) morceaux.push({ texte: texte.slice(fin) });
  return morceaux;
}

/** Le texte sans balisage, pour la recherche et les attributs. */
export const texteBrut = (texte: string) => texte.replace(BALISE, "$2");
