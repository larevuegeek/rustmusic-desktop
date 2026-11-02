<script lang="ts">
// Page d'un compositeur (tag `composer`) : ses albums, ses morceaux ou ses interprètes (`?vue=`), comme un genre.
import { page } from "$app/state";
import { goto } from "$app/navigation";
import Icon from "@iconify/svelte";
import { untrack } from "svelte";
import { invoke } from "@tauri-apps/api/core";
import { t, currentLocale } from "#lib/i18n";
import { libraryContentStore } from "#lib/stores/library/libraryContent.store";
import { selectionStore } from "#lib/stores/ui/selection.store";
import { viewMode } from "#lib/stores/ui/viewMode.store";
import { settingsStore } from "#lib/stores/settings/settings.store";
import { handleTracksPlay } from "#lib/actions/queue/QueueAction";
import { shownKeys, resolveColumns, readWidths, resetTagCache, sortTracks, type SortDir } from "#lib/config/trackColumns";
import { listenDuration } from "#lib/helper/tools/dateTools";
import { sortKey, comparerNaturel } from "#lib/helper/library/sortKey";
import CoverImg from "#lib/components/ui/image/CoverImg.svelte";
import ArtistAvatar from "#lib/components/ui/image/ArtistAvatar.svelte";
import DetailPage from "#lib/components/ui/layout/DetailPage.svelte";
import PageState from "#lib/components/ui/layout/PageState.svelte";
import PlayButton from "#lib/components/ui/button/PlayButton.svelte";
import RoundButton from "#lib/components/ui/button/RoundButton.svelte";
import ToolbarButton from "#lib/components/ui/button/ToolbarButton.svelte";
import ChipLink from "#lib/components/ui/link/ChipLink.svelte";
import SegmentedControl from "#lib/components/ui/input/SegmentedControl.svelte";
import ViewModeSwitch from "#lib/components/ui/input/ViewModeSwitch.svelte";
import SelectionToggle from "#lib/components/ui/selection/SelectionToggle.svelte";
import TrackColumnsButton from "#lib/components/library/track/TrackColumnsButton.svelte";
import AlbumTrackRow from "#lib/components/library/album/AlbumTrackRow.svelte";
import LibraryTrackTable from "#lib/components/library/track/LibraryTrackTable.svelte";
import AlbumListItem from "#lib/components/library/album/AlbumListItem.svelte";
import AlbumListRow from "#lib/components/library/album/AlbumListRow.svelte";
import ArtistListItem from "#lib/components/library/artist/ArtistListItem.svelte";
import ArtistListRow from "#lib/components/library/artist/ArtistListRow.svelte";
import type { ArtistListView } from "#lib/types/ui/library/artist/ArtistListView";
import type { TrackListView } from "#lib/types/ui/library/track/TrackListView";

const libraryId = $derived(Number(page.params.library_id));
// Déjà décodé par SvelteKit.
const nom = $derived(String((page.params as Record<string, string>).composer ?? ""));

let tracks = $state<TrackListView[]>([]);
let charge = $state(false);
let erreur = $state(false);
let defilement = $state<HTMLDivElement | null>(null);
let tour = 0;

$effect(() => {
  const id = libraryId;
  const n = nom;
  if (!id || !n) return;
  const k = ++tour;
  untrack(() => load(id, n, k));
});

async function load(id: number, n: string, k: number) {
  charge = false;
  erreur = false;
  tracks = [];
  try {
    const r = await invoke<TrackListView[]>("get_tracks_by_composer", { libraryId: id, name: n });
    if (k !== tour) return;
    resetTagCache();
    tracks = r ?? [];
    defilement?.scrollTo({ top: 0 });
  } catch (e) {
    if (k === tour) erreur = true;
    console.error("[compositeur] chargement :", e);
  } finally {
    if (k === tour) charge = true;
  }
}

// Le compositeur est aussi un artiste de la bibliothèque : sa photo et un lien vers sa page.
const commeArtiste = $derived($libraryContentStore.artists.find((a) => a.name.trim().toLowerCase() === nom.trim().toLowerCase()) ?? null);
// Sinon sa photo de compositeur : connue, ou cherchée sur Deezer à la première visite.
let photoTrouvee = $state<string | null>(null);
$effect(() => {
  const n = nom;
  photoTrouvee = null;
  if (!n || commeArtiste?.thumbnail_path) return;
  let perime = false;
  invoke<string | null>("fetch_composer_image", { name: n })
    .then((p) => { if (!perime) photoTrouvee = p; })
    .catch(() => {});
  return () => { perime = true; };
});
const photo = $derived(commeArtiste?.thumbnail_path ?? photoTrouvee);

// ─── Albums et interprètes (tirés de ses morceaux) ───
const byTitle = (x: string, y: string) => comparerNaturel.compare(sortKey(x), sortKey(y));
const albums = $derived.by(() => {
  const ids = new Set(tracks.map((x) => x.album_id).filter(Boolean));
  return $libraryContentStore.albums.filter((a) => ids.has(a.id)).sort((x, y) => (y.year ?? 0) - (x.year ?? 0) || byTitle(x.title, y.title));
});
type Vue = "albums" | "morceaux" | "interpretes";
const vue = $derived.by((): Vue => {
  const v = page.url.searchParams.get("vue");
  return v === "morceaux" || v === "interpretes" || v === "albums" ? v : albums.length ? "albums" : "morceaux";
});
function setVue(v: string) {
  const url = new URL(page.url.href);
  url.searchParams.set("vue", v);
  goto(url.pathname + url.search, { replace: true, reset: false });
}

// Les plus présents d'abord, sans le compositeur lui-même.
const interpretes = $derived.by((): ArtistListView[] => {
  const n = new Map<string, number>();
  for (const x of tracks) if (x.artist_id && x.artist_id !== commeArtiste?.id) n.set(x.artist_id, (n.get(x.artist_id) ?? 0) + 1);
  const parId = new Map($libraryContentStore.artists.map((a) => [a.id, a]));
  return [...n.entries()].sort((a, b) => b[1] - a[1]).map(([id]) => parId.get(id)).filter((a): a is ArtistListView => !!a);
});

const formatNumber = (n: number) => n.toLocaleString($currentLocale);
const plural = (n: number, un: string, n_: string) => $t(n === 1 ? un : n_);
const duree = $derived(tracks.reduce((s, x) => s + (x.duration ?? 0), 0));

// ─── Tous les titres ───
let tri = $state<string>("album");
let sens = $state<SortDir>("asc");
const tries = $derived(sortTracks(tracks, tri, sens));
const LOT = 200;
let affiches = $state(LOT);
$effect(() => {
  void nom;
  void vue;
  affiches = LOT;
});
const visibles = $derived(tries.slice(0, affiches));

function changeSort(cle: string) {
  if (tri === cle) sens = sens === "asc" ? "desc" : "asc";
  else {
    tri = cle;
    sens = cle === "year" ? "desc" : "asc";
  }
}
const sortArrow = (cle: string) => (tri === cle ? (sens === "asc" ? "material-symbols:arrow-upward-rounded" : "material-symbols:arrow-downward-rounded") : null);

const grille = $derived($viewMode !== "list");
$effect(() => {
  if (vue === "morceaux" && grille) selectionStore.setOrder(visibles.map((x) => ({ id: x.id, track: x })));
});

const colonnes = $derived(resolveColumns(shownKeys($settingsStore.track_columns)));
const largeurs = $derived(readWidths($settingsStore.track_column_widths));
const grilleCartes = "grid grid-cols-[repeat(auto-fill,minmax(176px,1fr))] gap-x-4.5 gap-y-6";
</script>

{#snippet portrait(taille: string, texte: string)}
  <div class="relative {taille} shrink-0 rounded-full overflow-hidden bg-(--rg-s2) shadow-[0_24px_60px_rgba(0,0,0,0.3)] dark:shadow-[0_24px_60px_rgba(0,0,0,0.55)]">
    {#if photo}
      <CoverImg path={photo} alt={nom} size="2x" class="w-full h-full object-cover" />
    {:else}
      <ArtistAvatar name={nom} deux class="w-full h-full tracking-[-0.02em] {texte}" />
    {/if}
  </div>
{/snippet}

{#if erreur}
  <PageState icon="material-symbols:music-note-rounded" message={$t("composers_view.error")} lien={{ href: `/library/${libraryId}/artists`, label: $t("composers_view.back") }} />

{:else if !charge && tracks.length === 0}
  <PageState chargement message={$t("composers_view.loading")} />

{:else}
<DetailPage bind:defilement image={photo ?? albums[0]?.cover_url ?? null} retourHref={`/library/${libraryId}/artists`} retourLabel={$t("composers_view.back")}
            playLabel={$t("artist_view.play_all")} onplay={() => handleTracksPlay(tries)}>
  {#snippet mini()}
    {@render portrait("w-8.5 h-8.5", "text-[12px]")}
    <b class="truncate text-[15px] text-(--rg-tx)">{nom}</b>
  {/snippet}
  {#snippet barre()}<SelectionToggle /><ViewModeSwitch class="bg-(--rg-carte)/80" />{/snippet}

    <!-- ─── En-tête ─── -->
    <div class="relative flex flex-wrap items-end gap-8 pt-3 pb-7">
      {@render portrait("w-55 h-55 @max-[760px]:w-40 @max-[760px]:h-40", "text-[80px] @max-[760px]:text-[56px]")}

      <div class="flex-[1_1_320px] min-w-0 flex flex-col gap-3">
        <div class="flex items-center gap-2 text-xs font-bold tracking-[0.1em] uppercase text-(--rg-tx2)">
          <Icon icon="material-symbols:music-note-rounded" width="15" class="text-(--rg-g)" />{$t("composers_view.kicker")}
        </div>
        <h1 class="font-extrabold leading-[1.12] tracking-[-0.03em] text-balance line-clamp-2 text-(--rg-tx)
                   {nom.length > 24 ? 'text-[56px] @max-[760px]:text-[34px]' : 'text-[72px] @max-[760px]:text-[44px]'}" title={nom}>{nom}</h1>

        <div class="flex flex-wrap items-center gap-x-3 gap-y-2 text-[15px] text-(--rg-mu)">
          {#if albums.length}
            <span><b class="font-bold text-(--rg-tx)">{formatNumber(albums.length)}</b> {plural(albums.length, "library_head.albums_one", "library_head.albums_n")}</span>
            <span class="text-(--rg-mu2)">·</span>
          {/if}
          <span><b class="font-bold text-(--rg-tx)">{formatNumber(tracks.length)}</b> {plural(tracks.length, "library_head.tracks_one", "library_head.tracks_n")}</span>
          {#if interpretes.length}
            <span class="text-(--rg-mu2)">·</span>
            <span><b class="font-bold text-(--rg-tx)">{formatNumber(interpretes.length)}</b> {plural(interpretes.length, "composers_view.performers_one", "composers_view.performers_n")}</span>
          {/if}
          {#if duree}
            <span class="text-(--rg-mu2)">·</span>
            <span>{listenDuration(duree)}</span>
          {/if}
        </div>

        <div class="flex flex-wrap items-center gap-2.5 mt-2">
          <PlayButton label={$t("artist_view.play_all")} disabled={!tracks.length} onclick={() => handleTracksPlay(tries)} />
          <RoundButton icon="material-symbols:shuffle-rounded" title={$t("album_view.shuffle")} disabled={!tracks.length} onclick={() => handleTracksPlay(tracks, true)} />
          <div class="flex flex-wrap gap-1.5 ml-1.5 @max-[760px]:ml-0">
            {#if commeArtiste}
              <ChipLink href={`/library/${libraryId}/artists/${commeArtiste.id}`} label={$t("composers_view.artist_page")} title={commeArtiste.name} />
            {/if}
            {#each interpretes.slice(0, 3) as a (a.id)}<ChipLink href={`/library/${libraryId}/artists/${a.id}`} label={a.name} title={$t("composers_view.performer_title")} />{/each}
          </div>
        </div>
      </div>
    </div>

    <!-- ─── Albums, morceaux ou interprètes ─── -->
    <div class="flex flex-wrap items-center gap-2.5 pb-4 border-t border-(--rg-line) pt-5">
      <SegmentedControl
        label={$t("period_view.explore")}
        value={vue}
        onchange={setVue}
        options={[
          { value: "albums", label: `${$t("library.albums")} · ${formatNumber(albums.length)}` },
          { value: "morceaux", label: `${$t("library.tracks")} · ${formatNumber(tracks.length)}` },
          { value: "interpretes", label: `${$t("composers_view.performers")} · ${formatNumber(interpretes.length)}` },
        ]}
      />
      <span class="flex-1"></span>
      {#if vue === "morceaux"}
        <SegmentedControl variant="discret" label={$t("common.sort")} value={tri} onchange={changeSort}
                          options={[
                            { value: "album", label: $t("composers_view.sort_album"), after: sortArrow("album") },
                            { value: "title", label: $t("artist_view.sort_title"), after: sortArrow("title") },
                            { value: "artist", label: $t("genre_view.sort_artist"), after: sortArrow("artist") },
                            { value: "year", label: $t("artist_view.sort_year"), after: sortArrow("year") },
                          ]} />
        {#if !grille}<TrackColumnsButton {libraryId} avecLibelle />{/if}
      {/if}
    </div>

    {#if vue === "albums"}
      {#if albums.length === 0}
        <p class="py-10 text-center text-sm text-(--rg-mu)">{$t("composers_view.empty")}</p>
      {:else if grille}
        <div class={grilleCartes}>
          {#each albums as a (a.id)}<AlbumListItem {libraryId} album={a} />{/each}
        </div>
      {:else}
        <div class="flex flex-col">
          {#each albums as a (a.id)}<AlbumListRow {libraryId} album={a} />{/each}
        </div>
      {/if}

    {:else if vue === "interpretes"}
      <p class="-mt-2 mb-4 text-[13px] text-(--rg-mu)">{$t("composers_view.performers_desc")}</p>
      {#if grille}
        <div class={grilleCartes}>
          {#each interpretes as a (a.id)}<ArtistListItem {libraryId} artist={a} />{/each}
        </div>
      {:else}
        <div class="flex flex-col">
          {#each interpretes as a (a.id)}<ArtistListRow {libraryId} artist={a} />{/each}
        </div>
      {/if}

    {:else}
    <section>
      {#if tracks.length === 0}
        <p class="py-10 text-center text-sm text-(--rg-mu)">{$t("composers_view.empty")}</p>
      {:else if grille}
        <div class="grid grid-cols-2 grid-flow-col gap-x-7 gap-y-0.5 grid-rows-[repeat(var(--rangs),auto)]
                    @max-[900px]:grid-cols-1 @max-[900px]:grid-flow-row @max-[900px]:grid-rows-none"
             style="--rangs: {Math.ceil(visibles.length / 2)}">
          {#each visibles as track, i (track.id)}
            <AlbumTrackRow {libraryId} {track} tracks={tries} jaquette position={i + 1} numero={i + 1}
                           sousTitre={[track.artist, track.album].filter(Boolean).join(" · ")} />
          {/each}
        </div>
      {:else}
        <LibraryTrackTable {libraryId} tracks={visibles} columns={colonnes} {largeurs} barreHorizontale sortKey={tri} sortDir={sens} colle={60}
                           onsort={(cle, dir) => { tri = cle; sens = dir; }} />
      {/if}

      {#if tries.length > affiches}
        <div class="flex justify-center mt-5">
          <ToolbarButton icon="material-symbols:expand-more-rounded" label={$t("genre_view.show_more").replace("{n}", formatNumber(Math.min(LOT, tries.length - affiches)))}
                         onclick={() => (affiches += LOT)} />
        </div>
      {/if}
    </section>
    {/if}
</DetailPage>
{/if}
