<script lang="ts">
import Icon from "@iconify/svelte";
import { goto } from "$app/navigation";
import { page } from "$app/state";
import { t } from "#lib/i18n";
import { libraryStore } from "#lib/stores/library/library.store";
import { settingsStore } from "#lib/stores/settings/settings.store";
import { sidebarStore } from "#lib/stores/ui/sidebar.store";
import {
  lireOnglets,
  lienOnglet,
  lirePlacement,
  memoriserOnglet,
  ongletCourant,
  resoudreOnglets,
} from "#lib/config/libraryTabs";

let { replie = false }: { replie?: boolean } = $props();

type Tuile = { cle: string; label: string; icone: string; actif: boolean; aller: () => void };

const pathname = $derived(page.url.pathname);
const ouvert = $derived(ongletCourant(pathname));
const libraryId = $derived($libraryStore.librarySelected?.id ?? null);

// « Les deux » = barre latérale + onglets en haut. En haut seul, l'accueil et les
// sections sont dans la barre d'onglets : plus aucune tuile ici.
const tuiles = $derived.by((): Tuile[] => {
  if (lirePlacement($settingsStore.library_tabs_position) === "top") return [];
  const accueil: Tuile = {
    cle: "home", label: $t("nav.home"), icone: "material-symbols:home-outline-rounded",
    actif: pathname === "/", aller: () => nav("/"),
  };
  const id = libraryId;
  const onglets = resoudreOnglets(lireOnglets($settingsStore.library_tabs));

  // Sans bibliothèque, les sections mènent à l'import ; les playlists, au profil, restent accessibles.
  if (id == null) {
    const section = pathname === "/import" ? page.url.searchParams.get("section") : null;
    return [accueil, ...onglets.map((o) => ({
      cle: o.key, label: $t(o.labelKey), icone: o.icon,
      actif: o.key === "playlists" ? ouvert === o.key : section === o.key,
      aller: () => nav(o.key === "playlists" ? "/playlists" : `/import?section=${o.key}`),
    }))];
  }

  return [accueil, ...onglets.map((o) => ({
    cle: o.key, label: $t(o.labelKey), icone: o.icon, actif: ouvert === o.key,
    aller: () => { memoriserOnglet(id, o.key); nav(lienOnglet(id, o.key)); },
  }))];
});

function nav(path: string) {
  goto(path);
  sidebarStore.close();
}
</script>

{#if tuiles.length > 0}
<nav class="grid {replie ? 'grid-cols-[48px] gap-1.5 justify-center' : 'grid-cols-3 gap-1.5'}">
  {#each tuiles as tuile (tuile.cle)}
    <button
      type="button"
      class="flex flex-col items-center justify-center gap-1 rounded-xl border cursor-pointer transition-colors
             text-xs font-semibold {replie ? 'w-12 h-12' : 'h-16.5 px-1'}
             {tuile.actif
               ? 'bg-(--sb-gbg) border-(--sb-gbd) text-(--sb-g)'
               : 'bg-(--sb-s1) border-(--sb-bd) text-(--sb-tx2) hover:border-(--sb-bd2) hover:text-(--sb-tx)'}"
      onclick={tuile.aller}
      title={tuile.label}
      aria-current={tuile.actif ? "page" : undefined}
    >
      <Icon icon={tuile.icone} width="22" class="sb-icone" />
      {#if !replie}
        <span class="max-w-full truncate">{tuile.label}</span>
      {/if}
    </button>
  {/each}
</nav>
{/if}
