<script lang="ts">
// Page d'une période : une décennie, ou l'une de ses années (`?annee=`). Albums, morceaux ou artistes (`?vue=`).
import { page } from "$app/state";
import { goto } from "$app/navigation";
import Icon from "@iconify/svelte";
import { untrack } from "svelte";
import { invoke } from "@tauri-apps/api/core";
import { t, currentLocale } from "$lib/i18n";
import { libraryContentStore } from "$lib/stores/library/libraryContent.store";
import { selectionStore } from "$lib/stores/ui/selection.store";
import { viewMode } from "$lib/stores/ui/viewMode.store";
import { settingsStore } from "$lib/stores/settings/settings.store";
import { handleTracksPlay } from "$lib/actions/queue/QueueAction";
import { clesAffichees, resoudreColonnes, lireLargeurs, resetTagCache, trierPistes, type SortDir } from "$lib/config/trackColumns";
import { dureeEcoute } from "$lib/helper/tools/dateTools";
import { cleTri, comparerNaturel } from "$lib/helper/library/cleTri";
import CoverImg from "$lib/components/ui/image/CoverImg.svelte";
import DetailPage from "$lib/components/ui/layout/DetailPage.svelte";
import PageState from "$lib/components/ui/layout/PageState.svelte";
import PlayButton from "$lib/components/ui/button/PlayButton.svelte";
import RoundButton from "$lib/components/ui/button/RoundButton.svelte";
import ToolbarButton from "$lib/components/ui/button/ToolbarButton.svelte";
import SegmentedControl from "$lib/components/ui/input/SegmentedControl.svelte";
import ViewModeSwitch from "$lib/components/ui/input/ViewModeSwitch.svelte";
import SelectionToggle from "$lib/components/ui/selection/SelectionToggle.svelte";
import TrackColumnsButton from "$lib/components/library/track/TrackColumnsButton.svelte";
import AlbumTrackRow from "$lib/components/library/album/AlbumTrackRow.svelte";
import LibraryTrackTable from "$lib/components/library/track/LibraryTrackTable.svelte";
import AlbumListItem from "$lib/components/library/album/AlbumListItem.svelte";
import AlbumListRow from "$lib/components/library/album/AlbumListRow.svelte";
import ArtistListItem from "$lib/components/library/artist/ArtistListItem.svelte";
import ArtistListRow from "$lib/components/library/artist/ArtistListRow.svelte";
import type { ArtistListView } from "$lib/types/ui/library/artist/ArtistListView";
import type { TrackListView } from "$lib/types/ui/library/track/TrackListView";
import YearChips from "$lib/components/ui/input/YearChips.svelte";
import { estDatee, decennieDe, courtDecennie as court, teinteDecennie } from "$lib/helper/library/periode";

const libraryId = $derived(Number(page.params.library_id));
// Une décennie illisible ou d'avant 1900 vaut 0 : la page affiche son erreur.
const decennie = $derived.by(() => {
  const d = Number(page.params.decennie);
  return estDatee(d) ? decennieDe(d) : 0;
});
// L'année et la vue vivent dans l'adresse : elles se partagent et survivent au retour.
const annee = $derived.by(() => {
  const y = Number(page.url.searchParams.get("annee"));
  return y >= decennie && y <= decennie + 9 ? y : null;
});
type Vue = "albums" | "morceaux" | "artistes";
const vue = $derived.by((): Vue => {
  const v = page.url.searchParams.get("vue");
  return v === "morceaux" || v === "artistes" ? v : "albums";
});
const debut = $derived(annee ?? decennie);
const fin = $derived(annee ?? decennie + 9);

function changer(cle: string, valeur: string | null) {
  const url = new URL(page.url);
  if (valeur) url.searchParams.set(cle, valeur);
  else url.searchParams.delete(cle);
  goto(url.pathname + url.search, { replaceState: true, keepFocus: true, noScroll: true });
}

// ─── Les titres de la période ───
let tracks = $state<TrackListView[]>([]);
let charge = $state(false);
let erreur = $state(false);
let tour = 0;

$effect(() => {
  const [id, a, b] = [libraryId, debut, fin];
  if (!a) {
    erreur = true;
    return;
  }
  if (!id) return;
  const n = ++tour;
  // La sélection portait sur l'ancienne période.
  untrack(() => selectionStore.stop());
  untrack(() => charger(id, a, b, n));
});

async function charger(id: number, a: number, b: number, n: number) {
  charge = false;
  erreur = false;
  try {
    const r = await invoke<TrackListView[]>("get_tracks_by_years", { libraryId: id, debut: a, fin: b });
    if (n !== tour) return;
    resetTagCache();
    tracks = r ?? [];
  } catch (e) {
    if (n === tour) erreur = true;
    console.error("[période] chargement :", e);
  } finally {
    if (n === tour) charge = true;
  }
}

// ─── Albums (déjà en mémoire) ───
const parTitre = (x: string, y: string) => comparerNaturel.compare(cleTri(x), cleTri(y));
const albumsDecennie = $derived($libraryContentStore.albums.filter((a) => a.year != null && a.year >= decennie && a.year <= decennie + 9));
const parAnnee = $derived.by(() => {
  const m = new Map<number, number>();
  for (const a of albumsDecennie) m.set(a.year!, (m.get(a.year!) ?? 0) + 1);
  return m;
});

type TriAlbums = "year" | "title" | "artist";
let triAlbums = $state<TriAlbums>("year");
const albums = $derived(
  albumsDecennie
    .filter((a) => a.year! >= debut && a.year! <= fin)
    .sort((x, y) =>
      triAlbums === "title" ? parTitre(x.title, y.title)
      : triAlbums === "artist" ? parTitre(x.artist ?? "", y.artist ?? "") || (y.year! - x.year!)
      : (y.year! - x.year!) || parTitre(x.artist ?? "", y.artist ?? "") || parTitre(x.title, y.title)),
);

// ─── Artistes : les plus présents sur la période ───
const titresPar = $derived.by(() => {
  const n = new Map<string, number>();
  for (const x of tracks) if (x.artist_id) n.set(x.artist_id, (n.get(x.artist_id) ?? 0) + 1);
  return n;
});
const artistes = $derived.by((): ArtistListView[] => {
  const parId = new Map($libraryContentStore.artists.map((a) => [a.id, a]));
  return [...titresPar.entries()]
    .sort((a, b) => b[1] - a[1])
    .map(([id]) => parId.get(id))
    .filter((a): a is ArtistListView => !!a);
});

// ─── Chiffres et teinte ───
const h = $derived(teinteDecennie(decennie));
const titre = $derived(annee != null ? String(annee) : $t("home.decade_name").replace("{d}", court(decennie)));
// L'en-tête ne suit pas le tri choisi : les plus récents de la période.
const pochettes = $derived(
  albumsDecennie
    .filter((a) => a.year! >= debut && a.year! <= fin)
    .sort((x, y) => y.year! - x.year!)
    .map((a) => a.cover_url)
    .filter((c): c is string => !!c)
    .slice(0, 4),
);
const nombre = (n: number) => n.toLocaleString($currentLocale);
const plusieurs = (n: number, un: string, n_: string) => $t(n === 1 ? un : n_);
const duree = $derived(tracks.reduce((s, x) => s + (x.duration ?? 0), 0));

// ─── Morceaux ───
let tri = $state<string>("year");
let sens = $state<SortDir>("desc");
const tries = $derived(trierPistes(tracks, tri, sens));
// Une décennie compte vite des milliers de titres : un lot d'abord, le reste à la demande.
const LOT = 200;
let affiches = $state(LOT);
$effect(() => {
  void debut;
  void vue;
  affiches = LOT;
});
const visibles = $derived(tries.slice(0, affiches));

function trierPar(cle: string) {
  if (tri === cle) sens = sens === "asc" ? "desc" : "asc";
  else {
    tri = cle;
    sens = cle === "year" || cle === "created_at" ? "desc" : "asc";
  }
}
const flecheSens = (cle: string) => (tri === cle ? (sens === "asc" ? "material-symbols:arrow-upward-rounded" : "material-symbols:arrow-downward-rounded") : null);

const grille = $derived($viewMode !== "list");
$effect(() => {
  if (vue === "morceaux" && grille) selectionStore.setOrder(visibles.map((x) => ({ id: x.id, track: x })));
});

const colonnes = $derived(resoudreColonnes(clesAffichees($settingsStore.track_columns)));
const largeurs = $derived(lireLargeurs($settingsStore.track_column_widths));

const grilleCartes = "grid grid-cols-[repeat(auto-fill,minmax(176px,1fr))] gap-x-4.5 gap-y-6";
</script>

<!-- Quatre pochettes de la période, sinon des dégradés dans sa teinte avec son nombre. -->
{#snippet mosaique(taille: string, petit: boolean)}
  <div class="relative {taille} shrink-0 rounded-[14px] overflow-hidden grid grid-cols-2 grid-rows-2 gap-0.5 isolate bg-(--rg-bg)
              shadow-[0_24px_60px_rgba(0,0,0,0.3)] dark:shadow-[0_24px_60px_rgba(0,0,0,0.55)]">
    {#each [0, 1, 2, 3] as k (k)}
      {#if pochettes[k]}
        <CoverImg path={pochettes[k]} alt="" size="2x" class="w-full h-full object-cover" />
      {:else}
        <span style="background: linear-gradient({135 + k * 40}deg, oklch({0.55 - k * 0.06} 0.1 {(h + k * 23) % 360}), oklch(0.2 0.04 {(h + k * 23) % 360}))"></span>
      {/if}
    {/each}
    {#if !petit}
      <span class="absolute inset-0 bg-linear-to-t from-black/70 via-black/10 to-transparent pointer-events-none"></span>
      <span class="absolute left-4 bottom-3 text-white font-extrabold leading-none tracking-[-0.03em] text-[44px] @max-[760px]:text-[32px]">
        {annee ?? court(decennie)}
      </span>
    {/if}
    <span class="absolute inset-0 rounded-[14px] ring-1 ring-inset ring-black/5 dark:ring-white/6 pointer-events-none"></span>
  </div>
{/snippet}

{#if erreur}
  <PageState icon="material-symbols:calendar-month-outline-rounded" message={$t("period_view.error")} lien={{ href: `/library/${libraryId}/years`, label: $t("period_view.back") }} />

{:else if !charge && tracks.length === 0}
  <PageState chargement message={$t("period_view.loading")} />

{:else}
<DetailPage image={pochettes[0] ?? null} retourHref={`/library/${libraryId}/years`} retourLabel={$t("period_view.back")}
            playLabel={$t("artist_view.play_all")} onplay={() => handleTracksPlay(tries)}>
  {#snippet mini()}
    {@render mosaique("w-8.5 h-8.5 rounded-[7px]!", true)}
    <b class="truncate text-[15px] text-(--rg-tx)">{titre}</b>
  {/snippet}
  {#snippet barre()}<ViewModeSwitch class="bg-(--rg-carte)/80" />{/snippet}

    <!-- ─── En-tête ─── -->
    <div class="relative flex flex-wrap items-end gap-8 pt-3 pb-6" style="--gc: oklch(0.7 0.17 {h})">
      {@render mosaique("w-55 h-55 @max-[760px]:w-40 @max-[760px]:h-40", false)}

      <div class="flex-[1_1_320px] min-w-0 flex flex-col gap-3">
        <div class="flex items-center gap-2 text-xs font-bold tracking-[0.1em] uppercase text-(--rg-tx2)">
          <span class="w-5 h-0.75 rounded-full bg-(--gc)"></span>{$t(annee != null ? "period_view.kicker_year" : "period_view.kicker_decade")}
        </div>
        <h1 class="font-extrabold leading-none tracking-[-0.03em] text-(--rg-tx) text-[72px] @max-[760px]:text-[44px]">{titre}</h1>

        <!-- Estompés tant que les titres de la nouvelle période se chargent. -->
        <div class="flex flex-wrap items-center gap-x-3 gap-y-2 text-[15px] text-(--rg-mu) transition-opacity {charge ? '' : 'opacity-50'}">
          <span><b class="font-bold text-(--rg-tx)">{nombre(albums.length)}</b> {plusieurs(albums.length, "library_head.albums_one", "library_head.albums_n")}</span>
          <span class="text-(--rg-mu2)">·</span>
          <span><b class="font-bold text-(--rg-tx)">{nombre(tracks.length)}</b> {plusieurs(tracks.length, "library_head.tracks_one", "library_head.tracks_n")}</span>
          {#if artistes.length}
            <span class="text-(--rg-mu2)">·</span>
            <span><b class="font-bold text-(--rg-tx)">{nombre(artistes.length)}</b> {plusieurs(artistes.length, "library_head.artists_one", "library_head.artists_n")}</span>
          {/if}
          {#if duree}
            <span class="text-(--rg-mu2)">·</span>
            <span>{dureeEcoute(duree)}</span>
          {/if}
        </div>

        <div class="flex flex-wrap items-center gap-2.5 mt-2">
          <PlayButton label={$t("artist_view.play_all")} disabled={!tracks.length} onclick={() => handleTracksPlay(tries)} />
          <RoundButton icon="material-symbols:shuffle-rounded" title={$t("album_view.shuffle")} disabled={!tracks.length} onclick={() => handleTracksPlay(tracks, true)} />
        </div>
      </div>
    </div>

    <div class="pb-6">
      <YearChips {decennie} {annee} total={albumsDecennie.length} label={$t("home.decade_name").replace("{d}", court(decennie))}
                 compte={(y) => parAnnee.get(y) ?? 0} onchoisir={(y) => changer("annee", y == null ? null : String(y))} />
    </div>

    <!-- ─── Albums, morceaux ou artistes ─── -->
    <div class="flex flex-wrap items-center gap-2.5 pb-4 border-t border-(--rg-line) pt-5">
      <SegmentedControl
        label={$t("period_view.explore")}
        value={vue}
        onchange={(v) => changer("vue", v === "albums" ? null : v)}
        options={[
          { value: "albums", label: `${$t("library.albums")} · ${nombre(albums.length)}` },
          { value: "morceaux", label: `${$t("library.tracks")} · ${nombre(tracks.length)}` },
          { value: "artistes", label: `${$t("library.artists")} · ${nombre(artistes.length)}` },
        ]}
      />
      <span class="flex-1"></span>
      {#if vue === "albums"}
        <SegmentedControl variant="discret" label={$t("common.sort")} value={triAlbums} onchange={(v) => (triAlbums = v as TriAlbums)}
                          options={[
                            { value: "year", label: $t("period_view.sort_year") },
                            { value: "title", label: $t("period_view.sort_title") },
                            { value: "artist", label: $t("period_view.sort_artist") },
                          ]} />
      {:else if vue === "morceaux"}
        <SegmentedControl variant="discret" label={$t("common.sort")} value={tri} onchange={trierPar}
                          options={[
                            { value: "year", label: $t("artist_view.sort_year"), after: flecheSens("year") },
                            { value: "title", label: $t("artist_view.sort_title"), after: flecheSens("title") },
                            { value: "artist", label: $t("genre_view.sort_artist"), after: flecheSens("artist") },
                            { value: "created_at", label: $t("genre_view.sort_added"), after: flecheSens("created_at") },
                          ]} />
        {#if !grille}<TrackColumnsButton {libraryId} avecLibelle />{/if}
        <SelectionToggle avecLibelle />
      {/if}
    </div>

    {#if (vue === "albums" && albums.length === 0) || (vue === "artistes" && charge && artistes.length === 0)}
      <p class="py-10 text-center text-sm text-(--rg-mu)">{$t("period_view.empty")}</p>

    {:else if vue === "albums"}
      {#if grille}
        <div class={grilleCartes}>
          {#each albums as a (a.id)}<AlbumListItem {libraryId} album={a} />{/each}
        </div>
      {:else}
        <div class="flex flex-col">
          {#each albums as a (a.id)}<AlbumListRow {libraryId} album={a} />{/each}
        </div>
      {/if}

    {:else if vue === "artistes"}
      <p class="-mt-2 mb-4 text-[13px] text-(--rg-mu)">{$t("period_view.artists_desc")}</p>
      {#if grille}
        <div class={grilleCartes}>
          {#each artistes as a (a.id)}<ArtistListItem {libraryId} artist={a} />{/each}
        </div>
      {:else}
        <div class="flex flex-col">
          {#each artistes as a (a.id)}<ArtistListRow {libraryId} artist={a} />{/each}
        </div>
      {/if}

    {:else}
      {#if !charge}
        <div class="flex justify-center py-10 text-(--rg-mu)"><Icon icon="material-symbols:progress-activity" width="22" class="animate-spin" /></div>
      {:else if tracks.length === 0}
        <p class="py-10 text-center text-sm text-(--rg-mu)">{$t("period_view.empty")}</p>
      {:else if grille}
        <!-- Deux colonnes lues de haut en bas, une seule quand la place manque. -->
        <div class="grid grid-cols-2 grid-flow-col gap-x-7 gap-y-0.5 grid-rows-[repeat(var(--rangs),auto)]
                    @max-[900px]:grid-cols-1 @max-[900px]:grid-flow-row @max-[900px]:grid-rows-none"
             style="--rangs: {Math.ceil(visibles.length / 2)}">
          {#each visibles as track, i (track.id)}
            <AlbumTrackRow {libraryId} {track} tracks={tries} jaquette position={i + 1} numero={i + 1}
                           sousTitre={[track.artist, track.album, track.year?.slice(0, 4)].filter(Boolean).join(" · ")} />
          {/each}
        </div>
      {:else}
        <LibraryTrackTable {libraryId} tracks={visibles} columns={colonnes} {largeurs} barreHorizontale sortKey={tri} sortDir={sens} colle={60}
                           onsort={(cle, dir) => { tri = cle; sens = dir; }} />
      {/if}

      {#if tries.length > affiches}
        <div class="flex justify-center mt-5">
          <ToolbarButton icon="material-symbols:expand-more-rounded" label={$t("genre_view.show_more").replace("{n}", nombre(Math.min(LOT, tries.length - affiches)))}
                         onclick={() => (affiches += LOT)} />
        </div>
      {/if}
    {/if}
</DetailPage>
{/if}
