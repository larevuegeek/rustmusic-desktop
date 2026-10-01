// Le jeu complet des tags d'un fichier (colonne `tags`, JSON) : champs connus et tags personnalisés.
export type TagsFichier = { champs: Record<string, unknown>; perso: Map<string, string> };

export function lireTagsFichier(json: string | null | undefined): TagsFichier | null {
  if (!json) return null;
  try {
    const champs = JSON.parse(json) as Record<string, unknown>;
    const brut = (champs.custom_tags ?? []) as [string, string][];
    return { champs, perso: new Map(brut.map(([k, v]) => [String(k).toUpperCase(), String(v)])) };
  } catch {
    return null;
  }
}
