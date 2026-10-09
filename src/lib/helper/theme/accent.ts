// Couleur d'accent : une palette de 11 nuances calculée en OKLCH depuis la couleur choisie.
// Le niveau 500 est la couleur elle-même ; vide ou vert d'origine = palette d'origine.

export const DEFAULT_ACCENT = "#22c55e";
export const ACCENT_STORAGE_KEY = "rustmusic:accent";

export const ACCENT_PRESETS = [
  { hex: "#22c55e", labelKey: "settings.accent_green" },
  { hex: "#14b8a6", labelKey: "settings.accent_teal" },
  { hex: "#06b6d4", labelKey: "settings.accent_cyan" },
  { hex: "#3b82f6", labelKey: "settings.accent_blue" },
  { hex: "#6366f1", labelKey: "settings.accent_indigo" },
  { hex: "#a855f7", labelKey: "settings.accent_purple" },
  { hex: "#ec4899", labelKey: "settings.accent_pink" },
  { hex: "#ef4444", labelKey: "settings.accent_red" },
  { hex: "#f97316", labelKey: "settings.accent_orange" },
  { hex: "#f59e0b", labelKey: "settings.accent_amber" },
];

const SHADES = ["50", "100", "200", "300", "400", "500", "600", "700", "800", "900", "950"] as const;
// Écarts de clarté et de chroma autour du 500, relevés sur la palette verte de Tailwind.
const LIGHTNESS_DELTA = [0.259, 0.239, 0.202, 0.148, 0.069, 0, -0.096, -0.196, -0.275, -0.33, -0.457];
const CHROMA_RATIO = [0.08, 0.2, 0.38, 0.68, 0.95, 1, 0.89, 0.7, 0.54, 0.43, 0.3];

type Oklch = { l: number; c: number; h: number };

const toLinear = (v: number) => (v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4);
const fromLinear = (v: number) => (v <= 0.0031308 ? 12.92 * v : 1.055 * v ** (1 / 2.4) - 0.055);

function hexToOklch(hex: string): Oklch {
  const n = parseInt(hex.slice(1), 16);
  const [r, g, b] = [(n >> 16) & 255, (n >> 8) & 255, n & 255].map((v) => toLinear(v / 255));
  const l = Math.cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b);
  const m = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b);
  const s = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b);
  const L = 0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s;
  const A = 1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s;
  const B = 0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s;
  return { l: L, c: Math.hypot(A, B), h: ((Math.atan2(B, A) * 180) / Math.PI + 360) % 360 };
}

function oklchToRgb({ l, c, h }: Oklch): [number, number, number] {
  const a = c * Math.cos((h * Math.PI) / 180);
  const b = c * Math.sin((h * Math.PI) / 180);
  const L = (l + 0.3963377774 * a + 0.2158037573 * b) ** 3;
  const M = (l - 0.1055613458 * a - 0.0638541728 * b) ** 3;
  const S = (l - 0.0894841775 * a - 1.291485548 * b) ** 3;
  return [
    4.0767416621 * L - 3.3077115913 * M + 0.2309699292 * S,
    -1.2684380046 * L + 2.6097574011 * M - 0.3413193965 * S,
    -0.0041960863 * L - 0.7034186147 * M + 1.707614701 * S,
  ];
}

/** En sRGB : on baisse la chroma jusqu'à ce que la nuance soit affichable. */
function toHex(color: Oklch): string {
  let { c } = color;
  let rgb = oklchToRgb(color);
  for (let i = 0; i < 24 && rgb.some((v) => v < -0.0001 || v > 1.0001); i++) {
    c *= 0.92;
    rgb = oklchToRgb({ ...color, c });
  }
  return "#" + rgb.map((v) => Math.round(Math.min(1, Math.max(0, fromLinear(v))) * 255).toString(16).padStart(2, "0")).join("");
}

/** Variables CSS de la palette (--acc-50 … --acc-950, --acc-on). */
export function accentVariables(hex: string): Record<string, string> {
  const base = hexToOklch(hex);
  const vars: Record<string, string> = {};
  SHADES.forEach((shade, i) => {
    const d = LIGHTNESS_DELTA[i];
    // Clairs jusqu'à 0,985, sombres jusqu'à 0,2 : la palette reste ordonnée quelle que soit la couleur.
    const l = d >= 0 ? base.l + d * ((0.985 - base.l) / 0.259) : base.l + d * ((base.l - 0.2) / 0.457);
    vars[`--acc-${shade}`] = shade === "500" ? hex : toHex({ l, c: base.c * CHROMA_RATIO[i], h: base.h });
  });
  // Texte posé sur l'accent : sombre sur une couleur claire, blanc sinon (le 600 sert en thème clair).
  const l600 = hexToOklch(vars["--acc-600"]).l;
  vars["--acc-on"] = l600 >= 0.6 ? toHex({ l: 0.18, c: 0.04, h: base.h }) : "#ffffff";
  return vars;
}

export function isValidAccent(hex: string | undefined): hex is string {
  return !!hex && /^#[0-9a-f]{6}$/i.test(hex);
}

/** Applique l'accent sur <html> et le retient pour l'écran de chargement du prochain lancement. */
export function applyAccent(hex: string | undefined): void {
  const root = document.documentElement;
  const custom = isValidAccent(hex) && hex.toLowerCase() !== DEFAULT_ACCENT;
  const vars = custom ? accentVariables(hex.toLowerCase()) : {};
  for (const shade of [...SHADES, "on"]) root.style.removeProperty(`--acc-${shade}`);
  for (const [k, v] of Object.entries(vars)) root.style.setProperty(k, v);
  if (custom) root.setAttribute("data-accent", "");
  else root.removeAttribute("data-accent");
  try {
    if (custom) localStorage.setItem(ACCENT_STORAGE_KEY, JSON.stringify(vars));
    else localStorage.removeItem(ACCENT_STORAGE_KEY);
  } catch {}
}
