/** Poids lisible : « 412 Go », « 8,5 Mo » ; unités françaises en français, anglaises ailleurs. */
export function tailleLisible(octets: number, locale: string): string {
  if (!octets) return "";
  const unites = locale.startsWith("fr") ? ["o", "Ko", "Mo", "Go", "To"] : ["B", "KB", "MB", "GB", "TB"];
  const i = Math.min(unites.length - 1, Math.floor(Math.log(octets) / Math.log(1024)));
  const v = octets / Math.pow(1024, i);
  return `${v.toLocaleString(locale, { maximumFractionDigits: v < 10 ? 1 : 0 })} ${unites[i]}`;
}
