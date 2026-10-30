<script lang="ts">
  // Une pochette en plein, ou une mosaïque : 2 côte à côte, 3 (une grande et deux petites), 4 en carré.
  import CoverImg from "./CoverImg.svelte";

  let { covers, class: classes = "", imgClass = "" }: { covers: string[]; class?: string; imgClass?: string } = $props();

  const shown = $derived(covers.slice(0, 4));
  const grid = $derived(
    shown.length === 2 ? "grid-cols-2" : shown.length >= 3 ? "grid-cols-2 grid-rows-2" : "",
  );
</script>

<span class="grid gap-px bg-(--rg-bd) {grid} {classes}">
  {#each shown as cover, i (cover)}
    <span class="relative min-h-0 overflow-hidden bg-(--rg-s2) {shown.length === 3 && i === 0 ? 'row-span-2' : ''}">
      <CoverImg path={cover} size={shown.length === 1 ? "2x" : "1x"} class="absolute inset-0 w-full h-full object-cover {imgClass}" />
    </span>
  {/each}
</span>
