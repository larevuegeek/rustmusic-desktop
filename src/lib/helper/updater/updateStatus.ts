import type { UpdateState } from "#lib/stores/updater/updater.store";

type Verification = "jamais" | "a-jour" | "maj" | "echec";

/** Où en sont les mises à jour, en une ligne : partagé par la carte version et « À propos ». */
export function libelleMiseAJour(t: (cle: string) => string, etat: UpdateState, derniere: Verification): string {
  switch (etat.kind) {
    case "checking": return t("settings.update_checking");
    case "available": return t("settings.update_available").replace("{version}", etat.version);
    case "downloading": {
      const pct = etat.total ? Math.round((etat.downloaded / etat.total) * 100) : null;
      return t("settings.update_downloading") + (pct !== null ? ` ${pct} %` : "");
    }
    case "installing": return t("settings.update_installing");
    case "ready": return t("settings.update_ready");
    case "error": return t("settings.update_failed");
    default:
      return derniere === "a-jour" ? t("settings.update_latest")
        : derniere === "maj" ? t("settings.update_pending")
        : derniere === "echec" ? t("settings.update_failed")
        : t("settings.update_unchecked");
  }
}
