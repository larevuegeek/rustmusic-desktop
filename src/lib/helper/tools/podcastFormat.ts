// Dates et durées des épisodes, dans la langue de l'appli.

/** Date d'un épisode (UTC « AAAA-MM-JJ HH:MM:SS ») ; l'année seulement si ce n'est pas la courante. */
export function episodeDate(published: string | null, locale: string): string {
  if (!published) return "";
  const d = new Date(published.replace(" ", "T") + "Z");
  if (Number.isNaN(d.getTime())) return "";
  const sameYear = d.getFullYear() === new Date().getFullYear();
  return d.toLocaleDateString(locale, { day: "numeric", month: "short", year: sameYear ? undefined : "numeric" });
}

/** « 1 h 02 », « 34 min », « 45 s ». */
export function episodeDuration(seconds: number | null): string {
  if (!seconds || seconds <= 0) return "";
  const s = Math.round(seconds);
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  if (h > 0) return `${h} h ${String(m).padStart(2, "0")}`;
  if (m > 0) return `${m} min`;
  return `${s} s`;
}
