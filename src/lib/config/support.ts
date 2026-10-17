// Liens de soutien du projet : section « Soutenir » d'À propos et message après une mise à jour.
export const SUPPORT_LINKS = {
  patreon: "https://patreon.com/larevuegeek",
  kofi: "https://ko-fi.com/larevuegeek",
  paypal: "https://paypal.me/larevuegeek",
  github: "https://github.com/larevuegeek/rustmusic-desktop",
} as const;

/** Section « Soutenir » du site, dans la langue de l'appli (objectif de dons et moyens de soutien). */
export function supportPageUrl(locale: string): string {
  const prefix = ["en", "es", "it", "de"].includes(locale) ? `/${locale}` : "";
  return `https://rustmusic.dev${prefix}/#support`;
}
