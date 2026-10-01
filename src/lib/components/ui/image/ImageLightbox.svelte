<script lang="ts">
  // Image en grand par-dessus une popin (z-60) ; un clic n'importe où la ferme.
  // Échap reste à la charge de l'appelant, seul à connaître l'ordre de ses couches.
  import { fade, scale } from "svelte/transition";

  let {
    src,
    alt = "",
    title = "",
    caption = "",
    onclose,
  }: {
    src: string;
    alt?: string;
    title?: string;
    caption?: string;
    onclose: () => void;
  } = $props();
</script>

<div
  role="presentation"
  class="fixed inset-0 z-60 flex flex-col items-center justify-center gap-3 p-10
         bg-black/88 backdrop-blur-md cursor-zoom-out"
  transition:fade={{ duration: 120 }}
  onclick={onclose}
>
  <img
    {src}
    {alt}
    class="max-w-full max-h-[72vh] object-contain rounded-xl shadow-2xl ring-1 ring-white/10"
    transition:scale={{ duration: 140, start: 0.94 }}
  />
  {#if title || caption}
    <div class="text-center">
      {#if title}
        <p class="text-sm font-medium text-white/90">{title}</p>
      {/if}
      {#if caption}
        <p class="text-[11px] text-white/50">{caption}</p>
      {/if}
    </div>
  {/if}
</div>
