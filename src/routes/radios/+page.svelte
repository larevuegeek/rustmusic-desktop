<script lang="ts">
  // Radios : recherche ou adresse collée, mes radios favorites, les plus écoutées du pays par genre.
  import Icon from "@iconify/svelte";
  import { t, currentLocale } from "#lib/i18n";
  import { settingsStore } from "#lib/stores/settings/settings.store";
  import { profilSelector } from "#lib/stores/profil/profil.store";
  import { radioStore, toResult } from "#lib/stores/radio/radio.store";
  import { toasts } from "#lib/stores/ui/toast.store";
  import { radioPlayer } from "#lib/stores/radio/radioPlayer.store";
  import { podcastCountry, countryName, countryOptions } from "#lib/helper/tools/podcastCountry";
  import MenuSelect from "#lib/components/ui/menu/MenuSelect.svelte";
  import CarouselSection from "#lib/components/ui/carousel/CarouselSection.svelte";
  import HeroSearchBar from "#lib/components/ui/input/HeroSearchBar.svelte";
  import HeaderGlow from "#lib/components/ui/deco/HeaderGlow.svelte";
  import PageState from "#lib/components/ui/layout/PageState.svelte";
  import RadioCard from "#lib/components/radio/RadioCard.svelte";
  import type { RadioResult } from "#lib/types/ui/radio/Radio";

  const actif = $derived($settingsStore.radios === "true");
  const profilId = $derived($profilSelector.profilSelected?.id ?? null);
  $effect(() => {
    if (actif && profilId != null) radioStore.ensureLoaded(profilId);
  });
  const favoris = $derived($radioStore.profilId === profilId ? $radioStore.radios : []);
  const recentes = $derived($radioStore.profilId === profilId ? $radioStore.recent : []);
  const formatNumber = (n: number) => n.toLocaleString($currentLocale);

  // Pays des stations populaires : celui choisi, sinon celui de l'ordinateur.
  const pays = $derived(podcastCountry($settingsStore.radio_country, $currentLocale));
  const paysOptions = $derived(countryOptions($currentLocale, $t("podcasts.country_auto")));


  // ─── Recherche, ou adresse de flux collée ───
  let saisie = $state("");
  let resultats = $state<RadioResult[] | null>(null);
  let cherche = $state(false);
  $effect(() => {
    if (!saisie.trim()) resultats = null;
  });

  async function submit(s: string, adresse: boolean) {
    cherche = true;
    try {
      resultats = adresse ? [await radioStore.resolve(s)] : await radioStore.search(s, null, null);
    } catch (err) {
      toasts.push({ type: "error", title: $t(adresse ? "radios.error_resolve" : "radios.error_search"), message: String(err) });
    } finally {
      cherche = false;
    }
  }

  // ─── Les plus écoutées, par genre ───
  const GENRES: (string | null)[] = [null, "news", "pop", "rock", "jazz", "classical", "electronic", "dance", "hiphop", "chillout", "talk", "oldies"];
  let genre = $state<string | null>(null);
  let populaires = $state<RadioResult[] | null>(null);
  let populairesErreur = $state(false);
  let tous = $state(false);
  $effect(() => {
    if (!actif) return;
    const g = genre;
    const c = pays;
    populaires = null;
    populairesErreur = false;
    tous = false;
    // Seule la dernière demande (genre et pays) s'affiche.
    const current = () => g === genre && c === pays;
    radioStore.top(c, g).then(
      (l) => { if (current()) populaires = l; },
      () => { if (current()) populairesErreur = true; },
    );
  });
  const VISIBLES = 18;

  const grille = "grid grid-cols-[repeat(auto-fill,minmax(150px,1fr))] gap-x-5 gap-y-7";
</script>

{#if !actif}
  <PageState icon="material-symbols:radio-rounded" message={$t("radios.disabled")} lien={{ href: "/settings/online", label: $t("radios.disabled_link") }} />
{:else}
<div class="h-full overflow-y-auto scrollbar-app">
  <header class="relative px-4 md:px-8 pt-9 pb-8 overflow-hidden">
    <HeaderGlow tone="rose" />
    <div class="relative">
      <p class="flex items-center gap-2 text-xs font-bold uppercase tracking-widest text-rose-600 dark:text-rose-300">
        <Icon icon="material-symbols:radio-rounded" width="16" />{$t("podcasts.kicker")}
      </p>
      <h1 class="mt-1.5 text-[44px] max-sm:text-3xl font-extrabold tracking-[-0.03em] leading-none text-(--rg-tx)">{$t("radios.title")}</h1>
      <p class="mt-2.5 text-[15px] text-(--rg-tx2)">
        {#if favoris.length > 0}
          <b class="font-bold text-(--rg-tx)">{formatNumber(favoris.length)}</b> {$t(favoris.length === 1 ? "radios.favorites_one" : "radios.favorites_n")}
        {:else}
          {$t("radios.tagline")}
        {/if}
      </p>
      <div class="mt-7">
        <HeroSearchBar bind:value={saisie} busy={cherche} onsubmit={submit} accent="rose"
                       placeholder={$t("radios.search_placeholder")} searchLabel={$t("radios.search")} addLabel={$t("radios.open_address")}
                       addressIcon="material-symbols:radio-rounded" />
      </div>
    </div>
  </header>

  <div class="px-4 md:px-8 pb-24">
    {#if resultats}
      <section class="mt-2">
        <div class="flex flex-wrap items-end gap-3 mb-4">
          <h2 class="text-[22px] font-extrabold tracking-[-0.01em] text-(--rg-tx)">{$t("radios.results")}</h2>
          <span class="flex-1"></span>
          <button type="button" onclick={() => (saisie = "")} class="text-[13px] font-semibold cursor-pointer text-(--rg-mu) hover:text-(--rg-tx)">{$t("common.clear")}</button>
        </div>
        {#if resultats.length === 0}
          <p class="py-3 max-w-lg text-sm text-(--rg-mu)">{$t("radios.no_results")}</p>
        {:else}
          <div class={grille}>
            {#each resultats as r (r.uuid ?? r.stream_url)}<RadioCard station={r} onplay={() => radioPlayer.play(r)} />{/each}
          </div>
        {/if}
      </section>
    {:else}
      {#if favoris.length > 0}
        <CarouselSection titre={$t("radios.my_radios")} sousTitre={$t("radios.my_radios_hint")} grille={true}
                         items={favoris} cle={(r) => String(r.id)} class="mt-2">
          {#snippet carte(r)}<RadioCard station={toResult(r)} logoPath={r.logo_path} onplay={() => radioPlayer.play(toResult(r), r.logo_path, r.id)} />{/snippet}
          {#snippet ligne(r)}<RadioCard station={toResult(r)} logoPath={r.logo_path} onplay={() => radioPlayer.play(toResult(r), r.logo_path, r.id)} />{/snippet}
        </CarouselSection>
      {:else if $radioStore.loaded}
        <div class="mt-2 flex items-center gap-4 p-5 rounded-2xl border border-dashed border-(--rg-bd2) max-w-180">
          <span class="w-12 h-12 shrink-0 rounded-xl flex items-center justify-center bg-rose-500/10 text-rose-600 dark:text-rose-300">
            <Icon icon="material-symbols:star-outline-rounded" width="26" />
          </span>
          <p class="text-sm leading-relaxed text-(--rg-tx2)">{$t("radios.empty")}</p>
        </div>
      {/if}

      {#if recentes.length > 0}
        <CarouselSection titre={$t("radios.recent")} sousTitre={$t("radios.recent_hint")} grille={true}
                         items={recentes} cle={(r) => String(r.id)} class="mt-12">
          {#snippet carte(r)}<RadioCard station={toResult(r)} logoPath={r.logo_path} onplay={() => radioPlayer.play(toResult(r), r.logo_path)} />{/snippet}
          {#snippet ligne(r)}<RadioCard station={toResult(r)} logoPath={r.logo_path} onplay={() => radioPlayer.play(toResult(r), r.logo_path)} />{/snippet}
        </CarouselSection>
      {/if}

      <section class="mt-12">
        <div class="flex flex-wrap items-end gap-3 mb-3.5">
          <div class="flex flex-col gap-1 min-w-0">
            <h2 class="text-[22px] font-extrabold tracking-[-0.01em] text-(--rg-tx)">{$t("radios.popular")}</h2>
            <span class="text-[13px] text-(--rg-mu)">{$t("radios.popular_hint").replace("{country}", countryName(pays, $currentLocale))}</span>
          </div>
          <span class="flex-1"></span>
          <MenuSelect value={$settingsStore.radio_country} onchange={(v) => settingsStore.set("radio_country", v)}
                      options={paysOptions} menuClass="w-64 max-h-80 overflow-y-auto" />
        </div>
        <div class="flex gap-2 overflow-x-auto pb-1 mb-5 scrollbar-none" role="tablist" aria-label={$t("radios.popular")}>
          {#each GENRES as g (g ?? "all")}
            <button type="button" role="tab" aria-selected={genre === g} onclick={() => (genre = g)}
                    class="shrink-0 h-8.5 px-3.5 rounded-full border text-[13px] font-semibold cursor-pointer transition-colors
                           {genre === g
                             ? 'bg-rose-600 border-rose-600 text-white'
                             : 'border-(--rg-bd2) text-(--rg-tx2) hover:border-rose-400/60 hover:text-(--rg-tx)'}">
              {$t(`radios.genre_${g ?? "all"}`)}
            </button>
          {/each}
        </div>
        {#if populairesErreur}
          <p class="py-3 text-sm text-(--rg-mu)">{$t("radios.popular_error")}</p>
        {:else if populaires === null}
          <div class={grille} aria-busy="true">
            {#each { length: 6 } as _, i (i)}
              <div class="flex flex-col gap-2.5">
                <span class="aspect-square rounded-2xl bg-(--rg-s2) animate-pulse"></span>
                <span class="h-3.5 w-3/4 rounded bg-(--rg-s2) animate-pulse"></span>
              </div>
            {/each}
          </div>
        {:else if populaires.length === 0}
          <p class="py-3 text-sm text-(--rg-mu)">{$t("radios.no_results")}</p>
        {:else}
          <div class={grille}>
            {#each tous ? populaires : populaires.slice(0, VISIBLES) as r, i (r.uuid ?? r.stream_url)}
              <RadioCard station={r} rang={i + 1} onplay={() => radioPlayer.play(r)} />
            {/each}
          </div>
          {#if !tous && populaires.length > VISIBLES}
            <button type="button" onclick={() => (tous = true)}
                    class="mt-6 h-9.5 px-4 rounded-xl border cursor-pointer text-[13px] font-semibold transition-colors border-(--rg-bd2) text-(--rg-tx) hover:border-rose-400/60">
              {$t("radios.show_more").replace("{n}", formatNumber(populaires.length - VISIBLES))}
            </button>
          {/if}
        {/if}
      </section>
    {/if}
  </div>
</div>
{/if}
