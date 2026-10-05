/**
 * Helpers for displaying audio format / quality info in the UI.
 * DSD-specific labels live here so any list / badge / detail page
 * stays consistent.
 */
import { get } from "svelte/store";
import { t, currentLocale } from "#lib/i18n";

/** True when the audio format is one of the DSD container formats. */
export function isDsdFormat(format: string | null | undefined): boolean {
  if (!format) return false;
  const f = format.toUpperCase();
  return f === "DSF" || f === "DFF";
}

/**
 * Compute the user-facing DSD label from a sample rate in Hz.
 * 2 822 400 Hz → DSD64, 5 644 800 Hz → DSD128, etc.
 * Falls back to "DSD" when the rate is non-standard.
 */
export function dsdLabel(sampleRate: number | null | undefined): string {
  if (!sampleRate) return "DSD";
  const ratio = Math.round(sampleRate / 44100);
  if (ratio === 64) return "DSD64";
  if (ratio === 128) return "DSD128";
  if (ratio === 256) return "DSD256";
  if (ratio === 512) return "DSD512";
  if (ratio === 1024) return "DSD1024";
  return "DSD";
}

/** « 2,82 MHz » : fréquence DSD, décimales à la langue de l'appli. */
export function formatDsdRate(sampleRate: number | null | undefined): string {
  if (!sampleRate) return "?";
  return `${(sampleRate / 1_000_000).toLocaleString(get(currentLocale), { minimumFractionDigits: 2, maximumFractionDigits: 2 })} MHz`;
}

/** « Mono » / « Stéréo » / « 5.1 » / « 6 ch ». */
export function formatChannels(channels: number | null | undefined): string {
  if (channels === 1) return get(t)("units.mono");
  if (channels === 2) return get(t)("units.stereo");
  if (channels === 6) return "5.1";
  if (channels === 8) return "7.1";
  if (!channels) return "?";
  return `${channels} ch`;
}

/**
 * Friendly bitrate display.
 * Below 1000 kb/s → "320 kb/s" (typical lossy / 16-bit lossless).
 * Above or equal to 1000 kb/s → "5.6 Mb/s" (HD lossless, DSD).
 */
export function formatBitrate(kbps: number | null | undefined): string {
  if (!kbps || kbps <= 0) return "—";
  const loc = get(currentLocale);
  if (kbps >= 1000) {
    return `${(kbps / 1000).toLocaleString(loc, { minimumFractionDigits: 1, maximumFractionDigits: 1 })} Mb/s`;
  }
  return `${Math.round(kbps).toLocaleString(loc)} kb/s`;
}

/**
 * Badge Hi-Res d'un album d'après sa meilleure piste : « 24/96 », « DSD64 » ;
 * `null` en qualité CD (16 bits, 48 kHz au plus) ou inconnue.
 */
export function etiquetteHiRes(bits: number | null | undefined, frequence: number | null | undefined): string | null {
  if (bits === 1) return dsdLabel(frequence);
  if (!bits || !frequence || (bits <= 16 && frequence <= 48000)) return null;
  return `${bits}/${Math.floor(frequence / 1000)}`;
}

export type PalierPiste = "hires" | "lossless" | "lossy";
const SANS_PERTE = new Set(["FLAC", "ALAC", "WAV", "AIFF", "AIF", "APE", "WV", "WAVPACK", "TTA", "DSF", "DFF"]);

/**
 * Palier d'une piste, mêmes critères que les filtres de la base :
 * Hi-Res au-delà de 16 bits ou 48 kHz (DSD compris), sinon sans perte selon le format.
 */
export function palierPiste(format: string | null | undefined, bits: number | null | undefined, frequence: number | null | undefined): PalierPiste {
  if ((bits ?? 0) > 16 || (frequence ?? 0) > 48000) return "hires";
  return SANS_PERTE.has((format ?? "").toUpperCase()) ? "lossless" : "lossy";
}

/** « FLAC 24/96 », « DSD64 », « MP3 » : le format et, sans perte, sa résolution. */
export function formatCourt(format: string | null | undefined, bits: number | null | undefined, frequence: number | null | undefined): string {
  const f = (format ?? "").toUpperCase();
  if (bits === 1 || f === "DSF" || f === "DFF") return dsdLabel(frequence);
  if (!bits || !frequence) return f;
  const khz = frequence % 1000 ? (frequence / 1000).toFixed(1).replace(".", ",") : String(frequence / 1000);
  return `${f} ${bits}/${khz}`;
}

type PisteQualite = { audio_format: string | null; extension?: string | null; bits_per_sample: number | null; sample_rate: number | null; bitrate: number | null };

/** La meilleure piste d'un ensemble (DSD, puis profondeur, fréquence, débit) : palier et « FLAC 24 bit / 96 kHz ». */
export function meilleureQualite(pistes: PisteQualite[]): { palier: PalierPiste; texte: string } | null {
  const rang = (x: PisteQualite) => (x.bits_per_sample === 1 ? 1e9 : (x.bits_per_sample ?? 0) * 1e6) + (x.sample_rate ?? 0) + (x.bitrate ?? 0) / 1e4;
  let m: PisteQualite | null = null;
  for (const x of pistes) if (!m || rang(x) > rang(m)) m = x;
  if (!m) return null;
  const f = (m.audio_format ?? m.extension ?? "").toUpperCase();
  const palier = palierPiste(m.audio_format, m.bits_per_sample, m.sample_rate);
  const khz = (r: number) => (r % 1000 ? (r / 1000).toFixed(1).replace(".", ",") : String(r / 1000));
  if (m.bits_per_sample === 1 || f === "DSF" || f === "DFF") return { palier, texte: dsdLabel(m.sample_rate) };
  if (palier !== "lossy" && m.bits_per_sample && m.sample_rate) return { palier, texte: `${f} ${m.bits_per_sample} bit / ${khz(m.sample_rate)} kHz` };
  return { palier, texte: m.bitrate ? `${f} ${formatBitrate(m.bitrate)}` : f };
}
