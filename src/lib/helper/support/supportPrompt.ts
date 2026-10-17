import { readLocal, writeLocal } from "#lib/helper/tools/storage";

// Message de soutien : une seule fois par nouvelle version, jamais à la première installation,
// et plus jamais si on l'a demandé. Le moment choisi : juste après une mise à jour.
const CLE_VERSION = "rm-support-version";
const CLE_JAMAIS = "rm-support-never";

/** À appeler une fois au lancement : vrai si le message doit s'afficher pour cette version. */
export function isSupportMessageDue(version: string): boolean {
  if (readLocal(CLE_JAMAIS, null) === "1") return false;
  const precedente = readLocal(CLE_VERSION, null);
  // Première installation : on retient la version, sans rien demander.
  if (precedente === null) {
    writeLocal(CLE_VERSION, version);
    return false;
  }
  return precedente !== version;
}

/** Le message a été vu (soutien, plus tard) : il ne revient qu'à la prochaine version. */
export function markSupportMessageSeen(version: string) {
  writeLocal(CLE_VERSION, version);
}

export function disableSupportMessage(version: string) {
  writeLocal(CLE_JAMAIS, "1");
  writeLocal(CLE_VERSION, version);
}
