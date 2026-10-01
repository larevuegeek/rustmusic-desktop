/** Une teinte stable (0–359) pour un texte : même album, même couleur. */
export function teinte(texte: string | null | undefined): number {
  let h = 0;
  for (const c of texte ?? '') h = (h * 31 + c.charCodeAt(0)) >>> 0;
  return h % 360;
}
