<script lang="ts">
import Icon from "@iconify/svelte";
import { goto } from "$app/navigation";
import { page } from "$app/state";
import { t } from "$lib/i18n";
import { libraryStore } from "$lib/stores/library/library.store";
import { settingsStore } from "$lib/stores/settings/settings.store";
import { sidebarStore } from "$lib/stores/ui/sidebar.store";
import {
  dernierOnglet,
  lireOnglets,
  lirePlacement,
  memoriserOnglet,
  ongletCourant,
  resoudreOnglets,
} from "$lib/config/libraryTabs";

let { replie = false }: { replie?: boolean } = $props();

type Tuile = { cle: string; label: string; icone: string; actif: boolean; aller: () => void };

const pathname = $derived(page.url.pathname);
const ouvert = $derived(ongletCourant(pathname));
const libraryId = $derived($libraryStore.librarySelected?.id ?? null);

// Sections en haut : une seule tuile « Bibliothèque », sinon doublon.
const tuiles = $derived.by((): Tuile[] => {
  const accueil: Tuile = {
    cle: "home", label: $t("nav.home"), icone: "material-symbols:home-outline-rounded",
    actif: pathname === "/", aller: () => nav("/"),
  };
  const id = libraryId;
  if (id == null) return [accueil];

  if (lirePlacement($settingsStore.library_tabs_position) === "top") {
    return [accueil, {
      cle: "library", label: $t("nav.library"), icone: "material-symbols:library-music-outline-rounded",
      actif: ouvert !== null, aller: () => nav(`/library/${id}/${dernierOnglet(id)}`),
    }];
  }

  return [accueil, ...resoudreOnglets(lireOnglets($settingsStore.library_tabs)).map((o) => ({
    cle: o.key, label: $t(o.labelKey), icone: o.icon, actif: ouvert === o.key,
    aller: () => { memoriserOnglet(id, o.key); nav(`/library/${id}/${o.key}`); },
  }))];
});

function nav(path: string) {
  goto(path);
  sidebarStore.close();
}
</script>

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
