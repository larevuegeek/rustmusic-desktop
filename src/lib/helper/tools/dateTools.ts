import { get } from "svelte/store";
import { t } from "#lib/i18n";


function formatTime(seconds: number): string {
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${String(mins).padStart(2, '0')}:${String(secs).padStart(2, '0')}`;
}

/** « 1:12 », « 12:05 » : minutes sans zéro devant, comme les lecteurs. */
function minutesSecondes(seconds: number): string {
    return `${Math.floor(seconds / 60)}:${String(Math.floor(seconds % 60)).padStart(2, '0')}`;
}

/** « 25 j 6 h », « 18 h 20 min », « 42 min » : une durée totale d'écoute, unités de la langue. */
function dureeEcoute(secondes: number): string {
    const u = get(t);
    const min = Math.round(secondes / 60);
    const j = Math.floor(min / 1440), h = Math.floor((min % 1440) / 60), m = min % 60;
    if (j > 0) return `${j} ${u("units.day_short")} ${h} ${u("units.hour_short")}`;
    return h > 0 ? `${h} ${u("units.hour_short")} ${String(m).padStart(2, '0')} ${u("units.min_short")}` : `${m} ${u("units.min_short")}`;
}

function dateToYear(string: string): string {

    let year = String("");

    const match = string.match(/\b(\d{4})\b/);
    if (match) {
      const y = Number(match[1]);
      year = (y >= 1000 && y <= 2999) ? String(y) : "";
    }

    return year;
}

/** « il y a 8 min », « hier »… Les dates SQLite sans fuseau sont en UTC. */
function ilYA(date: string | null | undefined, locale: string, style: "long" | "short" = "long"): string {
    if (!date) return "";
    const instant = new Date(date.includes("T") ? date : date.replace(" ", "T") + "Z").getTime();
    if (Number.isNaN(instant)) return "";
    const secondes = Math.round((instant - Date.now()) / 1000);
    const fmt = new Intl.RelativeTimeFormat(locale, { numeric: "auto", style });
    const abs = Math.abs(secondes);
    if (abs < 60) return fmt.format(0, "second");
    if (abs < 3600) return fmt.format(Math.round(secondes / 60), "minute");
    if (abs < 86400) return fmt.format(Math.round(secondes / 3600), "hour");
    if (abs < 86400 * 30) return fmt.format(Math.round(secondes / 86400), "day");
    if (abs < 86400 * 365) return fmt.format(Math.round(secondes / (86400 * 30)), "month");
    return fmt.format(Math.round(secondes / (86400 * 365)), "year");
}

/**
 * « il y a 3 min », « il y a 2 h », « il y a 4 j », via `sidebar.ago_*`.
 * Format maison : `Intl` abrège différemment selon le moteur (« 5 m. »).
 */
function depuisCourt(date: string, now: number, t: (cle: string) => string): string {
    const instant = new Date(date.includes("T") ? date : date.replace(" ", "T") + "Z").getTime();
    const s = Math.max(0, (now - instant) / 1000);
    if (Number.isNaN(s) || s < 60) return t("sidebar.just_now");
    const [cle, n]: [string, number] = s < 3600 ? ["ago_min", s / 60] : s < 86400 ? ["ago_h", s / 3600] : ["ago_d", s / 86400];
    return t(`sidebar.${cle}`).replace("{n}", String(Math.floor(n)));
}

export { formatTime, minutesSecondes, dureeEcoute, dateToYear, ilYA, depuisCourt };