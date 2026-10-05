<script lang="ts">
  // Fenêtre en miniature : boutons dans le style et du côté choisis, dessinés comme la barre de titre.
  let { style, position }: { style: "macos" | "windows" | "linux"; position: "left" | "right" } = $props();

  const glyphs = {
    minimize: "M2 6h8",
    maximize: "M2.5 2.5h7v7h-7z",
    close: "M2.5 2.5l7 7M9.5 2.5l-7 7",
  };

  // Mêmes pastilles que la barre de titre : fermer, réduire, plein écran.
  const MAC_LIGHTS = [
    { fill: "#ff5f57", glyph: "#4d0000", d: "M3.8 3.8l4.4 4.4M8.2 3.8l-4.4 4.4", filled: false },
    { fill: "#febc2e", glyph: "#985700", d: "M3 6h6", filled: false },
    { fill: "#28c840", glyph: "#006500", d: "M3 3h3.6L3 6.6zM9 9H5.4L9 5.4z", filled: true },
  ];
</script>

<div
  class="h-18 rounded-lg overflow-hidden flex flex-col border
         bg-[#f6f4ef] border-[#e2ddd3] dark:bg-[#0b0d0c] dark:border-[#1f2522]"
  aria-hidden="true"
>
  <div
    class="h-9 shrink-0 flex items-center border-b
           bg-[#efece5] border-[#e2ddd3] dark:bg-[#080a09] dark:border-[#161a18]
           {position === 'left' ? 'flex-row-reverse' : ''}"
  >
    <!-- Le logo cède la place, jamais les boutons. -->
    <span class="mx-2.5 w-8 min-w-1 h-2 shrink rounded-full bg-[#22c55e]"></span>
    <span class="flex-1"></span>

    {#if style === "macos"}
      <span class="shrink-0 flex items-center gap-1.5 px-2.5">
        {#each MAC_LIGHTS as light (light.fill)}
          <span class="w-3 h-3 rounded-full" style="background: {light.fill}; color: {light.glyph}">
            <svg viewBox="0 0 12 12" class="w-3 h-3"><path d={light.d} fill={light.filled ? "currentColor" : "none"} stroke={light.filled ? "none" : "currentColor"} stroke-width="1.1" stroke-linecap="round" /></svg>
          </span>
        {/each}
      </span>
    {:else if style === "linux"}
      <span class="shrink-0 flex items-center gap-1.5 px-2">
        {#each Object.values(glyphs) as d (d)}
          <span class="w-5 h-5 rounded-full flex items-center justify-center
                       bg-neutral-200 text-[#5e625d] dark:bg-white/10 dark:text-neutral-300">
            <svg viewBox="0 0 12 12" width="9" height="9" fill="none" stroke="currentColor" stroke-width="1.5"><path {d} /></svg>
          </span>
        {/each}
      </span>
    {:else}
      <span class="shrink-0 flex h-full">
        {#each Object.values(glyphs) as d (d)}
          <span class="w-6.5 h-full flex items-center justify-center text-[#5e625d] dark:text-neutral-300">
            <svg viewBox="0 0 12 12" width="11" height="11" fill="none" stroke="currentColor" stroke-width="1.1"><path {d} /></svg>
          </span>
        {/each}
      </span>
    {/if}
  </div>
  <div class="flex-1 flex flex-col gap-1.25 p-2">
    <span class="h-1.25 w-3/5 rounded-[3px] bg-[#d8d3c8] dark:bg-[#2a312d]"></span>
    <span class="h-1.25 w-2/5 rounded-[3px] bg-[#d8d3c8] dark:bg-[#2a312d]"></span>
  </div>
</div>
