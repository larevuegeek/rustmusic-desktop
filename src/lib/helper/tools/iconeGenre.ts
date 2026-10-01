/**
 * Une icône qui évoque le genre. Les tags sont du texte libre (« Rap/Hip Hop »,
 * « Musiques de films », « Hark Rock »…) : on cherche des mots-clés, du plus
 * précis au plus général — la première règle qui répond l'emporte.
 */
const REGLES: [RegExp, string][] = [
  [/comedie musicale|musicals?\b|stage & screen/, "ph:mask-happy-duotone"],
  [/film|soundtrack|bande.? originale|\bscore|\bost\b|\bbo\b|games/, "ph:film-slate-duotone"],
  [/humou?r|comedy/, "ph:mask-happy-duotone"],
  [/spoken word|poesie/, "ph:megaphone-duotone"],
  [/metal|gothi|stoner|doom/, "ph:skull-duotone"],
  [/hard ?rock|hark rock/, "ph:lightning-duotone"],
  [/grunge/, "ph:drop-duotone"],
  [/punk|\bska\b/, "ph:flame-duotone"],
  [/psyche/, "ph:rainbow-duotone"],
  [/progressi/, "ph:planet-duotone"],
  [/new wave|synthwave|80s|annees 80/, "ph:cassette-tape-duotone"],
  [/\brap\b|trap/, "ph:microphone-stage-duotone"],
  [/hip.?hop/, "ph:headphones-duotone"],
  [/jazz|swing|bebop|lounge/, "ph:martini-duotone"],
  [/disco|funk/, "ph:disco-ball-duotone"],
  [/r ?& ?b|rhythm and blues|soul/, "ph:heart-duotone"],
  [/blues/, "ph:guitar-duotone"],
  [/downtempo|chill|ambient|trip.?hop|new age|relax/, "ph:cloud-moon-duotone"],
  [/house/, "ph:speaker-hifi-duotone"],
  [/dance|club/, "ph:disco-ball-duotone"],
  [/electr|techno|trance|\bedm\b|synth/, "ph:waveform-duotone"],
  [/reggae|\bdub\b/, "ph:leaf-duotone"],
  [/country|western|bluegrass/, "ph:cactus-duotone"],
  [/folk|acoustic|singer.?songwriter/, "ph:campfire-duotone"],
  [/classi(que|cal)|opera|baroque|orchest|symphon|instrumental|piano/, "ph:piano-keys-duotone"],
  [/chanson|variete|french|francais|franzosisch/, "ph:microphone-duotone"],
  [/latin|salsa|bossa|tango/, "ph:pepper-duotone"],
  [/monde|world|afro/, "ph:globe-hemisphere-west-duotone"],
  [/gospel|religi|christian|spiritu/, "ph:church-duotone"],
  [/enfant|child|kids|comptine/, "ph:balloon-duotone"],
  [/noel|christmas|xmas/, "ph:tree-evergreen-duotone"],
  [/indie|inde\b|alternati|\balt\b|college/, "ph:vinyl-record-duotone"],
  [/top ?\d+|hits/, "ph:shooting-star-duotone"],
  [/pop/, "ph:sparkle-duotone"],
  [/rock|garage/, "ph:guitar-duotone"],
];

export const ICONE_GENRE_DEFAUT = "ph:music-notes-duotone";

export function iconeGenre(nom: string | null | undefined): string {
  const n = (nom ?? "").normalize("NFD").replace(/[̀-ͯ]/g, "").toLowerCase();
  for (const [motif, icone] of REGLES) if (motif.test(n)) return icone;
  return ICONE_GENRE_DEFAUT;
}
