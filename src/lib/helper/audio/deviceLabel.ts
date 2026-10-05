import { maxSampleRate, type AudioDeviceInfo, type WasapiDeviceCapabilities } from "#lib/stores/audio/audioDevices.store";

export type LibelleSortie = { nom: string; detail: string; icone: string };

// Windows met un rôle générique devant l'appareil : « Haut-parleurs (Fosi Audio K7) ».
const ROLE = /^(haut-parleurs?|speakers?|casques?( audio)?|headphones?|headset|écouteurs|earphones|lautsprecher|kopfhörer|altavoces|auriculares|altoparlanti|cuffie|auricolari)$/i;
const CASQUE = /casque|headphone|headset|écouteur|earphone|kopfhörer|auricular|cuffi|auricolar/i;
const ECRAN = /hdmi|displayport|(nvidia|amd|ati|intel\(r\)) (high definition|display) audio/i;
const NUMERIQUE = /digital output|s\/?pdif|optical|optique/i;

/** « Rôle (Appareil) » → [rôle, appareil], parenthèses internes comprises (« Realtek(R) Audio »). */
function scinder(nom: string): [string, string | null] {
  if (!nom.endsWith(")")) return [nom, null];
  let profondeur = 0;
  for (let i = nom.length - 1; i >= 0; i--) {
    if (nom[i] === ")") profondeur++;
    else if (nom[i] === "(" && --profondeur === 0) return [nom.slice(0, i).trim(), nom.slice(i + 1, -1).trim()];
  }
  return [nom, null];
}

// Un nom brut suffit (celui que renvoie la lecture) ; bus et pilote affinent le détail.
type SortieDecrite = Pick<AudioDeviceInfo, "name"> & Partial<Pick<AudioDeviceInfo, "bus" | "manufacturer" | "driver">>;

function connexion(d: SortieDecrite, texte: string): string | null {
  if (ECRAN.test(texte)) return "HDMI";
  if (NUMERIQUE.test(texte)) return "S/PDIF";
  const bus = d.bus?.toUpperCase() ?? "";
  if (bus.startsWith("BTH")) return "Bluetooth";
  if (bus === "USB" || /usb/i.test(texte)) return "USB";
  return null;
}

/** Nom lisible d'une sortie, sa ligne de détail (rôle ou pilote · connexion) et son icône. */
export function decrireSortie(d: SortieDecrite): LibelleSortie {
  let [avant, dedans] = scinder(d.name);
  // ALSA : « Fosi Audio K7, USB Audio ».
  if (!dedans && d.name.includes(", ")) {
    const i = d.name.indexOf(", ");
    [avant, dedans] = [d.name.slice(0, i), d.name.slice(i + 2)];
  }
  const appareil = dedans?.replace(/^\d+-\s*/, "") ?? null;
  const role = !!appareil && ROLE.test(avant);
  const detail = (role ? avant : appareil) ?? d.manufacturer ?? d.driver ?? "";
  const lien = connexion(d, d.name);

  return {
    nom: role ? appareil! : avant,
    detail: lien && !detail.toLowerCase().includes(lien.toLowerCase()) ? [detail, lien].filter(Boolean).join(" · ") : detail,
    icone: ECRAN.test(d.name)
      ? "material-symbols:tv-outline-rounded"
      : CASQUE.test(avant)
        ? "material-symbols:headphones-outline-rounded"
        : "material-symbols:speaker-outline-rounded",
  };
}

/** Plus grande profondeur exposée : « 32-bit float » → 32. */
export function profondeurMax(formats: string[]): number | null {
  const bits = formats.map((f) => parseInt(f, 10)).filter((n) => !Number.isNaN(n));
  return bits.length ? Math.max(...bits) : null;
}

export type Palier = "cd" | "hires" | "ultra";

/** Palier de qualité d'une sortie ; sous 24 bits, elle reste « CD » quelle que soit sa fréquence. */
export function palierQualite(taux: number | null, bits: number | null): Palier | null {
  if (!taux) return null;
  if (taux < 88_200 || (bits !== null && bits < 24)) return "cd";
  return taux >= 352_800 ? "ultra" : "hires";
}

/** Badge de chaque palier : mêmes couleurs que les fréquences de la fenêtre de détails. */
export const PALIERS: Record<Palier, { labelKey: string; ton: "neutral" | "amber" | "purple" }> = {
  cd: { labelKey: "settings.audio_devices_rate_cd", ton: "neutral" },
  hires: { labelKey: "settings.audio_devices_rate_hires", ton: "amber" },
  ultra: { labelKey: "settings.audio_devices_rate_dsd_pcm", ton: "purple" },
};

/**
 * Ce que la sortie sait vraiment faire : l'exclusif sondé ; sans exclusif, le format du mixeur
 * Windows (la table CPAL y est la même pour tous) ; hors Windows, ce que déclare CPAL.
 */
export function capacitesSortie(d: AudioDeviceInfo, caps?: WasapiDeviceCapabilities) {
  if (caps?.exclusiveRates.length) {
    return {
      taux: Math.max(...caps.exclusiveRates),
      bits: caps.exclusiveBitDepths.length ? Math.max(...caps.exclusiveBitDepths) : null,
      canaux: d.maxChannels,
    };
  }
  if (caps?.mixRate) return { taux: caps.mixRate, bits: caps.mixBitDepth, canaux: caps.mixChannels ?? d.maxChannels };
  return { taux: maxSampleRate(d.sampleRates), bits: profondeurMax(d.sampleFormats), canaux: d.maxChannels };
}
