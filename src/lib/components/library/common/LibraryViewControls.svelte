<script lang="ts">
  import { t } from "$lib/i18n";
import { page } from "$app/state";
import Icon from "@iconify/svelte";
import ViewModeToggle from "$lib/components/ui/input/ViewModeToggle.svelte";
import { selectionStore } from "$lib/stores/ui/selection.store";

// Extraites de la barre d'onglets pour la suivre quand elle remonte dans
// l'en-tête général : seules, elles y laissaient un bandeau vide.
const selection = $derived($selectionStore);

const segments = $derived(page.url.pathname.split("/").filter(Boolean));
const isListingPage = $derived(segments.length <= 3);
// La navigation alphabétique n'a de sens que sur une longue liste triée.
const hasAlphabetNav = $derived(
  isListingPage && ['albums', 'artists'].includes(segments[2])
);
// Albums, Morceaux et Artistes portent leurs propres commandes (vue, sélection) dans leur barre d'outils.
const pageOutillee = $derived(segments.length === 3 && ['albums', 'tracks', 'artists', 'genres', 'folders'].includes(segments[2]));
</script>

{#if !pageOutillee}
<div class="shrink-0 flex items-center gap-1.5">
  {#if !selection.active}
    <button
      type="button"
      class="flex items-center gap-1 px-2 py-1.5 rounded-lg shrink-0
             text-[11px] font-medium cursor-pointer transition-colors
             text-neutral-500 hover:text-neutral-800 dark:hover:text-neutral-200
             hover:bg-neutral-100 dark:hover:bg-white/5"
      onclick={() => selectionStore.start()}
      title={$t("common.select_multiple")}
    >
      <Icon icon="lucide:check-square" width={12} />
      <span class="hidden md:inline">Sélectionner</span>
    </button>
  {/if}

  <ViewModeToggle showAlphabet={hasAlphabetNav} />
</div>
{/if}
