<script lang="ts">
import LibraryTabs from "$lib/components/library/common/LibraryTabs.svelte";
import LibraryViewControls from "$lib/components/library/common/LibraryViewControls.svelte";

// Posée en mode « à gauche » et « les deux » seulement. En mode « en haut »,
// sections et commandes passent dans l'en-tête général.
let { libraryId, avecOnglets = true }: { libraryId: number; avecOnglets?: boolean } = $props();
</script>

<div class="px-3 md:px-6 border-b border-neutral-200 dark:border-neutral-800">
  <div class="flex items-center justify-between py-2">
    {#if avecOnglets}
      <LibraryTabs {libraryId} />
    {:else}
      <div class="min-w-0 flex-1"></div>
    {/if}

    <div class="ml-2">
      <LibraryViewControls />
    </div>
  </div>
</div>
