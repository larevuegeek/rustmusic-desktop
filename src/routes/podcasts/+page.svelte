<script lang="ts">
  // Accueil des podcasts : recherche, épisodes à écouter, mes podcasts, les plus écoutés du moment.
  import Icon from "@iconify/svelte";
  import { goto } from "$app/navigation";
  import { t, currentLocale } from "#lib/i18n";
  import { settingsStore } from "#lib/stores/settings/settings.store";
  import { profilSelector } from "#lib/stores/profil/profil.store";
  import { podcastStore } from "#lib/stores/podcast/podcast.store";
  import { player } from "#lib/stores/player/player.store";
  import { toasts } from "#lib/stores/ui/toast.store";
  import { invoke } from "@tauri-apps/api/core";
  import { playEpisode } from "#lib/actions/podcast/PodcastAction";
  import { podcastCountry, countryName, countryOptions } from "#lib/helper/tools/podcastCountry";
  import {
    readPodcastSort, writePodcastSort, sortPodcasts, podcastSortOptions, type PodcastSort,
    readPopularSort, writePopularSort, sortDirectory, popularSortOptions, type PopularSort,
  } from "#lib/helper/tools/podcastSort";
  import MenuSelect from "#lib/components/ui/menu/MenuSelect.svelte";
  import CarouselSection from "#lib/components/ui/carousel/CarouselSection.svelte";
  import Carousel from "#lib/components/ui/carousel/Carousel.svelte";
  import HeroSearchBar from "#lib/components/ui/input/HeroSearchBar.svelte";
  import PodcastCard from "#lib/components/podcast/PodcastCard.svelte";
  import PodcastDirectoryCard from "#lib/components/podcast/PodcastDirectoryCard.svelte";
  import PodcastEpisodeCard from "#lib/components/podcast/PodcastEpisodeCard.svelte";
  import PageState from "#lib/components/ui/layout/PageState.svelte";
  import HeaderGlow from "#lib/components/ui/deco/HeaderGlow.svelte";
  import type { DirectoryResult, EpisodeView, LatestEpisode, PodcastView } from "#lib/types/ui/podcast/Podcast";

  const actif = $derived($settingsStore.podcasts === "true");
  const profilId = $derived($profilSelector.profilSelected?.id ?? null);
  $effect(() => {
    if (actif && profilId != null) podcastStore.ensureLoaded(profilId);
  });

  const podcasts = $derived($podcastStore.profilId === profilId ? $podcastStore.podcasts : []);
  let tri = $state<PodcastSort>(readPodcastSort());
  $effect(() => writePodcastSort(tri));
  const podcastsTries = $derived(sortPodcasts(podcasts, tri));
  const nouveaux = $derived(podcasts.reduce((n, p) => n + p.new_count, 0));
  const suivis = $derived(new Map(podcasts.map((p) => [p.feed_url, p.id])));
  const formatNumber = (n: number) => n.toLocaleString($currentLocale);

  // Pays de l'annuaire : celui choisi, sinon celui de l'ordinateur (voir `podcastCountry`).
  const pays = $derived(podcastCountry($settingsStore.podcast_country, $currentLocale));
  const paysOptions = $derived(countryOptions($currentLocale, $t("podcasts.country_auto")));

  // ─── À écouter : commencés, puis nouveaux ───
  let aEcouter = $state<LatestEpisode[]>([]);
  // Le statut seul : le lecteur publie sa position plusieurs fois par seconde.
  const playerStatus = $derived($player.status);
  $effect(() => {
    if (!actif || profilId == null) return;
    void $podcastStore.podcasts;
    void playerStatus;
    podcastStore.latest(20).then((l) => (aEcouter = l)).catch(() => {});
  });

  // La suite : les épisodes non écoutés qui viennent après dans la liste de l'émission.
  async function playLatest(e: LatestEpisode) {
    const podcast = { title: e.podcast_title, cover_path: e.cover_path } as PodcastView;
    const liste = await invoke<EpisodeView[]>("get_podcast_episodes", { podcastId: e.podcast_id }).catch(() => [] as EpisodeView[]);
    const i = liste.findIndex((x) => x.id === e.id);
    playEpisode(podcast, e, i >= 0 ? liste.slice(i + 1) : []);
  }

  // ─── Recherche, ou adresse de flux collée ───
  let saisie = $state("");
  let resultats = $state<DirectoryResult[] | null>(null);
  let cherche = $state(false);
  $effect(() => {
    if (!saisie.trim()) resultats = null;
  });

  async function submit(s: string, adresse: boolean) {
    if (adresse) {
      const p = await follow(s);
      if (p) {
        saisie = "";
        goto(`/podcasts/${p.id}`);
      }
      return;
    }
    cherche = true;
    try {
      resultats = await podcastStore.search(s, pays);
    } catch (err) {
      toasts.push({ type: "error", title: $t("podcasts.error_search"), message: String(err) });
    } finally {
      cherche = false;
    }
  }

  // ─── Abonnement ───
  let ajout = $state<string | null>(null);
  async function follow(url: string): Promise<PodcastView | null> {
    ajout = url;
    try {
      const p = await podcastStore.follow(url);
      toasts.push({ type: "success", title: $t("podcasts.followed").replace("{title}", p.title) });
      return p;
    } catch (err) {
      toasts.push({ type: "error", title: $t("podcasts.error_follow"), message: String(err) });
      return null;
    } finally {
      ajout = null;
    }
  }

  // Clic sur une carte de l'annuaire : sa page, en s'abonnant si besoin. L'aperçu est au clic droit / « … ».
  async function open(r: DirectoryResult) {
    const id = suivis.get(r.feed_url) ?? (await follow(r.feed_url))?.id;
    if (id != null) goto(`/podcasts/${id}`);
  }

  // ─── Les plus écoutés, par catégorie Apple ───
  const GENRES: { id: number | null; cle: string }[] = [
    { id: null, cle: "all" },
    { id: 1489, cle: "news" },
    { id: 1324, cle: "society" },
    { id: 1487, cle: "history" },
    { id: 1488, cle: "true_crime" },
    { id: 1303, cle: "comedy" },
    { id: 1533, cle: "science" },
    { id: 1304, cle: "education" },
    { id: 1321, cle: "business" },
    { id: 1545, cle: "sports" },
    { id: 1301, cle: "arts" },
    { id: 1318, cle: "technology" },
  ];
  let genre = $state<number | null>(null);
  let populaires = $state<DirectoryResult[] | null>(null);
  let populairesErreur = $state(false);
  let tous = $state(false);
  $effect(() => {
    if (!actif) return;
    const g = genre;
    const c = pays;
    populaires = null;
    populairesErreur = false;
    tous = false;
    // Seule la dernière demande (catégorie et pays) s'affiche.
    const current = () => g === genre && c === pays;
    podcastStore.top(c, g).then(
      (l) => { if (current()) populaires = l; },
      () => { if (current()) populairesErreur = true; },
    );
  });
  const POPULAIRES_VISIBLES = 15;
  let triPopulaires = $state<PopularSort>(readPopularSort());
  $effect(() => writePopularSort(triPopulaires));
  // La place au classement reste affichée, quel que soit le tri.
  const rangs = $derived(new Map((populaires ?? []).map((r, i) => [r.feed_url, i + 1])));
  const populairesTries = $derived(populaires ? sortDirectory(populaires, triPopulaires) : null);

  let verification = $state(false);
  async function checkAll() {
    verification = true;
    try {
      const n = await podcastStore.refreshAll();
      toasts.push({
        type: "info",
        title: n === 0 ? $t("podcasts.up_to_date") : `${formatNumber(n)} ${$t(n === 1 ? "podcasts.new_one" : "podcasts.new_n")}`,
      });
    } catch (err) {
      toasts.push({ type: "error", title: $t("podcasts.error_refresh"), message: String(err) });
    } finally {
      verification = false;
    }
  }

  const bouton = "h-10 px-4 inline-flex items-center gap-2 rounded-xl border text-sm font-semibold transition-colors cursor-pointer bg-(--rg-carte)/70 backdrop-blur-sm border-(--rg-bd2) text-(--rg-tx) hover:border-violet-400/60 hover:text-violet-600 dark:hover:text-violet-300";
  const grille = "grid grid-cols-[repeat(auto-fill,minmax(158px,1fr))] gap-x-5 gap-y-7";
</script>

{#snippet carteAnnuaire(r: DirectoryResult, rang: number | null)}
  <PodcastDirectoryCard podcast={r} {rang} suivi={suivis.has(r.feed_url)} busy={ajout === r.feed_url}
                        onopen={() => open(r)} onfollow={() => follow(r.feed_url)} />
{/snippet}

{#if !actif}
  <PageState icon="material-symbols:podcasts-rounded" message={$t("podcasts.disabled")} lien={{ href: "/settings/online", label: $t("podcasts.disabled_link") }} />
{:else}
<div class="h-full overflow-y-auto scrollbar-app">
  <header class="relative px-4 md:px-8 pt-9 pb-8 overflow-hidden">
    <HeaderGlow />
    <div class="relative">
      <div class="flex flex-wrap items-start gap-x-6 gap-y-4">
        <div class="flex-[1_1_320px] min-w-0">
          <p class="flex items-center gap-2 text-xs font-bold uppercase tracking-widest text-violet-600 dark:text-violet-300">
            <Icon icon="material-symbols:podcasts-rounded" width="16" />{$t("podcasts.kicker")}
          </p>
          <h1 class="mt-1.5 text-[44px] max-sm:text-3xl font-extrabold tracking-[-0.03em] leading-none text-(--rg-tx)">{$t("podcasts.title")}</h1>
          <p class="mt-2.5 text-[15px] text-(--rg-tx2)">
            {#if podcasts.length > 0}
              <b class="font-bold text-(--rg-tx)">{formatNumber(podcasts.length)}</b> {$t(podcasts.length === 1 ? "podcasts.count_one" : "podcasts.count_n")}
              {#if nouveaux > 0}· <b class="font-bold text-violet-600 dark:text-violet-300">{formatNumber(nouveaux)}</b> {$t(nouveaux === 1 ? "podcasts.new_one" : "podcasts.new_n")}{/if}
            {:else}
              {$t("podcasts.tagline")}
            {/if}
          </p>
        </div>
        <div class="flex flex-wrap items-center gap-2.5">
          <a href="/podcasts/downloads" class={bouton}>
            <Icon icon="material-symbols:download-for-offline-outline-rounded" width="18" />{$t("podcasts.downloads")}
          </a>
          {#if podcasts.length > 0}
            <button type="button" onclick={checkAll} disabled={verification} class="{bouton} disabled:opacity-60 disabled:cursor-wait">
              <Icon icon={verification ? "material-symbols:progress-activity" : "material-symbols:refresh-rounded"} width="18" class={verification ? "animate-spin" : ""} />
              {verification ? $t("podcasts.checking") : $t("podcasts.check_all")}
            </button>
          {/if}
        </div>
      </div>
      <div class="mt-7">
        <HeroSearchBar bind:value={saisie} busy={cherche || (ajout !== null && ajout === saisie.trim())} onsubmit={submit}
                       placeholder={$t("podcasts.search_placeholder")} searchLabel={$t("podcasts.search")} addLabel={$t("podcasts.add_feed")}
                       addressIcon="material-symbols:rss-feed-rounded" />
      </div>
    </div>
  </header>

  <div class="px-4 md:px-8 pb-24">
    {#if resultats}
      <!-- ─── Résultats de recherche ─── -->
      <section class="mt-2">
        <div class="flex flex-wrap items-end gap-3 mb-4">
          <div class="flex flex-col gap-1 min-w-0">
            <h2 class="text-[22px] font-extrabold tracking-[-0.01em] text-(--rg-tx)">{$t("podcasts.results")}</h2>
            <span class="text-[13px] text-(--rg-mu)">{$t("podcasts.results_hint")}</span>
          </div>
          <span class="flex-1"></span>
          <button type="button" onclick={() => (saisie = "")} class="text-[13px] font-semibold cursor-pointer text-(--rg-mu) hover:text-(--rg-tx)">{$t("podcasts.clear")}</button>
        </div>
        {#if resultats.length === 0}
          <p class="py-3 max-w-lg text-sm text-(--rg-mu)">{$t("podcasts.no_results")}</p>
        {:else}
          <div class={grille}>
            {#each resultats as r (r.feed_url)}{@render carteAnnuaire(r, null)}{/each}
          </div>
        {/if}
      </section>
    {:else}
      <!-- ─── À écouter ─── -->
      {#if aEcouter.length > 0}
        <section class="mt-2">
          <div class="flex flex-col gap-1 mb-4">
            <h2 class="text-[22px] font-extrabold tracking-[-0.01em] text-(--rg-tx)">{$t("podcasts.to_listen")}</h2>
            <span class="text-[13px] text-(--rg-mu)">{$t("podcasts.to_listen_hint")}</span>
          </div>
          <Carousel class="gap-4.5 pb-1">
            {#each aEcouter as e (e.id)}
              <div class="snap-start shrink-0 w-[clamp(160px,17%,210px)]">
                <PodcastEpisodeCard episode={e} enLecture={$player.pathFile === e.path && $player.status === "playing"} onplay={() => playLatest(e)} />
              </div>
            {/each}
          </Carousel>
        </section>
      {/if}

      <!-- ─── Mes podcasts ─── -->
      {#if podcasts.length > 0}
        <CarouselSection titre={$t("podcasts.my_podcasts")} sousTitre={$t("podcasts.my_podcasts_hint")} grille={true}
                         items={podcastsTries} cle={(p) => String(p.id)} reinit={tri} class={aEcouter.length > 0 ? "mt-12" : "mt-2"}>
          {#snippet extra()}
            <MenuSelect value={tri} onchange={(v) => (tri = v as PodcastSort)} labelClass="max-md:hidden" menuClass="w-60" options={podcastSortOptions($t)} />
            <a href="/podcasts/subscriptions" class="h-8.5 px-3 mr-1 inline-flex items-center gap-1 rounded-full text-[13px] font-semibold text-(--rg-tx2) hover:bg-(--rg-s2) hover:text-(--rg-tx)">
              {$t("podcasts.see_all")}<Icon icon="material-symbols:chevron-right-rounded" width="18" />
            </a>
          {/snippet}
          {#snippet carte(p)}<PodcastCard podcast={p} />{/snippet}
          {#snippet ligne(p)}<PodcastCard podcast={p} />{/snippet}
        </CarouselSection>
      {:else if $podcastStore.loaded}
        <div class="mt-2 flex items-center gap-4 p-5 rounded-2xl border border-dashed border-(--rg-bd2) max-w-180">
          <span class="w-12 h-12 shrink-0 rounded-xl flex items-center justify-center bg-violet-500/10 text-violet-600 dark:text-violet-300">
            <Icon icon="material-symbols:library-add-outline-rounded" width="26" />
          </span>
          <p class="text-sm leading-relaxed text-(--rg-tx2)">{$t("podcasts.empty")}</p>
        </div>
      {/if}

      <!-- ─── Les plus écoutés du moment ─── -->
      <section class="mt-12">
        <div class="flex flex-wrap items-end gap-3 mb-3.5">
          <div class="flex flex-col gap-1 min-w-0">
            <h2 class="text-[22px] font-extrabold tracking-[-0.01em] text-(--rg-tx)">{$t("podcasts.popular")}</h2>
            <span class="text-[13px] text-(--rg-mu)">{$t("podcasts.popular_hint").replace("{country}", countryName(pays, $currentLocale))}</span>
          </div>
          <span class="flex-1"></span>
          <MenuSelect value={triPopulaires} onchange={(v) => (triPopulaires = v as PopularSort)} labelClass="max-md:hidden" menuClass="w-56"
                      options={popularSortOptions($t)} />
          <!-- Pays du classement et de la recherche, retenu dans les réglages. -->
          <MenuSelect value={$settingsStore.podcast_country} onchange={(v) => settingsStore.set("podcast_country", v)}
                      options={paysOptions} menuClass="w-64 max-h-80 overflow-y-auto" />
        </div>
        <div class="flex gap-2 overflow-x-auto pb-1 mb-5 scrollbar-none" role="tablist" aria-label={$t("podcasts.popular")}>
          {#each GENRES as g (g.cle)}
            <button type="button" role="tab" aria-selected={genre === g.id} onclick={() => (genre = g.id)}
                    class="shrink-0 h-8.5 px-3.5 rounded-full border text-[13px] font-semibold cursor-pointer transition-colors
                           {genre === g.id
                             ? 'bg-violet-600 border-violet-600 text-white'
                             : 'border-(--rg-bd2) text-(--rg-tx2) hover:border-violet-400/60 hover:text-(--rg-tx)'}">
              {$t(`podcasts.genre_${g.cle}`)}
            </button>
          {/each}
        </div>
        {#if populairesErreur}
          <p class="py-3 text-sm text-(--rg-mu)">{$t("podcasts.popular_error")}</p>
        {:else if populaires === null}
          <div class={grille} aria-busy="true">
            {#each { length: 6 } as _, i (i)}
              <div class="flex flex-col gap-2.5">
                <span class="aspect-square rounded-2xl bg-(--rg-s2) animate-pulse"></span>
                <span class="h-3.5 w-3/4 rounded bg-(--rg-s2) animate-pulse"></span>
              </div>
            {/each}
          </div>
        {:else if populairesTries}
          <div class={grille}>
            {#each tous ? populairesTries : populairesTries.slice(0, POPULAIRES_VISIBLES) as r (r.feed_url)}{@render carteAnnuaire(r, rangs.get(r.feed_url) ?? null)}{/each}
          </div>
          {#if !tous && populaires.length > POPULAIRES_VISIBLES}
            <button type="button" onclick={() => (tous = true)}
                    class="mt-6 h-9.5 px-4 rounded-xl border cursor-pointer text-[13px] font-semibold transition-colors border-(--rg-bd2) text-(--rg-tx) hover:border-violet-400/60">
              {$t("podcasts.show_more_popular").replace("{n}", String(populaires.length - POPULAIRES_VISIBLES))}
            </button>
          {/if}
        {/if}
      </section>
    {/if}
  </div>
</div>
{/if}
