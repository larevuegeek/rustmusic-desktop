<script lang="ts">
  import { t } from "$lib/i18n";
  import { fade, scale } from "svelte/transition";
  import CoverImg from "$lib/components/ui/image/CoverImg.svelte";

  import type { Snippet } from "svelte";

  let {
    src = "",
    path = "",
    alt = "Image",
    width = 96,
    children,
  }: {
    src?: string;
    path?: string;
    alt?: string;
    width?: number;
    children?: Snippet;
  } = $props();

  let coverOpen = $state(false);

  function openCover() { coverOpen = true; }
  function closeCover() { coverOpen = false; }

  function portal(node: HTMLElement) {
    document.body.appendChild(node);
    return {
      destroy() {
        if (node.parentNode) node.parentNode.removeChild(node);
      }
    };
  }
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && coverOpen && closeCover()} />

<button type="button" class="cursor-zoom-in" onclick={openCover}>
  {#if children}
    {@render children()}
  {:else}
    <img
      src={src}
      {alt}
      loading="lazy"
      class="rounded"
      {width}
    />
  {/if}
</button>

{#if coverOpen}
  <div use:portal>
    <!-- Overlay -->
    <button
      type="button"
      class="fixed inset-0 bg-black/85 backdrop-blur-xl cursor-default"
      style="z-index: 9998;"
      aria-label={$t("common.close")}
      onclick={closeCover}
      transition:fade={{ duration: 150 }}
    ></button>

    <!-- Image plein écran -->
    <div class="fixed inset-0 grid place-items-center p-8 pointer-events-none"
         style="z-index: 9999;">
      <button
        type="button"
        class="pointer-events-auto cursor-zoom-out"
        onclick={closeCover}
        transition:scale={{ duration: 200, start: 0.92 }}
      >
        {#if path}
          <CoverImg
            {path}
            alt="Cover agrandie"
            size="full"
            class="max-w-[80vw] max-h-[80vh] w-auto h-auto rounded-2xl shadow-2xl shadow-black/50 border border-white/10"
          />
        {:else}
          <img
            src={src}
            alt="Cover agrandie"
            class="max-w-[80vw] max-h-[80vh] w-auto h-auto rounded-2xl shadow-2xl shadow-black/50 border border-white/10"
          />
        {/if}
      </button>
    </div>
  </div>
{/if}
