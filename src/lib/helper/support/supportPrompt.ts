import { lireLocal, ecrireLocal } from "#lib/helper/tools/stockage";

// Message de soutien : une seule fois par nouvelle version, jamais à la première installation,
// et plus jamais si on l'a demandé. Le moment choisi : juste après une mise à jour.
const CLE_VERSION = "rm-support-version";
const CLE_JAMAIS = "rm-support-never";

/** À appeler une fois au lancement : vrai si le message doit s'afficher pour cette version. */
export function messageSoutienAttendu(version: string): boolean {
  if (lireLocal(CLE_JAMAIS, null) === "1") return false;
  const precedente = lireLocal(CLE_VERSION, null);
  // Première installation : on retient la version, sans rien demander.
  if (precedente === null) {
    ecrireLocal(CLE_VERSION, version);
    return false;
  }
  return precedente !== version;
}

/** Le message a été vu (soutien, plus tard) : il ne revient qu'à la prochaine version. */
export function messageSoutienVu(version: string) {
  ecrireLocal(CLE_VERSION, version);
}

export function messageSoutienJamais(version: string) {
  ecrireLocal(CLE_JAMAIS, "1");
  ecrireLocal(CLE_VERSION, version);
}
