<script lang="ts">
import type { RecentFileListView } from "$lib/types/ui/recent/RecentFileListView";
import { formatTime } from "$lib/helper/tools/dateTools";
import { recent } from "$lib/stores/recent/recent.store";
import Icon from "@iconify/svelte";
import { invoke } from "@tauri-apps/api/core";
import ViewModeToggle from "$lib/components/ui/input/ViewModeToggle.svelte";
import TrackTable from "$lib/components/library/track/TrackTable.svelte";
import { viewMode } from "$lib/stores/ui/viewMode.store";
import { libraryStore } from "$lib/stores/library/library.store";
import { trierPistes, resetTagCache, type SortDir } from "$lib/config/trackColumns";
import type { TrackListView } from "$lib/types/ui/library/track/TrackListView";
import PageHeader from "$lib/components/ui/header/PageHeader.svelte";
import TrackContextMenu from "$lib/components/ui/contextmenu/TrackContextMenu.svelte";
import { handleSelectTrack, handlePlayTrack } from "$lib/actions/player/PlayerAction";
import CoverImg from "$lib/components/ui/image/CoverImg.svelte";
import { toasts } from "$lib/stores/ui/toast.store";
import { onMount } from "svelte";
import { t } from "$lib/i18n";

let tracks: RecentFileListView[] = $state([]);

// ─── Vue tableau ───
//
// Les mêmes colonnes et le même tri que l'onglet Morceaux. La vue tableau se
// compose à partir de `TrackListView`, que l'historique ne porte pas : il
// désigne ses morceaux par chemin. On va donc les chercher, et seulement quand
// le mode tableau est demandé.
let tableTracks = $state<TrackListView[]>([]);
let tableSortKey = $state<string | null>(null);
let tableSortDir = $state<SortDir>('asc');
let tableLoading = $state(false);

const tableLibraryId = $derived(
  // La bibliothèque par défaut d'abord : c'est celle dont les tags ont le
  // plus de chances de correspondre à ce qu'on regarde. Retomber sur la
  // première venue dépendrait d'un ordre que rien ne garantit.
  $libraryStore.libraries.find(l => l.is_default)?.id
    ?? $libraryStore.libraries[0]?.id
    ?? null
);
const tableSorted = $derived(trierPistes(tableTracks, tableSortKey, tableSortDir));

function handleTableSort(key: string, dir: SortDir) {
  tableSortKey = key;
  tableSortDir = dir;
}

$effect(() => {
  const chemins = tracks.map(t => t.path).filter((p): p is string => !!p);
  if ($viewMode !== 'list' || chemins.length === 0) return;
  chargerTableau(chemins);
});

async function chargerTableau(chemins: string[]) {
  tableLoading = true;
  try {
    resetTagCache();
    tableTracks = await invoke<TrackListView[]>('get_tracks_view_by_paths', { paths: chemins });
  } catch (e) {
    console.error('Vue tableau :', e);
    tableTracks = [];
  } finally {
    tableLoading = false;
  }
}
let loading = $state(false);
let contextMenu = $state<{ x: number; y: number; track: RecentFileListView } | null>(null);

const unsubscribe = recent.subscribe((files) => {
  tracks = files;
});

async function loadRecent() {
  loading = true;
  await recent.refreshRecent();
  loading = false;
}

function handleClearRecent() {
  recent.clearRecent();
  toasts.push({
    type: "success",
    title: "Historique vidé",
    message: "Les titres récents ont été supprimés"
  });
}

onMount(() => {
  loadRecent();
});
</script>

<!-- La fenêtre ne défile jamais : `html, body` sont en `overflow: hidden`, et
     le conteneur de contenu de la mise en page l'est aussi. Chaque page doit
     donc fournir sa propre zone de défilement, sinon ce qui dépasse est
     simplement rogné — ici la fin de la liste, cachée derrière le lecteur. -->
<div class="h-full overflow-y-auto scrollbar-app py-5 px-4 md:px-10">

  <PageHeader
    title="Récemment joués"
    subtitle="Historique"
    icon="mynaui:clock-8"
    iconColor="#0ea5e9"
    count={tracks.length}
    countLabel="titre"
  >
    {#snippet actions()}
      <div class="flex items-center gap-2">
        <!-- La bascule grille/liste vit dans la barre d'onglets de la bibliothèque,
             qui ne couvre pas ces pages. Sans elle ici, le mode tableau existait
             sans qu'on puisse le demander. -->
        <ViewModeToggle />
      </div>
      {#if tracks.length > 0}
        <button
          class="text-xs text-red-400/60 hover:text-red-400 cursor-pointer transition-colors flex items-center gap-1"
          onclick={handleClearRecent}
        >
          <Icon icon="lucide:trash-2" width="12" />
          {$t('playlist_page.clear_history')}
        </button>
      {/if}
    {/snippet}
  </PageHeader>

{#if tracks.length === 0 && !loading}
  <div class="flex flex-col items-center justify-center py-24 px-6 text-center">
    <div class="relative mb-6">
      <div class="absolute inset-0 rounded-full bg-cyan-500 blur-2xl scale-[2.5] animate-pulse opacity-20 pointer-events-none"></div>
      <div class="relative w-20 h-20 rounded-full bg-cyan-500/10 border border-cyan-500/20 flex items-center justify-center">
        <Icon icon="mynaui:clock-8" width="36" class="text-cyan-500 drop-shadow-[0_0_8px_rgba(6,182,212,0.5)]" />
      </div>
    </div>

    <h2 class="text-lg font-semibold text-neutral-800 dark:text-neutral-100 mb-2">
      {$t('playlist_page.empty_recent')}
    </h2>

    <p class="text-sm text-neutral-500 dark:text-neutral-400 max-w-xs leading-relaxed">
      {$t('playlist_page.empty_recent_desc')}
    </p>
  </div>
{:else}

  {#if $viewMode === "list"}
    {#if tableLoading}
      <div class="flex items-center justify-center py-16">
        <Icon icon="lucide:loader-2" width="20" class="animate-spin text-neutral-400" />
      </div>
    {:else}
      <TrackTable
        libraryId={tableLibraryId}
        tracks={tableSorted}
        sortKey={tableSortKey}
        sortDir={tableSortDir}
        onsort={handleTableSort}
      />
    {/if}
  {:else}
  {#each tracks as track, index (track.path)}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="flex items-center justify-between py-3 px-3 rounded-md
                hover:bg-neutral-100 dark:hover:bg-neutral-900
                transition-colors duration-150"
         ondblclick={() => handlePlayTrack(track.path)}
         oncontextmenu={(e) => { e.preventDefault(); contextMenu = { x: e.clientX, y: e.clientY, track }; }}>
      <!-- LEFT -->
      <div class="flex items-center gap-4 min-w-0">
        <div class="w-6 text-xs text-neutral-400 text-right shrink-0">
          {String(index + 1).padStart(2, "0")}
        </div>

        <button onclick={() => handleSelectTrack(track.path)} class="cursor-pointer">
          <div class="w-22 h-22 rounded-md overflow-hidden
                      bg-neutral-200 dark:bg-neutral-700
                      flex items-center justify-center shrink-0">
            {#if track.thumbnail_path}
              <CoverImg path={track.thumbnail_path} alt="Cover"
                   class="w-full h-full object-cover" />
            {:else}
              <Icon icon="lucide:music" width={18} class="text-neutral-400" />
            {/if}
          </div>
        </button>

        <div class="flex flex-col items-stretch min-w-0">
          <button onclick={() => handleSelectTrack(track.path)} class="text-left cursor-pointer min-w-0 w-full">
            <span class="block font-medium text-neutral-800 dark:text-neutral-200 truncate"
                  title={track.title ?? "Titre inconnu"}>
              {track.title ?? "Titre inconnu"}
            </span>
          </button>

          <div class="text-xs text-neutral-500 dark:text-neutral-400 truncate">
            {track.artist ?? "Artiste inconnu"}
          </div>

          <div class="text-xs text-neutral-500 dark:text-neutral-400 truncate my-1">
            <span class="font-semibold">{track.album ?? "Album inconnu"}</span>
          </div>

          <span class="text-[11px] text-neutral-400 dark:text-neutral-500 truncate tracking-wide">
            {#if track.audio_format}
              {track.audio_format}
            {/if}
            {#if track.bits_per_sample}
              · {track.bits_per_sample}bit
            {/if}
            {#if track.sample_rate}
              · {Math.round(track.sample_rate / 1000)}kHz
            {/if}
          </span>
        </div>
      </div>

      <!-- RIGHT -->
      <div class="flex items-center gap-6 text-sm text-neutral-500 dark:text-neutral-400 shrink-0">
        <span class="tabular-nums">
          {formatTime(track.duration ?? 0)}
        </span>

        <button
          onclick={(e) => { contextMenu = { x: e.clientX, y: e.clientY, track }; }}
          class="p-2 rounded-md cursor-pointer text-neutral-500 dark:text-neutral-400
                 hover:bg-black/5 dark:hover:bg-white/10"
          aria-label="Actions"
        >
          <Icon icon="uit:ellipsis-v" width={24} height={24} />
        </button>
      </div>
    </div>
  {/each}
  {/if}
{/if}
</div>

{#if contextMenu}
  <TrackContextMenu
    track={contextMenu.track}
    x={contextMenu.x}
    y={contextMenu.y}
    onclose={() => contextMenu = null}
  />
{/if}