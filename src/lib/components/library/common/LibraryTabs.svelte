<script lang="ts">
import { goto } from "$app/navigation";
import { page } from "$app/state";
import Icon from "@iconify/svelte";
import { t } from "$lib/i18n";
import { settingsStore } from "$lib/stores/settings/settings.store";
import { sidebarStore } from "$lib/stores/ui/sidebar.store";
import {
  lireOnglets,
  memoriserOnglet,
  ongletCourant,
  resoudreOnglets,
  type LibraryTabKey,
} from "$lib/config/libraryTabs";

// La rangée seule, sans décor : elle vit dans la barre de bibliothèque et, en
// mode « en haut », dans la mise en page générale. C'est l'appelant qui décide
// de l'afficher.
let { libraryId }: { libraryId: number } = $props();

const courant = $derived(ongletCourant(page.url.pathname));
const tabs = $derived(
  resoudreOnglets(lireOnglets($settingsStore.library_tabs), courant)
);

function navigateTab(key: LibraryTabKey) {
  memoriserOnglet(libraryId, key);
  goto(`/library/${libraryId}/${key}`);
  sidebarStore.close(); // sur mobile elle recouvre la page
}
</script>

<div class="flex items-center gap-1 md:gap-2 overflow-x-auto scrollbar-none min-w-0 py-0.5">
  {#each tabs as tab (tab.key)}
    <button
      onclick={() => navigateTab(tab.key)}
      class="relative flex items-center gap-1.5 md:gap-2 px-3 md:px-4 py-1.5 md:py-2 rounded-full text-xs md:text-sm font-medium transition-all duration-200 cursor-pointer whitespace-nowrap
          {courant === tab.key
          ? 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 shadow-[0_0_0_1px_rgba(16,185,129,0.25)]'
          : 'text-neutral-500 hover:text-neutral-800 dark:hover:text-neutral-200 hover:bg-neutral-100 dark:hover:bg-neutral-800/60'}"
    >
      <Icon icon={tab.icon} width={14} />
      <span class="hidden sm:inline">{$t(tab.labelKey)}</span>

      {#if courant === tab.key}
        <span class="absolute inset-0 rounded-full bg-emerald-500/5 pointer-events-none"></span>
      {/if}
    </button>
  {/each}
</div>
