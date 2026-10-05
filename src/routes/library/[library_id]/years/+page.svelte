<script lang="ts">
// Années : un déroulé par décennie (la plus récente d'abord), une année ou toute la décennie, et son mix.
import { page } from "$app/state";
import { onDestroy } from "svelte";
import Icon from "@iconify/svelte";
import { t, currentLocale } from "#lib/i18n";
import { libraryHeader } from "#lib/stores/library/libraryHeader";
import { libraryContentStore } from "#lib/stores/library/libraryContent.store";
import { handlePeriodeMix, handleRandomMix } from "#lib/actions/queue/QueueAction";
import { lireLocal, ecrireLocal } from "#lib/helper/tools/stockage";
import LibraryImportingLoader from "#lib/components/library/common/loader/LibraryImportingLoader.svelte";
import { importAffiche } from "#lib/stores/library/importProgress.store";
import AlbumListItem from "#lib/components/library/album/AlbumListItem.svelte";
import Carousel from "#lib/components/ui/carousel/Carousel.svelte";
import MenuSelect from "#lib/components/ui/menu/MenuSelect.svelte";
import YearChips from "#lib/components/ui/input/YearChips.svelte";
import { estDatee, decennieDe, courtDecennie as court, teinteDecennie as teinteDe } from "#lib/helper/library/periode";
import type { AlbumListView } from "#lib/types/ui/library/album/AlbumListView";

const libraryId = $derived(Number(page.params.library_id));

// Au-delà, le carrousel « Tout » s'arrête : une année montre le reste.
const PLAFOND = 40;

// ─── Ordre (retenu d'une visite à l'autre) ───
const CLE_ORDRE = "annees:ordre";
let recentes = $state(lireLocal(CLE_ORDRE, "recent") !== "ancien");
$effect(() => ecrireLocal(CLE_ORDRE, recentes ? "recent" : "ancien"));

type Decennie = { d: number; albums: AlbumListView[]; parAnnee: Map<number, AlbumListView[]>; titres: number; artistes: number };

const albums = $derived($libraryContentStore.albums);
const sansAnnee = $derived(albums.filter((a) => !estDatee(a.year)).length);

// Regroupé une fois ; l'ordre se pose à part.
const groupes = $derived.by((): Decennie[] => {
  const par = new Map<number, AlbumListView[]>();
  for (const a of albums) {
    if (!estDatee(a.year)) continue;
    const d = decennieDe(a.year);
    (par.get(d) ?? par.set(d, []).get(d)!).push(a);
  }
  return [...par.entries()]
    .map(([d, liste]) => {
      const parAnnee = new Map<number, AlbumListView[]>();
      for (const a of liste) (parAnnee.get(a.year!) ?? parAnnee.set(a.year!, []).get(a.year!)!).push(a);
      return {
        d,
        albums: liste,
        parAnnee,
        titres: liste.reduce((s, a) => s + (a.total_tracks ?? 0), 0),
        artistes: new Set(liste.map((a) => a.artist_id ?? a.artist).filter(Boolean)).size,
      };
    });
});
const decennies = $derived([...groupes].sort((x, y) => (recentes ? y.d - x.d : x.d - y.d)));

// L'année choisie dans chaque décennie ; absente = toute la décennie. Remise à zéro avec la bibliothèque.
let choix = $state<Record<number, number | null>>({});
$effect(() => {
  void libraryId;
  choix = {};
});

const comparer = new Intl.Collator(undefined, { numeric: true, sensitivity: "base" });
function selection(dec: Decennie): AlbumListView[] {
  const annee = choix[dec.d] ?? null;
  const liste = annee == null ? dec.albums : dec.parAnnee.get(annee) ?? [];
  return [...liste].sort((x, y) =>
    (recentes ? y.year! - x.year! : x.year! - y.year!)
    || comparer.compare(x.artist ?? "", y.artist ?? "")
    || comparer.compare(x.title, y.title));
}

const nombre = (n: number) => n.toLocaleString($currentLocale);
const plusieurs = (n: number, un: string, n_: string) => $t(n === 1 ? un : n_);

// ─── En-tête : chiffres et mix au hasard ───
$effect(() => {
  const n = decennies.length;
  const dates = albums.length - sansAnnee;
  const sans = sansAnnee;
  libraryHeader.update(() => ({
    action: { cle: "genres_view.random_mix", icone: "material-symbols:shuffle-rounded", lancer: () => handleRandomMix(libraryId) },
    chiffres: [
      { n, un: "library_head.decades_one", plusieurs: "library_head.decades_n" },
      { n: dates, un: "library_head.albums_one", plusieurs: "library_head.albums_n" },
      { n: sans, un: "library_head.undated_one", plusieurs: "library_head.undated_n" },
    ],
  }));
});
onDestroy(() => libraryHeader.update((h) => ({ ...h, action: null, chiffres: null })));

// ─── Index des décennies ───
let defilement = $state<HTMLDivElement | null>(null);
function aller(d: number) {
  defilement?.querySelector(`#decennie-${d}`)?.scrollIntoView({ behavior: "smooth", block: "start" });
}

// Une décennie ne se construit qu'en approchant de l'écran : ses pochettes coûtent.
let montees = $state(new Set<number>());
function monter(noeud: HTMLElement, d: number) {
  const io = new IntersectionObserver((e) => {
    if (e.some((x) => x.isIntersecting)) {
      montees = new Set(montees).add(d);
      io.disconnect();
    }
  }, { root: noeud.closest(".overflow-y-auto"), rootMargin: "600px 0px" });
  io.observe(noeud);
  return { destroy: () => io.disconnect() };
}

const puce = "h-8 px-3 inline-flex items-center gap-1.5 rounded-full border text-[13px] font-semibold transition-colors";
</script>

{#if $importAffiche}
  <LibraryImportingLoader />

{:else if $libraryContentStore.isLoading && albums.length === 0}
  <div class="flex items-center justify-center py-20 text-(--rg-mu)">
    <Icon icon="material-symbols:progress-activity" width="26" class="animate-spin" />
  </div>

{:else if decennies.length === 0}
  <div class="flex flex-col items-center justify-center py-20 px-6 text-center">
    <div class="w-16 h-16 mb-5 rounded-2xl border flex items-center justify-center bg-(--rg-gbg) border-(--rg-gbd) text-(--rg-g)">
      <Icon icon="material-symbols:calendar-month-outline-rounded" width="30" />
    </div>
    <h3 class="text-base font-semibold text-(--rg-tx) mb-1.5">{$t("years_view.empty")}</h3>
    <p class="text-sm text-(--rg-mu) max-w-xs leading-relaxed">{$t("years_view.empty_desc")}</p>
  </div>

{:else}
  <div class="flex flex-col h-full">
    <!-- ─── Outils : index des décennies, ordre ─── -->
    <div class="@container shrink-0 pl-4 md:pl-8 pr-4 md:pr-14 py-3">
      <div class="flex flex-wrap items-center gap-2.5">
        <nav class="flex flex-wrap items-center gap-1.5" aria-label={$t("years_view.jump")}>
          {#each decennies as dec (dec.d)}
            <button
              type="button"
              class="{puce} cursor-pointer border-(--rg-bd) bg-(--rg-creux) text-(--rg-tx2) hover:border-(--rg-bd2) hover:text-(--rg-tx)"
              title={$t("home.decade_name").replace("{d}", court(dec.d))}
              onclick={() => aller(dec.d)}
            >
              <span class="w-2 h-2 rounded-full rg-teinte-vive" style="--h: {teinteDe(dec.d)}"></span>{$t("home.decade_short").replace("{d}", court(dec.d))}
            </button>
          {/each}
        </nav>
        <span class="flex-1"></span>
        <MenuSelect
          value={recentes ? "recent" : "ancien"}
          options={[
            { value: "recent", label: $t("years_view.newest_first"), icon: "material-symbols:arrow-downward-rounded" },
            { value: "ancien", label: $t("years_view.oldest_first"), icon: "material-symbols:arrow-upward-rounded" },
          ]}
          onchange={(v) => (recentes = v === "recent")}
          labelClass="@max-xl:hidden"
          menuClass="w-60"
        />
      </div>
    </div>

    <!-- ─── Le déroulé ─── -->
    <div class="flex-1 relative min-h-0">
      <div class="absolute inset-0 overflow-y-auto scrollbar-app pl-4 md:pl-8 pr-10 md:pr-14 pb-16" bind:this={defilement}>
        {#each decennies as dec (dec.d)}
          {@const annee = choix[dec.d] ?? null}
          {@const liste = selection(dec)}
          {@const nom = $t("home.decade_name").replace("{d}", court(dec.d))}
          {@const lien = `/library/${libraryId}/years/${dec.d}${annee == null ? "" : `?annee=${annee}`}`}
          <section id="decennie-{dec.d}" class="scroll-mt-2 pt-6 pb-9 border-b border-(--rg-line) last:border-b-0" use:monter={dec.d}>
            <!-- En-tête : vignette teintée, nom, chiffres, mix. -->
            <div class="flex flex-wrap items-center gap-x-4 gap-y-3">
              <span class="rg-icone-teinte shrink-0 w-14 h-14 flex items-center justify-center rounded-2xl text-xl font-extrabold tracking-tight" style="--h: {teinteDe(dec.d)}">
                {court(dec.d)}
              </span>
              <div class="min-w-0">
                <h2 class="text-[26px] leading-tight font-extrabold tracking-[-0.01em] text-(--rg-tx)"><a href={lien} class="hover:underline">{nom}</a></h2>
                <p class="text-[13px] text-(--rg-mu)">
                  <b class="font-semibold text-(--rg-tx2)">{nombre(dec.albums.length)}</b> {plusieurs(dec.albums.length, "library_head.albums_one", "library_head.albums_n")}
                  · <b class="font-semibold text-(--rg-tx2)">{nombre(dec.titres)}</b> {plusieurs(dec.titres, "library_head.tracks_one", "library_head.tracks_n")}
                  · <b class="font-semibold text-(--rg-tx2)">{nombre(dec.artistes)}</b> {plusieurs(dec.artistes, "library_head.artists_one", "library_head.artists_n")}
                </p>
              </div>
              <span class="flex-1"></span>
              <a
                href={lien}
                title={$t("period_view.explore_title")}
                class="h-10 px-4 inline-flex items-center gap-2 rounded-full border text-sm font-bold transition-colors
                       border-(--rg-bd2) text-(--rg-tx) hover:border-(--rg-gbd) hover:text-(--rg-gtx)"
              >
                <Icon icon="material-symbols:explore-outline-rounded" width="19" />{$t("period_view.explore")}
              </a>
              <button
                type="button"
                class="h-10 pl-3.5 pr-4.5 inline-flex items-center gap-2 rounded-full cursor-pointer text-sm font-bold
                       bg-(--rg-g) text-(--rg-on-g) hover:brightness-110 transition"
                onclick={() => handlePeriodeMix(libraryId, dec.d, annee)}
              >
                <Icon icon="material-symbols:play-arrow-rounded" width="22" />
                {annee == null ? $t("years_view.mix_decade").replace("{d}", court(dec.d)) : $t("years_view.mix_year").replace("{y}", String(annee))}
              </button>
            </div>

            <div class="mt-4">
              <YearChips decennie={dec.d} {annee} total={dec.albums.length} label={nom}
                         compte={(y) => dec.parAnnee.get(y)?.length ?? 0} onchoisir={(y) => (choix[dec.d] = y)} />
            </div>

            <!-- Les albums de la sélection. -->
            {#if montees.has(dec.d)}
              {#key `${annee}|${recentes}`}
                <Carousel axe="88px" class="mt-5 gap-4.5 pb-1">
                  {#each liste.slice(0, annee == null ? PLAFOND : liste.length) as a (a.id)}
                    <div class="w-44 shrink-0 snap-start"><AlbumListItem {libraryId} album={a} /></div>
                  {/each}
                </Carousel>
              {/key}
            {:else}
              <div class="mt-5 h-[248px]"></div>
            {/if}
            <!-- Tout voir : la page de la période, albums, morceaux et artistes. -->
            <a href={lien} class="mt-3 inline-flex items-center gap-1 text-[13px] font-semibold text-(--rg-gtx) hover:underline">
              {annee != null
                ? $t("years_view.see_year").replace("{y}", String(annee))
                : $t("period_view.see_albums").replace("{n}", nombre(liste.length))}<span aria-hidden="true">→</span>
            </a>
          </section>
        {/each}
      </div>
    </div>
  </div>
{/if}
