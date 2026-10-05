<script lang="ts">
  // Badge de qualité en police mono : or Hi-Res, vert sans perte, neutre avec perte.
  import type { PalierPiste } from "#lib/helper/tools/audioFormatTools";

  let { palier, texte, mono = true, class: classes = "" }: { palier: PalierPiste; texte: string; mono?: boolean; class?: string } = $props();

  const TEINTES: Record<PalierPiste, string> = {
    hires: "text-amber-700 border-amber-300 bg-amber-50 dark:text-[#e8c46a] dark:border-[#4a3d1c] dark:bg-[#231d0e]",
    lossless: "text-emerald-700 border-(--rg-gbd) bg-(--rg-gbg) dark:text-[#6ee7a0] dark:bg-[#0f2016]",
    lossy: "text-(--rg-mu) border-(--rg-bd) bg-(--rg-carte)",
  };
</script>

<span class="whitespace-nowrap text-[11px] rounded-md border {mono ? 'font-mono normal-case tracking-[0.02em]' : 'font-bold uppercase tracking-[0.06em]'} {TEINTES[palier]} {classes}">{texte}</span>
