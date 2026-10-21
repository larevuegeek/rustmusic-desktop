<script lang="ts">
  // Tous les podcasts suivis par le profil : filtre, tri, grille.
  import Icon from "@iconify/svelte";
  import { t, currentLocale } from "#lib/i18n";
  import { settingsStore } from "#lib/stores/settings/settings.store";
  import { profilSelector } from "#lib/stores/profil/profil.store";
  import { podcastStore } from "#lib/stores/podcast/podcast.store";
  import { readPodcastSort, writePodcastSort, sortPodcasts, podcastSortOptions, type PodcastSort } from "#lib/helper/tools/podcastSort";
  import SearchField from "#lib/components/ui/input/SearchField.svelte";
  import MenuSelect from "#lib/components/ui/menu/MenuSelect.svelte";
  import EmptyResult from "#lib/components/ui/feedback/EmptyResult.svelte";
  import PageState from "#lib/components/ui/layout/PageState.svelte";
  import PodcastCard from "#lib/components/podcast/PodcastCard.svelte";

  const actif = $derived($settingsStore.podcasts === "true");
  const profilId = $derived($profilSelector.profilSelected?.id ?? null);
  $effect(() => {
    if (actif && profilId != null) podcastStore.ensureLoaded(profilId);
  });
  const podcasts = $derived($podcastStore.profilId === profilId ? $podcastStore.podcasts : []);

  let recherche = $state("");
  let tri = $state<PodcastSort>(readPodcastSort());
  $effect(() => writePodcastSort(tri));
  const visibles = $derived.by(() => {
    const q = recherche.trim().toLowerCase();
    return sortPodcasts(podcasts.filter((p) => !q || `${p.title} ${p.author ?? ""}`.toLowerCase().includes(q)), tri);
  });
  const formatNumber = (n: number) => n.toLocaleString($currentLocale);
</script>

{#if !actif}
  <PageState icon="material-symbols:podcasts-rounded" message={$t("podcasts.disabled")} lien={{ href: "/settings/online", label: $t("podcasts.disabled_link") }} />
{:else}
<div class="biblio-maquette flex flex-col h-full">
  <div class="shrink-0 flex flex-wrap items-center gap-x-5 gap-y-4 px-4 md:pl-8 md:pr-14 pt-6 pb-4">
    <div class="w-16 h-16 shrink-0 rounded-2xl border flex items-center justify-center text-violet-600 dark:text-violet-300
                bg-violet-500/10 border-violet-500/25 max-sm:w-12 max-sm:h-12">
      <Icon icon="material-symbols:podcasts-rounded" width="30" />
    </div>
    <div class="flex-[1_1_240px] min-w-60">
      <a href="/podcasts" class="text-xs font-bold uppercase tracking-widest text-(--rg-mu) hover:text-(--rg-tx)">{$t("podcasts.title")}</a>
      <h1 class="mt-0.5 mb-1.5 text-[34px] max-sm:text-2xl font-extrabold tracking-[-0.02em] leading-tight text-(--rg-tx)">{$t("podcasts.my_podcasts")}</h1>
      <p class="text-sm text-(--rg-mu)"><b class="font-bold text-(--rg-tx)">{formatNumber(podcasts.length)}</b> {$t(podcasts.length === 1 ? "podcasts.count_one" : "podcasts.count_n")}</p>
    </div>
  </div>

  <div class="@container shrink-0 pl-4 md:pl-8 pr-4 md:pr-14 py-3 border-t border-(--rg-line)">
    <div class="flex flex-wrap items-center gap-2.5">
      <SearchField bind:value={recherche} class="w-65 @max-4xl:flex-1 min-w-36"
                   placeholder={$t("podcasts.filter").replace("{n}", formatNumber(podcasts.length))} clearLabel={$t("podcasts.clear")} />
      <span class="flex-1"></span>
      <MenuSelect value={tri} onchange={(v) => (tri = v as PodcastSort)} labelClass="@max-xl:hidden" menuClass="w-60"
                  options={podcastSortOptions($t)} />
    </div>
  </div>

  <div class="flex-1 min-h-0 overflow-y-auto scrollbar-app pl-4 md:pl-8 pr-10 md:pr-14 pt-4 pb-16">
    {#if visibles.length === 0 && recherche.trim()}
      <EmptyResult message={$t("podcasts.no_match")} effacerLabel={$t("podcasts.clear")} oneffacer={() => (recherche = "")} />
    {:else if podcasts.length === 0 && $podcastStore.loaded}
      <p class="mt-4 max-w-md text-sm leading-relaxed text-(--rg-mu)">{$t("podcasts.empty")}</p>
    {:else}
      <div class="grid grid-cols-[repeat(auto-fill,minmax(176px,1fr))] gap-x-5 gap-y-7">
        {#each visibles as p (p.id)}<PodcastCard podcast={p} />{/each}
      </div>
    {/if}
  </div>
</div>
{/if}
