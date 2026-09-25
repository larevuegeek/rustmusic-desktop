<script lang="ts">
import LibraryViewControls from "$lib/components/library/common/LibraryViewControls.svelte";

// Les sections vivent soit dans la barre latérale, soit dans l'en-tête général
// — jamais ici. Ne reste que la rangée de commandes, posée en mode « à gauche »
// où l'en-tête n'en porte aucune.
</script>

<div class="px-3 md:px-6 border-b border-neutral-200 dark:border-neutral-800">
  <div class="flex items-center justify-end py-2">
    <LibraryViewControls />
  </div>
</div>
