/** Termes expliqués par une bulle ; textes dans `glossary.<id>` des traductions. */
export const TERMES = [
  "dac",
  "pcm",
  "sample_rate",
  "bit_depth",
  "dsd",
  "dop",
  "shared",
  "exclusive",
  "resampling",
  "dsp",
  "replay_gain",
  "bit_perfect",
] as const;

export type TermeId = (typeof TERMES)[number];

export function estTerme(id: string): id is TermeId {
  return (TERMES as readonly string[]).includes(id);
}

/** Ancre d'un terme dans la page Comprendre le son. */
export const lienTerme = (id: TermeId) => `/settings/guide#terme-${id}`;
