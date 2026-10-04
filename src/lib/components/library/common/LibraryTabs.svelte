<script lang="ts">
import { goto } from "$app/navigation";
import { page } from "$app/state";
import Icon from "@iconify/svelte";
import { t } from "$lib/i18n";
import { settingsStore } from "$lib/stores/settings/settings.store";
import { sidebarStore } from "$lib/stores/ui/sidebar.store";
import {
  lienOnglet,
  lireOnglets,
  memoriserOnglet,
  ongletCourant,
  resoudreOnglets,
  type LibraryTabKey,
} from "$lib/config/libraryTabs";

// Accueil puis les sections cochées ; sans bibliothèque, Playlists seule (elle n'en dépend pas).
let { libraryId }: { libraryId: number | null } = $props();

const accueil = $derived(page.url.pathname === "/");
const courant = $derived(ongletCourant(page.url.pathname));
const tabs = $derived(
  resoudreOnglets(lireOnglets($settingsStore.library_tabs), courant).filter((o) => libraryId != null || o.key === "playlists")
);

function navigateTab(key: LibraryTabKey) {
  if (libraryId != null) memoriserOnglet(libraryId, key);
  goto(libraryId == null ? "/playlists" : lienOnglet(libraryId, key));
  sidebarStore.close(); // sur mobile elle recouvre la page
}

const onglet = "relative flex items-center gap-1.5 md:gap-2 px-3 md:px-4 py-1.5 md:py-2 rounded-full text-xs md:text-sm font-medium transition-all duration-200 cursor-pointer whitespace-nowrap";
const actif = "bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 shadow-[0_0_0_1px_rgba(16,185,129,0.25)]";
const inactif = "text-neutral-500 hover:text-neutral-800 dark:hover:text-neutral-200 hover:bg-neutral-100 dark:hover:bg-neutral-800/60";
</script>

<!-- Marge de 2 px : le défilement coupait le liseré de l'onglet actif au bord gauche. -->
<div class="flex items-center gap-1 md:gap-2 overflow-x-auto scrollbar-none min-w-0 p-0.5">
  <!-- L'accueil d'abord : en mode « en haut », la barre latérale n'a plus de tuiles. -->
  <button onclick={() => { goto("/"); sidebarStore.close(); }} class="{onglet} {accueil ? actif : inactif}" aria-current={accueil ? "page" : undefined}>
    <Icon icon="material-symbols:home-outline-rounded" width={14} />
    <span class="hidden sm:inline">{$t("nav.home")}</span>
  </button>
  {#if tabs.length > 0}<span class="shrink-0 w-px h-4 bg-neutral-300/70 dark:bg-white/10" aria-hidden="true"></span>{/if}
  {#each tabs as tab (tab.key)}
    <button
      onclick={() => navigateTab(tab.key)}
      class="{onglet} {courant === tab.key ? actif : inactif}"
      aria-current={courant === tab.key ? "page" : undefined}
    >
      <Icon icon={tab.icon} width={14} />
      <span class="hidden sm:inline">{$t(tab.labelKey)}</span>

      {#if courant === tab.key}
        <span class="absolute inset-0 rounded-full bg-emerald-500/5 pointer-events-none"></span>
      {/if}
    </button>
  {/each}
</div>
