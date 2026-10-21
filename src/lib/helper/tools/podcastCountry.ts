// Pays de l'annuaire et du classement des podcasts : celui choisi, sinon celui de l'ordinateur.

/** Boutiques Apple Podcasts proposées, par langue de l'appli d'abord. */
const PODCAST_COUNTRIES = [
  "FR", "BE", "CH", "CA", "LU",
  "US", "GB", "IE", "AU", "NZ",
  "DE", "AT",
  "ES", "MX", "AR",
  "IT",
  "PT", "BR", "NL", "SE", "NO", "DK", "FI", "PL", "JP", "IN",
];

/** Pays par défaut d'une langue de l'appli, quand l'ordinateur ne dit rien d'utile. */
const BY_LANGUAGE: Record<string, string> = { fr: "FR", en: "US", de: "DE", es: "ES", it: "IT" };

/**
 * Pays retenu : le réglage s'il y en a un ; sinon la région de l'ordinateur si sa langue est
 * celle de l'appli (un Anglais a « en-GB », un Québécois « fr-CA ») ; sinon le pays de la langue.
 */
export function podcastCountry(setting: string, appLanguage: string): string {
  if (PODCAST_COUNTRIES.includes(setting)) return setting;
  for (const tag of navigator.languages ?? [navigator.language]) {
    const [lang, region] = tag.split("-");
    const r = region?.toUpperCase();
    if (lang?.toLowerCase() === appLanguage && r && PODCAST_COUNTRIES.includes(r)) return r;
  }
  return BY_LANGUAGE[appLanguage] ?? "US";
}

// Noms écrits en entier : le plugin d'icônes ne repère que les littéraux (voir vite-icones.js).
const FLAGS: Record<string, string> = {
  FR: "circle-flags:fr", BE: "circle-flags:be", CH: "circle-flags:ch", CA: "circle-flags:ca", LU: "circle-flags:lu",
  US: "circle-flags:us", GB: "circle-flags:gb", IE: "circle-flags:ie", AU: "circle-flags:au", NZ: "circle-flags:nz",
  DE: "circle-flags:de", AT: "circle-flags:at", ES: "circle-flags:es", MX: "circle-flags:mx", AR: "circle-flags:ar",
  IT: "circle-flags:it", PT: "circle-flags:pt", BR: "circle-flags:br", NL: "circle-flags:nl", SE: "circle-flags:se",
  NO: "circle-flags:no", DK: "circle-flags:dk", FI: "circle-flags:fi", PL: "circle-flags:pl", JP: "circle-flags:jp",
  IN: "circle-flags:in",
};

/** Choix du pays pour un menu : « Automatique (France) » puis chaque pays, avec son drapeau. */
export function countryOptions(locale: string, autoLabel: string): { value: string; label: string; icon?: string }[] {
  const auto = podcastCountry("", locale);
  return [
    { value: "", label: autoLabel.replace("{country}", countryName(auto, locale)), icon: FLAGS[auto] },
    ...PODCAST_COUNTRIES.map((c) => ({ value: c, label: countryName(c, locale), icon: FLAGS[c] })),
  ];
}

/** Nom du pays dans la langue de l'appli (« Royaume-Uni », « United Kingdom »…). */
export function countryName(code: string, locale: string): string {
  try {
    return new Intl.DisplayNames([locale], { type: "region" }).of(code) ?? code;
  } catch {
    return code;
  }
}
