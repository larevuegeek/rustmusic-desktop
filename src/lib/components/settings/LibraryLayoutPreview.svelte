<script lang="ts">
  // Schéma de la fenêtre : où apparaissent les sections de la bibliothèque (en vert).
  import type { PlacementOnglets } from "#lib/config/libraryTabs";

  let { placement }: { placement: PlacementOnglets } = $props();

  const vide = "rounded bg-[#ebe7df] dark:bg-[#1b211e]";
  const vert = "rounded bg-(--rg-gbg) border border-(--rg-gbd)";
</script>

<div
  class="h-17.5 grid grid-cols-[32%_1fr] gap-1 p-1.25 rounded-[7px] border
         bg-[#f6f4ef] border-[#e2ddd3] dark:bg-[#0b0d0c] dark:border-[#1f2522]
         {placement === 'sidebar' ? '' : 'grid-rows-[12px_1fr]'}"
  aria-hidden="true"
>
  <span class="{placement === 'top' ? vide : vert} {placement === 'sidebar' ? '' : 'row-span-2'}"></span>
  {#if placement !== "sidebar"}<span class={vert}></span>{/if}
  <span class={vide}></span>
</div>
