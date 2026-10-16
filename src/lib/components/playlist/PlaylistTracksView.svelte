<script lang="ts">
  // Page d'une playlist (rangée, auto, likés, récents) : la charpente des fiches, le tableau à colonnes et la sélection.
  import Icon from "@iconify/svelte";
  import type { Snippet } from "svelte";
  import { t, currentLocale } from "#lib/i18n";
  import { settingsStore } from "#lib/stores/settings/settings.store";
  import { selectionStore } from "#lib/stores/ui/selection.store";
  import { viewMode } from "#lib/stores/ui/viewMode.store";
  import { handleTracksPlay } from "#lib/actions/queue/QueueAction";
  import { clesAffichees, resoudreColonnes, lireLargeurs, trierPistes, type SortDir } from "#lib/config/trackColumns";
  import { dureeEcoute } from "#lib/helper/tools/dateTools";
  import CoverImg from "#lib/components/ui/image/CoverImg.svelte";
  import DetailPage from "#lib/components/ui/layout/DetailPage.svelte";
  import PlayButton from "#lib/components/ui/button/PlayButton.svelte";
  import RoundButton from "#lib/components/ui/button/RoundButton.svelte";
  import SegmentedControl from "#lib/components/ui/input/SegmentedControl.svelte";
  import ViewModeSwitch from "#lib/components/ui/input/ViewModeSwitch.svelte";
  import SelectionToggle from "#lib/components/ui/selection/SelectionToggle.svelte";
  import TrackColumnsButton from "#lib/components/library/track/TrackColumnsButton.svelte";
  import LibraryTrackTable from "#lib/components/library/track/LibraryTrackTable.svelte";
  import AlbumTrackRow from "#lib/components/library/album/AlbumTrackRow.svelte";
  import type { TrackListView } from "#lib/types/ui/library/track/TrackListView";

  let {
    title,
    kicker,
    subtitle = null,
    color,
    icon,
    tracks,
    loading,
    libraryId,
    actions,
    onremove,
    removeLabel,
    emptyTitle,
    emptyDesc,
  }: {
    title: string;
    kicker: string;
    /** Sous le titre : les règles d'une playlist auto. */
    subtitle?: string | null;
    color: string;
    icon: string;
    /** Dans l'ordre de la playlist. */
    tracks: TrackListView[];
    loading: boolean;
    libraryId: number;
    /** Boutons ronds après lecture et mélange (modifier, épingler, supprimer…). */
    actions?: Snippet;
    onremove?: (track: TrackListView) => void;
    removeLabel?: string;
    emptyTitle: string;
    emptyDesc: string;
  } = $props();

  // Quatre pochettes différentes pour la mosaïque, sinon la couleur et l'icône de la playlist.
  const covers = $derived([...new Set(tracks.map((x) => x.thumbnail_path).filter((c): c is string => !!c))].slice(0, 4));
  const duration = $derived(tracks.reduce((sum, x) => sum + (x.duration ?? 0), 0));
  const count = (n: number) => n.toLocaleString($currentLocale);
  const tracksLabel = (n: number) => $t(n === 1 ? "library_head.tracks_one" : "library_head.tracks_n");

  // ─── Tri : l'ordre de la playlist d'abord ───
  let sortKey = $state<string>("order");
  let sortDir = $state<SortDir>("asc");
  const sorted = $derived(sortKey === "order" ? (sortDir === "asc" ? tracks : [...tracks].reverse()) : trierPistes(tracks, sortKey, sortDir));
  function sortBy(key: string) {
    if (sortKey === key) sortDir = sortDir === "asc" ? "desc" : "asc";
    else {
      sortKey = key;
      sortDir = "asc";
    }
  }
  const arrow = (key: string) => (sortKey === key ? (sortDir === "asc" ? "material-symbols:arrow-upward-rounded" : "material-symbols:arrow-downward-rounded") : null);

  const columns = $derived(resoudreColonnes(clesAffichees($settingsStore.track_columns)));
  const widths = $derived(lireLargeurs($settingsStore.track_column_widths));
  const grid = $derived($viewMode !== "list");
  $effect(() => {
    if (grid) selectionStore.setOrder(sorted.map((x) => ({ id: x.id, track: x })));
  });
</script>

{#snippet mosaic(size: string, iconSize: number)}
  <div class="relative {size} shrink-0 rounded-[14px] overflow-hidden isolate shadow-[0_24px_60px_rgba(0,0,0,0.3)] dark:shadow-[0_24px_60px_rgba(0,0,0,0.55)]"
       style="background: linear-gradient(140deg, {color}, color-mix(in oklab, {color} 45%, black))">
    {#if covers.length >= 4}
      <div class="absolute inset-0 grid grid-cols-2 grid-rows-2 gap-0.5 bg-black/20">
        {#each covers as c (c)}<CoverImg path={c} alt="" size="2x" class="w-full h-full object-cover" />{/each}
      </div>
    {:else if covers.length > 0}
      <CoverImg path={covers[0]} alt="" size="2x" class="absolute inset-0 w-full h-full object-cover" />
    {:else}
      <span class="absolute inset-0 flex items-center justify-center text-white/90"><Icon {icon} width={iconSize} /></span>
    {/if}
    <span class="absolute inset-0 rounded-[14px] ring-1 ring-inset ring-black/5 dark:ring-white/6 pointer-events-none"></span>
  </div>
{/snippet}

<DetailPage image={covers[0] ?? null} retourHref="/playlists" retourLabel={$t("playlists_view.all")}
            playLabel={$t("artist_view.play_all")} onplay={() => handleTracksPlay(sorted)}>
  {#snippet mini()}
    {@render mosaic("w-8.5 h-8.5 rounded-[7px]!", 16)}
    <b class="truncate text-[15px] text-(--rg-tx)">{title}</b>
  {/snippet}
  {#snippet barre()}<SelectionToggle /><ViewModeSwitch class="bg-(--rg-carte)/80" />{/snippet}

  <!-- ─── En-tête ─── -->
  <div class="relative flex flex-wrap items-end gap-8 pt-3 pb-7">
    {@render mosaic("w-55 h-55 @max-[760px]:w-40 @max-[760px]:h-40", 64)}

    <div class="flex-[1_1_320px] min-w-0 flex flex-col gap-3">
      <div class="flex items-center gap-2 text-xs font-bold tracking-[0.1em] uppercase text-(--rg-tx2)">
        <span class="w-5 h-0.75 rounded-full" style="background: {color}"></span>{kicker}
      </div>
      <h1 class="font-extrabold leading-none tracking-[-0.03em] text-balance line-clamp-2 text-(--rg-tx)
                 {title.length > 24 ? 'text-[56px] @max-[760px]:text-[34px]' : 'text-[72px] @max-[760px]:text-[44px]'}" {title}>{title}</h1>
      {#if subtitle}<p class="text-[14px] leading-snug text-(--rg-mu) line-clamp-2" title={subtitle}>{subtitle}</p>{/if}

      <div class="flex flex-wrap items-center gap-x-3 gap-y-2 text-[15px] text-(--rg-mu)">
        <span><b class="font-bold text-(--rg-tx)">{count(tracks.length)}</b> {tracksLabel(tracks.length)}</span>
        {#if duration}
          <span class="text-(--rg-mu2)">·</span>
          <span>{dureeEcoute(duration)}</span>
        {/if}
      </div>

      <div class="flex flex-wrap items-center gap-2.5 mt-2">
        <PlayButton label={$t("artist_view.play_all")} disabled={!tracks.length} onclick={() => handleTracksPlay(sorted)} />
        <RoundButton icon="material-symbols:shuffle-rounded" title={$t("album_view.shuffle")} disabled={!tracks.length} onclick={() => handleTracksPlay(tracks, true)} />
        {#if actions}{@render actions()}{/if}
      </div>
    </div>
  </div>

  <!-- ─── Titres ─── -->
  <section class="mt-6">
    <div class="flex flex-wrap items-center gap-2.5 pt-1 pb-3.5">
      <span class="mr-auto text-[13px] text-(--rg-mu)"><b class="font-semibold text-(--rg-tx2)">{count(tracks.length)}</b> {tracksLabel(tracks.length)}{#if duration}{" · "}{dureeEcoute(duration)}{/if}</span>
      <SegmentedControl variant="discret" label={$t("common.sort")} value={sortKey} onchange={sortBy}
                        options={[
                          { value: "order", label: $t("playlist_page.sort_order"), after: arrow("order") },
                          { value: "title", label: $t("artist_view.sort_title"), after: arrow("title") },
                          { value: "artist", label: $t("genre_view.sort_artist"), after: arrow("artist") },
                          { value: "album", label: $t("playlist_page.sort_album"), after: arrow("album") },
                        ]} />
      {#if !grid}<TrackColumnsButton {libraryId} avecLibelle />{/if}
    </div>

    {#if loading}
      <div class="flex justify-center py-10 text-(--rg-mu)"><Icon icon="material-symbols:progress-activity" width="22" class="animate-spin" /></div>
    {:else if tracks.length === 0}
      <div class="flex flex-col items-center py-16 text-center">
        <span class="w-16 h-16 mb-4 rounded-full flex items-center justify-center" style="background: color-mix(in oklab, {color} 14%, transparent); color: {color}">
          <Icon {icon} width="30" />
        </span>
        <p class="text-[17px] font-bold text-(--rg-tx)">{emptyTitle}</p>
        <p class="mt-1.5 max-w-sm text-sm leading-relaxed text-(--rg-mu)">{emptyDesc}</p>
      </div>
    {:else if grid}
      <!-- Deux colonnes lues de haut en bas, une seule quand la place manque. -->
      <div class="grid grid-cols-2 grid-flow-col gap-x-7 gap-y-0.5 grid-rows-[repeat(var(--rows),auto)]
                  @max-[900px]:grid-cols-1 @max-[900px]:grid-flow-row @max-[900px]:grid-rows-none"
           style="--rows: {Math.ceil(sorted.length / 2)}">
        {#each sorted as track, i (track.id)}
          <AlbumTrackRow {libraryId} {track} tracks={sorted} jaquette position={i + 1} numero={i + 1}
                         sousTitre={[track.artist, track.album].filter(Boolean).join(" · ")} {onremove} {removeLabel} />
        {/each}
      </div>
    {:else}
      <LibraryTrackTable {libraryId} tracks={sorted} {columns} largeurs={widths} barreHorizontale colle={60}
                         sortKey={sortKey === "order" ? null : sortKey} {sortDir} positions={sortKey === "order"}
                         onsort={(key, dir) => { sortKey = key; sortDir = dir; }} {onremove} {removeLabel} />
    {/if}
  </section>
</DetailPage>
