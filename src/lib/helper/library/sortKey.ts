/** Clé de tri sans accents ni article de tête : « The Doors » se range à D, « L'essentiel » à E. */
export function cleTri(texte: string | null | undefined): string {
  // L'article seulement s'il est un mot : « Léa » et « The-Dream » gardent leur initiale.
  return (texte ?? "").normalize("NFD").replace(/[̀-ͯ]/g, "").trim().replace(/^(?:(?:the|le|la|les)\s+|l')/i, "").toLowerCase();
}

/** Lettre du rail A–Z ; chiffres et ponctuation sous « # ». */
export function lettreTri(texte: string | null | undefined): string {
  const c = cleTri(texte).charAt(0).toUpperCase();
  return /[A-Z]/.test(c) ? c : "#";
}

/** Comparaison « naturelle » : « 2-13 » avant « 10-13 », sans tenir compte de la casse ni des accents. */
export const comparerNaturel = new Intl.Collator(undefined, { numeric: true, sensitivity: "base" });

type PisteOrdonnable = { disc_number: number | null; track_number: number | null; filename: string };

/** Ordre d'un disque : disque, numéro, puis nom de fichier quand les tags n'ont pas de numéro. */
export function ordreDisque(x: PisteOrdonnable, y: PisteOrdonnable): number {
  return (x.disc_number || 1) - (y.disc_number || 1) || (x.track_number ?? 1e4) - (y.track_number ?? 1e4) || comparerNaturel.compare(x.filename, y.filename);
}
