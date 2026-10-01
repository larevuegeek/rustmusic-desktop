/**
 * Teinte OKLCH dominante d'une pochette et son intensité (0 = gris, 1 = franche),
 * lue sur une vignette 32×32 : les pixels vifs comptent plus que les ternes.
 */
export type CouleurPochette = { h: number; s: number };

const cache = new Map<string, CouleurPochette | null>();

// sRGB 0–255 → OKLab (a, b) et clarté L.
function oklab(r: number, g: number, b: number): [number, number, number] {
  const lin = (c: number) => {
    c /= 255;
    return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  };
  const [R, G, B] = [lin(r), lin(g), lin(b)];
  const l = Math.cbrt(0.4122214708 * R + 0.5363325363 * G + 0.0514459929 * B);
  const m = Math.cbrt(0.2119034982 * R + 0.6806995451 * G + 0.1073969566 * B);
  const s = Math.cbrt(0.0883024619 * R + 0.2817188376 * G + 0.6299787005 * B);
  return [
    0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s,
    1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s,
    0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s,
  ];
}

export async function couleurPochette(src: string): Promise<CouleurPochette | null> {
  if (cache.has(src)) return cache.get(src)!;
  let resultat: CouleurPochette | null = null;
  try {
    const img = new Image();
    // `asset://` renvoie l'en-tête CORS de la fenêtre : le canvas reste lisible.
    img.crossOrigin = "anonymous";
    img.src = src;
    await img.decode();

    const taille = 32;
    const canvas = document.createElement("canvas");
    canvas.width = canvas.height = taille;
    const ctx = canvas.getContext("2d", { willReadFrequently: true });
    if (!ctx) return null;
    ctx.drawImage(img, 0, 0, taille, taille);
    const px = ctx.getImageData(0, 0, taille, taille).data;

    let sa = 0, sb = 0, poids = 0;
    for (let i = 0; i < px.length; i += 4) {
      const [L, a, b] = oklab(px[i], px[i + 1], px[i + 2]);
      // Noirs et blancs n'ont pas de teinte fiable.
      if (L < 0.18 || L > 0.96) continue;
      const c = Math.hypot(a, b);
      const w = c * c;
      sa += a * w;
      sb += b * w;
      poids += w;
    }
    const n = px.length / 4;
    if (poids > 0) {
      const h = ((Math.atan2(sb, sa) * 180) / Math.PI + 360) % 360;
      // Chroma moyen ramené à 0–1 : 0,1 en OKLCH est déjà une couleur franche.
      const chroma = Math.hypot(sa, sb) / poids;
      const couverture = Math.min(1, (poids / n) / 0.004);
      resultat = { h: Math.round(h), s: Math.min(1, (chroma / 0.1) * couverture) };
    } else {
      resultat = { h: 0, s: 0 };
    }
  } catch {
    resultat = null;
  }
  cache.set(src, resultat);
  return resultat;
}
