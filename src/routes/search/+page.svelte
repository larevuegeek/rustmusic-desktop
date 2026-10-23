<script lang="ts">
  // Résultats de recherche : meilleur résultat, morceaux, albums, artistes ; filtrables par type.
  import Icon from "@iconify/svelte";
  import { t, currentLocale } from "#lib/i18n";
  import { page } from "$app/state";
  import { goto } from "$app/navigation";
  import { invoke } from "@tauri-apps/api/core";
  import { sourceFilter } from "#lib/stores/source/sourceFilter.store";
  import { profilSelector } from "#lib/stores/profil/profil.store";
  import { libraryStore } from "#lib/stores/library/library.store";
  import { settingsStore } from "#lib/stores/settings/settings.store";
  import { activeServers, currentSource } from "#lib/stores/source/source.store";
  import { handlePlayTrack } from "#lib/actions/player/PlayerAction";
  import { toQueue } from "#lib/mapper/queue/mapQueueTrack";
  import { groupVersions, type SearchHit, type SearchResult } from "#lib/helper/search/searchVersions";
  import { formatCourt, trackTier } from "#lib/helper/tools/audioFormatTools";
  import { minutesSeconds } from "#lib/helper/tools/dateTools";
  import CoverImg from "#lib/components/ui/image/CoverImg.svelte";
  import QualityBadge from "#lib/components/ui/text/QualityBadge.svelte";
  import SegmentedControl from "#lib/components/ui/input/SegmentedControl.svelte";
  import PageState from "#lib/components/ui/layout/PageState.svelte";
  import VersionMarks from "#lib/components/ui/status/VersionMarks.svelte";
  import CloudMark from "#lib/components/ui/status/CloudMark.svelte";
  import TrackContextMenu from "#lib/components/ui/contextmenu/TrackContextMenu.svelte";
  import type { TrackListView } from "#lib/types/ui/library/track/TrackListView";

  const query = $derived(page.url.searchParams.get("q") ?? "");
  let results = $state<SearchResult[]>([]);
  let loading = $state(false);
  // Fiches complètes des morceaux trouvés (durée, qualité), par chemin.
  let fiches = $state<Map<string, TrackListView>>(new Map());
  let menu = $state<{ x: number; y: number; result: SearchResult } | null>(null);

  type Filtre = "all" | "tracks" | "albums" | "artists";
  let filtre = $state<Filtre>("all");

  $effect(() => {
    if (query.length >= 2) void doSearch(query);
  });

  async function doSearch(q: string) {
    loading = true;
    try {
      const r = await invoke<SearchResult[]>("search", { query: q, limit: 50, profilId: $profilSelector.profilSelected?.id ?? 0, sourceId: $sourceFilter });
      if (q !== query) return;
      results = r;
      const paths = r.filter((x) => x.result_type === "track" && x.path).map((x) => x.path as string);
      const vues = paths.length ? await invoke<TrackListView[]>("get_tracks_view_by_paths", { paths }) : [];
      fiches = new Map(vues.map((v) => [v.path, v]));
    } catch (e) {
      console.error("Search failed:", e);
    } finally {
      loading = false;
    }
  }

  // « Toutes les sources » : un même résultat local et serveur n'en fait qu'un (version préférée en tête).
  const hits = $derived(groupVersions(results, $libraryStore.libraries, $currentSource === "all" && $activeServers.length > 0,
    $settingsStore.duplicate_version === "server"));
  const tracks = $derived(hits.filter((r) => r.result_type === "track"));
  const albums = $derived(hits.filter((r) => r.result_type === "album"));
  const artists = $derived(hits.filter((r) => r.result_type === "artist"));
  // Le meilleur résultat : un album d'abord, sinon un morceau, sinon un artiste.
  const top = $derived(albums[0] ?? tracks[0] ?? artists[0] ?? null);

  const n = (x: number) => x.toLocaleString($currentLocale);

  function open(r: SearchResult) {
    if (r.result_type === "track" && r.path) handlePlayTrack(r.path, toQueue(tracks));
    else if (r.result_type === "album" && r.library_id) goto(`/library/${r.library_id}/albums/${r.id}`);
    else if (r.result_type === "artist" && r.library_id) goto(`/library/${r.library_id}/artists/${r.id}`);
  }
  // Une version choisie à la main : elle remplace la préférée dans la file.
  function openVersion(hit: SearchHit, r: SearchResult) {
    if (r.result_type === "track" && r.path) handlePlayTrack(r.path, toQueue(tracks.map((x) => (x === hit ? { ...r, versions: hit.versions } : x))));
    else open(r);
  }
  const playAll = () => tracks[0]?.path && handlePlayTrack(tracks[0].path, toQueue(tracks));

  const typeLabel = (type: string) => $t(type === "track" ? "track_view.kicker" : type === "album" ? "sidebar.album" : "sidebar.artist");
  const fiche = (r: SearchResult) => (r.path ? fiches.get(r.path) : undefined);
</script>

{#snippet marks(r: SearchHit, size = 13)}
  {#if r.versions.length > 1}
    <VersionMarks versions={r.versions} onopen={(v) => openVersion(r, v)} {size} />
  {:else}
    <CloudMark path={r.path} libraryId={r.library_id} {size} />
  {/if}
{/snippet}

{#snippet trackRow(r: SearchHit, i: number)}
  {@const f = fiche(r)}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="group grid grid-cols-[28px_44px_minmax(0,1fr)_auto_44px_32px] items-center gap-3 px-2.5 py-2 rounded-xl transition-colors hover:bg-(--rg-hover)"
       ondblclick={() => open(r)}
       oncontextmenu={(e) => { e.preventDefault(); menu = { x: e.clientX, y: e.clientY, result: r }; }}>
    <button type="button" onclick={() => open(r)} aria-label={$t("home.play")}
            class="w-7 h-7 flex items-center justify-center rounded-full cursor-pointer text-[13px] tabular-nums text-(--rg-mu)">
      <span class="group-hover:hidden">{i + 1}</span>
      <Icon icon="material-symbols:play-arrow-rounded" width="22" class="hidden group-hover:block text-(--rg-g)" />
    </button>
    {#if r.thumbnail_path}
      <CoverImg path={r.thumbnail_path} alt="" class="w-11 h-11 rounded-lg object-cover" />
    {:else}
      <span class="w-11 h-11 rounded-lg flex items-center justify-center bg-(--rg-s2) text-(--rg-mu)"><Icon icon="material-symbols:music-note-rounded" width="18" /></span>
    {/if}
    <div class="min-w-0">
      <p class="text-[14px] font-semibold truncate text-(--rg-tx)" title={r.title}>{@render marks(r)}{" "}{r.title}</p>
      <p class="text-[12.5px] truncate text-(--rg-mu)">{[r.subtitle, f?.album].filter(Boolean).join(" · ")}</p>
    </div>
    <span class="max-md:hidden">
      {#if f?.audio_format}
        <QualityBadge palier={trackTier(f.audio_format, f.bits_per_sample, f.sample_rate)} texte={formatCourt(f.audio_format, f.bits_per_sample, f.sample_rate)} class="px-1.5 py-0.5" />
      {/if}
    </span>
    <span class="text-right text-[13px] tabular-nums text-(--rg-mu)">{f?.duration ? minutesSeconds(f.duration) : ""}</span>
    <button type="button" aria-label={$t("tracks_view.more")} onclick={(e) => (menu = { x: e.clientX, y: e.clientY, result: r })}
            class="w-8 h-8 rounded-lg flex items-center justify-center cursor-pointer opacity-0 group-hover:opacity-100 text-(--rg-mu) hover:text-(--rg-tx) hover:bg-(--rg-s2)">
      <Icon icon="material-symbols:more-horiz" width="20" />
    </button>
  </div>
{/snippet}

{#snippet albumCard(r: SearchHit)}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div role="button" tabindex="0" onclick={() => open(r)} onkeydown={(e) => { if (e.key === "Enter") open(r); }}
       class="group min-w-0 p-2.5 -m-2.5 rounded-2xl cursor-pointer transition-colors hover:bg-(--rg-hover)">
    <div class="relative aspect-square rounded-xl overflow-hidden shadow-[0_8px_24px_rgba(0,0,0,0.18)] bg-(--rg-s2)">
      {#if r.thumbnail_path}
        <CoverImg path={r.thumbnail_path} alt="" class="w-full h-full object-cover transition-transform duration-300 group-hover:scale-[1.03]" />
      {:else}
        <span class="w-full h-full flex items-center justify-center text-(--rg-mu)"><Icon icon="material-symbols:album-outline" width="40" /></span>
      {/if}
    </div>
    <p class="mt-2.5 text-[14px] font-semibold truncate text-(--rg-tx)" title={r.title}>{@render marks(r)}{" "}{r.title}</p>
    {#if r.subtitle}<p class="text-[12.5px] truncate text-(--rg-mu)">{r.subtitle}</p>{/if}
  </div>
{/snippet}

{#snippet artistCard(r: SearchHit)}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div role="button" tabindex="0" onclick={() => open(r)} onkeydown={(e) => { if (e.key === "Enter") open(r); }}
       class="group min-w-0 p-2.5 -m-2.5 rounded-2xl cursor-pointer text-center transition-colors hover:bg-(--rg-hover)">
    <div class="aspect-square rounded-full overflow-hidden mx-auto bg-(--rg-s2) shadow-[0_8px_24px_rgba(0,0,0,0.16)]">
      {#if r.thumbnail_path}
        <CoverImg path={r.thumbnail_path} alt="" class="w-full h-full object-cover" />
      {:else}
        <span class="w-full h-full flex items-center justify-center text-(--rg-mu)"><Icon icon="material-symbols:artist-outline-rounded" width="40" /></span>
      {/if}
    </div>
    <p class="mt-2.5 text-[14px] font-semibold truncate text-(--rg-tx)">{r.title}</p>
    <p class="text-[12.5px] text-(--rg-mu)">{typeLabel("artist")}</p>
  </div>
{/snippet}

{#snippet section(titre: string, count: number, voir: Filtre | null)}
  <div class="flex items-baseline gap-2.5 mb-3.5">
    <h2 class="text-[19px] font-extrabold tracking-[-0.01em] text-(--rg-tx)">{titre}</h2>
    {#if count}<span class="text-sm text-(--rg-mu)">{n(count)}</span>{/if}
    {#if voir}
      <span class="flex-1"></span>
      <button type="button" onclick={() => (filtre = voir)} class="text-[13px] font-semibold cursor-pointer text-(--rg-gtx) hover:underline underline-offset-2">{$t("search.see_all")}</button>
    {/if}
  </div>
{/snippet}

<div class="biblio-maquette flex flex-col h-full">
  <!-- ─── En-tête ─── -->
  <div class="shrink-0 flex flex-wrap items-center gap-x-5 gap-y-4 px-4 md:pl-8 md:pr-14 pt-6 pb-4">
    <div class="w-16 h-16 shrink-0 rounded-2xl border flex items-center justify-center text-(--rg-g)
                bg-(--rg-gbg) border-(--rg-gbd) dark:bg-linear-145 dark:from-[#15402a] dark:to-[#0e2519] max-sm:w-12 max-sm:h-12">
      <Icon icon="material-symbols:search-rounded" width="30" />
    </div>
    <div class="flex-[1_1_240px] min-w-60">
      <p class="text-xs font-bold uppercase tracking-widest text-(--rg-mu)">{$t("search.results_for")}</p>
      <h1 class="mt-0.5 mb-1.5 text-[34px] max-sm:text-2xl font-extrabold tracking-[-0.02em] leading-tight text-(--rg-tx) truncate">
        {$t("search.query_quoted").replace("{query}", query)}
      </h1>
      <div class="flex flex-wrap items-baseline gap-x-4.5 gap-y-1.5 text-sm text-(--rg-mu)">
        <span><b class="font-bold text-(--rg-tx)">{n(tracks.length)}</b> {$t("library.tracks").toLowerCase()}</span>
        <span><b class="font-bold text-(--rg-tx)">{n(albums.length)}</b> {$t("library.albums").toLowerCase()}</span>
        <span><b class="font-bold text-(--rg-tx)">{n(artists.length)}</b> {$t("library.artists").toLowerCase()}</span>
      </div>
    </div>
    {#if tracks.length}
      <button type="button" onclick={playAll}
              class="h-10 px-4 inline-flex items-center gap-2 rounded-xl cursor-pointer text-sm font-bold bg-(--rg-g) text-(--rg-on-g) hover:brightness-110 transition">
        <Icon icon="material-symbols:play-arrow-rounded" width="20" />{$t("library.play_all")}
      </button>
    {/if}
  </div>

  <!-- ─── Filtres ─── -->
  {#if hits.length}
    <div class="shrink-0 pl-4 md:pl-8 pr-4 md:pr-14 py-3 border-t border-(--rg-line)">
      <SegmentedControl variant="discret" label={$t("search.results_for")} value={filtre} onchange={(v) => (filtre = v as Filtre)}
                        options={[
                          { value: "all", label: $t("search.tab_all") },
                          ...(tracks.length ? [{ value: "tracks", label: `${$t("library.tracks")} · ${n(tracks.length)}` }] : []),
                          ...(albums.length ? [{ value: "albums", label: `${$t("library.albums")} · ${n(albums.length)}` }] : []),
                          ...(artists.length ? [{ value: "artists", label: `${$t("library.artists")} · ${n(artists.length)}` }] : []),
                        ]} />
    </div>
  {/if}

  <!-- ─── Résultats ─── -->
  <div class="flex-1 min-h-0 overflow-y-auto scrollbar-app pl-4 md:pl-8 pr-4 md:pr-14 pt-4 pb-16">
    {#if loading && !hits.length}
      <PageState chargement message={$t("common.loading")} />
    {:else if !hits.length}
      <div class="flex flex-col items-center justify-center text-center py-20">
        <span class="w-16 h-16 rounded-2xl flex items-center justify-center mb-4 bg-(--rg-s2) text-(--rg-mu)">
          <Icon icon="material-symbols:search-off-rounded" width="30" />
        </span>
        <h3 class="text-lg font-bold text-(--rg-tx)">{$t("search.no_result")}</h3>
        <p class="mt-1 text-sm text-(--rg-mu)">{$t("search.no_result_desc")}</p>
      </div>
    {:else if filtre === "all"}
      <!-- Meilleur résultat et premiers morceaux -->
      <div class="grid gap-x-8 gap-y-8 {tracks.length && top?.result_type !== 'track' ? 'lg:grid-cols-[minmax(0,5fr)_minmax(0,7fr)]' : ''} mb-10">
        {#if top}
          <div>
            {@render section($t("search.top_result"), 0, null)}
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <div role="button" tabindex="0" onclick={() => open(top)} onkeydown={(e) => { if (e.key === "Enter") open(top); }}
                 class="group relative p-5 rounded-2xl border cursor-pointer transition-colors bg-(--rg-carte) border-(--rg-bd) hover:border-(--rg-bd2)">
              {#if top.thumbnail_path}
                <CoverImg path={top.thumbnail_path} alt="" class="w-28 h-28 object-cover shadow-[0_10px_28px_rgba(0,0,0,0.25)] {top.result_type === 'artist' ? 'rounded-full' : 'rounded-xl'}" />
              {:else}
                <span class="w-28 h-28 flex items-center justify-center bg-(--rg-s2) text-(--rg-mu) {top.result_type === 'artist' ? 'rounded-full' : 'rounded-xl'}">
                  <Icon icon="material-symbols:album-outline" width="44" />
                </span>
              {/if}
              <p class="mt-4 text-[26px] font-extrabold tracking-[-0.02em] leading-tight truncate text-(--rg-tx)" title={top.title}>{top.title}</p>
              <p class="mt-1.5 flex items-center gap-2 text-sm text-(--rg-mu)">
                <span class="px-2 py-0.5 rounded-full text-[11px] font-bold uppercase tracking-[0.06em] bg-(--rg-s2) text-(--rg-tx2)">{typeLabel(top.result_type)}</span>
                {@render marks(top, 15)}
                {#if top.subtitle}<span class="truncate">{top.subtitle}</span>{/if}
              </p>
              <span class="absolute right-5 bottom-5 w-12 h-12 rounded-full flex items-center justify-center shadow-lg
                           bg-(--rg-g) text-(--rg-on-g) opacity-0 translate-y-1 transition-all group-hover:opacity-100 group-hover:translate-y-0">
                <Icon icon={top.result_type === "track" ? "material-symbols:play-arrow-rounded" : "material-symbols:arrow-forward-rounded"} width="26" />
              </span>
            </div>
          </div>
        {/if}
        {#if tracks.length && top?.result_type !== "track"}
          <div class="min-w-0">
            {@render section($t("library.tracks"), tracks.length, tracks.length > 5 ? "tracks" : null)}
            {#each tracks.slice(0, 5) as r, i (r.id)}{@render trackRow(r, i)}{/each}
          </div>
        {/if}
      </div>
      {#if tracks.length && top?.result_type === "track"}
        <div class="mb-10">
          {@render section($t("library.tracks"), tracks.length, tracks.length > 8 ? "tracks" : null)}
          {#each tracks.slice(0, 8) as r, i (r.id)}{@render trackRow(r, i)}{/each}
        </div>
      {/if}
      {#if albums.length}
        <div class="mb-10">
          {@render section($t("library.albums"), albums.length, albums.length > 6 ? "albums" : null)}
          <div class="grid grid-cols-[repeat(auto-fill,minmax(168px,1fr))] gap-x-5 gap-y-6">
            {#each albums.slice(0, 6) as r (r.id)}{@render albumCard(r)}{/each}
          </div>
        </div>
      {/if}
      {#if artists.length}
        <div class="mb-10">
          {@render section($t("library.artists"), artists.length, artists.length > 6 ? "artists" : null)}
          <div class="grid grid-cols-[repeat(auto-fill,minmax(150px,1fr))] gap-x-5 gap-y-6">
            {#each artists.slice(0, 6) as r (r.id)}{@render artistCard(r)}{/each}
          </div>
        </div>
      {/if}
    {:else if filtre === "tracks"}
      {#each tracks as r, i (r.id)}{@render trackRow(r, i)}{/each}
    {:else if filtre === "albums"}
      <div class="grid grid-cols-[repeat(auto-fill,minmax(168px,1fr))] gap-x-5 gap-y-6">
        {#each albums as r (r.id)}{@render albumCard(r)}{/each}
      </div>
    {:else}
      <div class="grid grid-cols-[repeat(auto-fill,minmax(150px,1fr))] gap-x-5 gap-y-6">
        {#each artists as r (r.id)}{@render artistCard(r)}{/each}
      </div>
    {/if}
  </div>
</div>

{#if menu}
  <TrackContextMenu
    track={{ path: menu.result.path, title: menu.result.title, id: menu.result.id }}
    x={menu.x}
    y={menu.y}
    libraryId={menu.result.library_id}
    onclose={() => (menu = null)}
  />
{/if}
