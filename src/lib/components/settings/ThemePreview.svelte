<script lang="ts">
  // Miniature de l'application dans un thème ; « auto » coupe l'écran en biais, clair sur sombre.
  let { mode }: { mode: "auto" | "light" | "dark" } = $props();

  type Palette = { sb: string; ct: string; pl: string; plbd: string; barre: string; hero: string };
  const palettes: Record<"light" | "dark", Palette> = {
    dark: { sb: "#0f1211", ct: "#0b0d0c", pl: "#141816", plbd: "#222", barre: "#2a312d", hero: "oklch(0.32 0.07 25)" },
    light: { sb: "#efece5", ct: "#f6f4ef", pl: "#ffffff", plbd: "#e2ddd3", barre: "#d8d3c8", hero: "oklch(0.9 0.04 25)" },
  };
</script>

{#snippet ecran(p: Palette, complet: boolean)}
  <div class="absolute inset-0 grid grid-cols-[30%_1fr] grid-rows-[1fr_16px]">
    <div class="flex flex-col gap-1 px-1.5 py-2" style:background={p.sb}>
      <i class="block h-1.25 w-3/5 rounded-[3px] bg-[#22c55e]"></i>
      <i class="block h-1.25 rounded-[3px]" style:background={p.barre}></i>
      <i class="block h-1.25 rounded-[3px]" style:background={p.barre}></i>
    </div>
    <div class="flex flex-col gap-1.25 p-2" style:background={p.ct}>
      <div class="h-7 shrink-0 rounded-[5px]" style:background={p.hero}></div>
      <i class="block h-1.25 rounded-[3px]" style:background={p.barre}></i>
      {#if complet}<i class="block h-1.25 w-[70%] rounded-[3px]" style:background={p.barre}></i>{/if}
    </div>
    <div class="col-span-2 border-t" style:background={p.pl} style:border-color={p.plbd}></div>
  </div>
{/snippet}

<div class="relative h-24 rounded-lg overflow-hidden ring-1 ring-(--rg-bd) dark:ring-0" aria-hidden="true">
  {#if mode === "auto"}
    {@render ecran(palettes.dark, false)}
    <div class="absolute inset-0 [clip-path:polygon(0_0,55%_0,45%_100%,0_100%)]">
      {@render ecran(palettes.light, false)}
    </div>
  {:else}
    {@render ecran(palettes[mode], true)}
  {/if}
</div>
