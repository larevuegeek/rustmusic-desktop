<script lang="ts">
// En-tête de bibliothèque : section en surtitre, nom et changement de bibliothèque, chiffres, actions.
import { page } from "$app/state";
import { goto } from "$app/navigation";
import Icon from "@iconify/svelte";
import { invoke } from "@tauri-apps/api/core";
import { t, currentLocale } from "$lib/i18n";
import { ilYA, dureeEcoute } from "$lib/helper/tools/dateTools";
import { libraryHeader } from "$lib/stores/library/libraryHeader";
import { tailleLisible } from "$lib/helper/tools/sizeTools";
import { ONGLETS_BIBLIOTHEQUE, ongletCourant } from "$lib/config/libraryTabs";
import { libraryStore } from "$lib/stores/library/library.store";
import { pistesAvecImport } from "$lib/stores/library/importProgress.store";
import Menu from "$lib/components/ui/menu/Menu.svelte";
import MenuItem from "$lib/components/ui/menu/MenuItem.svelte";
import LibraryActionBar from "./LibraryActionBar.svelte";
import type { Library } from "$lib/types/db/library/Library";
import type { LibraryDir } from "$lib/types/db/library/LibraryDir";

let { library }: { library: Library } = $props();

const section = $derived(ONGLETS_BIBLIOTHEQUE.find((o) => o.key === ongletCourant(page.url.pathname)) ?? null);

// Dossiers : poids total, dernier scan, état (pour le menu « Gérer »).
let dirs = $state<LibraryDir[]>([]);
async function chargerDossiers() {
  const id = library.id;
  try {
    const liste = await invoke<LibraryDir[]>("get_library_dirs", { libraryId: id });
    if (library.id === id) dirs = liste ?? [];
  } catch (e) {
    console.error("[bibliothèque] dossiers :", e);
  }
}
$effect(() => {
  void library.id;
  chargerDossiers();
});

const taille = $derived(dirs.reduce((s, d) => s + (d.total_size ?? 0), 0));
const dernierScan = $derived(
  dirs.map((d) => d.last_scan_at).filter((d): d is string => !!d).sort().at(-1) ?? null,
);

const nombre = (n: number) => n.toLocaleString($currentLocale);
const morceaux = $derived(section?.key === "tracks");
// Chaque section met ses propres chiffres en tête : les titres sur Morceaux, les albums ailleurs.
const chiffres = $derived.by(() => {
  const albums = { n: library.total_albums, un: "library_head.albums_one", plusieurs: "library_head.albums_n" };
  const artistes = { n: library.total_artists, un: "library_head.artists_one", plusieurs: "library_head.artists_n" };
  const titres = { n: $pistesAvecImport(library), un: "library_head.tracks_one", plusieurs: "library_head.tracks_n" };
  if ($libraryHeader.chiffres) return $libraryHeader.chiffres.filter((c) => c.n > 0);
  const ordre = morceaux ? [titres, albums, artistes] : section?.key === "artists" ? [artistes, albums, titres] : [albums, artistes, titres];
  return ordre.filter((c) => c.n > 0);
});

// Durée totale d'écoute, sur Morceaux seulement (elle demande les statistiques de la bibliothèque).
let ecoute = $state<number | null>(null);
$effect(() => {
  const id = library.id;
  ecoute = null;
  if (!morceaux) return;
  invoke<{ total_duration_sec: number }>("get_library_stats", { libraryId: id })
    .then((s) => { if (library.id === id) ecoute = s.total_duration_sec; })
    .catch(() => {});
});

// Autre bibliothèque : même section, comme le sélecteur de la barre latérale.
let choix = $state(false);
function changer(lib: Library) {
  choix = false;
  if (lib.id === library.id) return;
  libraryStore.selectLibrary(lib);
  goto(`/library/${lib.id}/${section?.key ?? ""}`);
}
</script>

<div class="shrink-0 flex flex-wrap items-center gap-x-5 gap-y-4 px-4 md:pl-8 md:pr-14 pt-6 pb-4">
  <div class="w-16 h-16 shrink-0 rounded-2xl border flex items-center justify-center text-(--rg-g)
              bg-(--rg-gbg) border-(--rg-gbd) dark:bg-linear-145 dark:from-[#15402a] dark:to-[#0e2519] max-sm:w-12 max-sm:h-12">
    <Icon icon={section?.key === "genres" ? "material-symbols:sell-outline-rounded" : section?.key === "folders" ? "material-symbols:folder-open-outline-rounded" : "material-symbols:library-music-outline-rounded"} width="30" />
  </div>

  <div class="flex-[1_1_240px] min-w-60">
    <p class="text-xs font-bold uppercase tracking-widest text-(--rg-mu)">
      {$t("library_head.kicker")}{#if section}{" · "}{$t(section.labelKey)}{/if}
    </p>
    <h1 class="mt-0.5 mb-1.5 flex items-center gap-2 text-[34px] max-sm:text-2xl font-extrabold tracking-[-0.02em] leading-tight text-(--rg-tx)">
      <span class="truncate">{library.name}</span>
      {#if $libraryStore.libraries.length > 1}
        <span class="relative">
          <button type="button" class="w-7.5 h-7.5 flex items-center justify-center rounded-lg cursor-pointer text-(--rg-mu) hover:bg-(--rg-carte) hover:text-(--rg-tx)"
                  title={$t("library_head.switch")} aria-label={$t("library_head.switch")} aria-expanded={choix} onclick={() => (choix = !choix)}>
            <Icon icon="material-symbols:expand-more-rounded" width="24" />
          </button>
          <Menu bind:open={choix} align="left" class="w-64 text-base font-normal tracking-normal">
            {#each $libraryStore.libraries as lib (lib.id)}
              <MenuItem icon="material-symbols:library-music-outline-rounded" actif={lib.id === library.id} onclick={() => changer(lib)}>
                {lib.name}
                {#snippet fin()}<span class="font-mono text-[11px] text-(--rg-mu2)">{nombre($pistesAvecImport(lib))}</span>{/snippet}
              </MenuItem>
            {/each}
          </Menu>
        </span>
      {/if}
    </h1>
    <div class="flex flex-wrap items-baseline gap-x-4.5 gap-y-1.5 text-sm text-(--rg-mu)">
      {#each chiffres as c (c.un)}
        <span class="whitespace-nowrap"><b class="font-bold text-(--rg-tx)">{nombre(c.n)}</b> {$t(c.n === 1 ? c.un : c.plusieurs)}</span>
      {/each}
      {#if $libraryHeader.chiffres}
        <!-- La page donne ses propres chiffres. -->
      {:else if morceaux && ecoute}
        {@const d = dureeEcoute(ecoute)}
        <span class="whitespace-nowrap"><b class="font-bold text-(--rg-tx)">{d}</b> {$t("library_head.listening")}</span>
      {:else if taille}
        {@const poids = tailleLisible(taille, $currentLocale)}
        <span class="whitespace-nowrap"><b class="font-bold text-(--rg-tx)">{poids.slice(0, poids.lastIndexOf(" "))}</b> {poids.slice(poids.lastIndexOf(" ") + 1)}</span>
      {/if}
      {#if dernierScan}
        <span class="whitespace-nowrap font-mono text-xs">{$t("library_head.scan_ago").replace("{t}", ilYA(dernierScan, $currentLocale, "short"))}</span>
      {/if}
    </div>
  </div>

  <LibraryActionBar {library} {dirs} onchange={chargerDossiers} action={$libraryHeader.action ?? null} />
</div>
