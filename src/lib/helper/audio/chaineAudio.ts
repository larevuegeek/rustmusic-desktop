import type { PipelineMode } from "$lib/stores/player/playbackPipeline.store";

/** Teintes des pastilles du lecteur, indépendantes de la pochette. */
export const TEINTE = {
  vert: "bg-emerald-500/12 border-emerald-600/25 text-emerald-800 dark:border-emerald-400/25 dark:text-emerald-300",
  violet: "bg-violet-500/12 border-violet-600/25 text-violet-800 dark:border-violet-400/30 dark:text-violet-300",
  ambre: "bg-amber-500/14 border-amber-600/30 text-amber-800 dark:border-amber-400/25 dark:text-amber-300",
  bleu: "bg-sky-500/12 border-sky-600/25 text-sky-800 dark:border-sky-400/25 dark:text-sky-300",
  neutre: "bg-(--lc-survol) border-(--lc-trait) text-(--lc-tx2)",
};

/** État de la chaîne : vert intact, violet DSD natif, ambre modifié. */
export const CHAINE: Record<PipelineMode, { cle: string; desc: string; teinte: string; point: string }> = {
  "bit-perfect": { cle: "pipeline.badge_bit_perfect", desc: "pipeline.bit_perfect_desc", teinte: TEINTE.vert, point: "bg-emerald-500 dark:bg-emerald-400" },
  dop: { cle: "pipeline.badge_dop", desc: "pipeline.dop_desc", teinte: TEINTE.violet, point: "bg-violet-500 dark:bg-violet-400" },
  resampled: { cle: "pipeline.badge_resampled", desc: "pipeline.resampled_desc", teinte: TEINTE.ambre, point: "bg-amber-500 dark:bg-amber-400" },
  dsd: { cle: "pipeline.badge_dsd_pcm", desc: "pipeline.dsd_pcm_desc", teinte: TEINTE.ambre, point: "bg-amber-500 dark:bg-amber-400" },
  shared: { cle: "pipeline.badge_shared", desc: "pipeline.shared_desc", teinte: TEINTE.neutre, point: "bg-(--lc-mu2)" },
};
