<script lang="ts">
import { page } from "$app/state";
import { goto } from "$app/navigation";
import { profilSelector } from "$lib/stores/profil/profil.store";
import type { Library } from "$lib/types/db/library/Library";
import type { TrackDetailView } from "$lib/types/ui/library/track/TrackDetailView";
import { loadTrack } from "$lib/services/library/library.service";
import { toasts } from "$lib/stores/ui/toast.store";
import Icon from "@iconify/svelte";
import { invoke } from "@tauri-apps/api/core";
import CoverImg from "$lib/components/ui/image/CoverImg.svelte";
import { handlePlayTrack } from "$lib/actions/player/PlayerAction";
import { liked } from "$lib/stores/playlist/like.store";
import TrackContextMenu from "$lib/components/ui/contextmenu/TrackContextMenu.svelte";
import StarRating from "$lib/components/ui/rating/StarRating.svelte";
import { formatBitrate } from "$lib/helper/tools/audioFormatTools";

let library: Library | null = $state(null);
let track: TrackDetailView | null = $state(null);
let isLoading = $state(true);
let error: string | null = $state(null);
let contextMenu = $state<{ x: number; y: number } | null>(null);

/** Vide tant que les liaisons ne sont pas posées : on retombe sur le tag brut. */
type TrackArtist = { artist_id: string; library_artist_id: string | null; name: string };
let artistes = $state<TrackArtist[]>([]);

const libraryId = $derived(Number(page.params.library_id));
const trackId = $derived(page.params.track_id);
const profil = $derived($profilSelector.profilSelected);
let isLiked = $derived.by(() => {
  const t = track;
  return t !== null ? $liked.paths.has(t.path) : false;
});

let currentTag = 0;

$effect(() => {
  const id = libraryId;
  const tId = trackId;
  const p = profil;
  error = null;

  if (!$profilSelector.initialized) return;
  if (!p) { error = "Aucun profil sélectionné"; isLoading = false; return; }
  if (!id || isNaN(id)) { error = "ID invalide"; isLoading = false; return; }
  if (!tId) { error = "ID de piste invalide"; isLoading = false; return; }

  const tag = ++currentTag;
  loadData(id, tId, p.id, tag);
});

async function loadData(libId: number, trackId: string, profilId: number, tag: number) {
  isLoading = true;
  error = null;
  try {
    const lib = await invoke<Library>('get_library', { libraryId: libId });
    if (tag !== currentTag) return;
    if (!lib || lib.profil_id !== profilId) {
      toasts.push({ type: "error", title: "Accès refusé", message: "Bibliothèque inaccessible." });
      goto("/"); return;
    }
    library = lib;
    track = await loadTrack(trackId);

    // Un échec ici ne doit pas priver la page du reste.
    try {
      const credits = await invoke<TrackArtist[]>('get_track_artists', { trackId });
      if (tag === currentTag) artistes = credits;
    } catch (e) {
      console.error('Failed to load track artists:', e);
      if (tag === currentTag) artistes = [];
    }
    if (tag !== currentTag) return;
  } catch (e) {
    if (tag !== currentTag) return;
    error = "Impossible de charger la piste";
  } finally {
    if (tag !== currentTag) return;
    isLoading = false;
  }
}

function goBack() { goto(`/library/${libraryId}/tracks`); }

function formatDuration(seconds: number | null) {
  if (!seconds) return "—";
  const m = Math.floor(seconds / 60);
  const s = Math.floor(seconds % 60).toString().padStart(2, "0");
  return `${m}:${s}`;
}

function formatSampleRate(sr: number | null) {
  if (!sr) return "—";
  return sr >= 1000 ? `${(sr / 1000).toFixed(1)} kHz` : `${sr} Hz`;
}

function formatSize(bytes: number | null) {
  if (!bytes) return "—";
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${(bytes / Math.pow(k, i)).toFixed(1)} ${sizes[i]}`;
}

function handleContextMenu(e: MouseEvent) {
  e.preventDefault();
  contextMenu = { x: e.clientX, y: e.clientY };
}
</script>

{#if isLoading}
  <div class="flex items-center justify-center h-full text-neutral-400">
    <Icon icon="lucide:loader-2" width="20" class="animate-spin mr-2" />
    Chargement…
  </div>
{:else if error}
  <div class="flex items-center justify-center h-full text-red-500">{error}</div>
{:else if library && track}

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="flex flex-col scrollbar-app overflow-y-auto px-8 py-6 h-full"
     oncontextmenu={handleContextMenu}>

  <!-- RETOUR -->
  <button
    type="button"
    onclick={goBack}
    class="flex items-center gap-2 text-sm text-neutral-400 hover:text-neutral-200 cursor-pointer
           transition-colors mb-6 w-fit"
  >
    <Icon icon="lucide:arrow-left" width={14} />
    Retour aux morceaux
  </button>

  <!-- HERO -->
  <div class="flex flex-col sm:flex-row gap-5 sm:gap-8 mb-8">

    <!-- COVER -->
    <div class="w-40 h-40 sm:w-56 sm:h-56 rounded-2xl overflow-hidden shadow-2xl shadow-black/30 shrink-0 mx-auto sm:mx-0
                bg-neutral-200 dark:bg-neutral-800 flex items-center justify-center">
      {#if track.thumbnail_path}
        <CoverImg path={track.thumbnail_path} alt={track.title}
             class="w-full h-full object-cover" />
      {:else}
        <Icon icon="lucide:music" width={56} class="text-neutral-400" />
      {/if}
    </div>

    <!-- INFOS -->
    <div class="flex-1 flex flex-col justify-between min-w-0">
      <div>
        <p class="text-[10px] uppercase tracking-widest text-neutral-500 mb-1">Morceau</p>
        <h1 class="text-3xl font-bold text-neutral-900 dark:text-neutral-100 truncate">
          {track.title}
        </h1>

        <div class="flex items-center gap-2 mt-2 text-sm text-neutral-500 dark:text-neutral-400">
          <!-- Un lien par artiste crédité. -->
          {#if artistes.length > 0}
            <span class="flex items-center gap-1 flex-wrap">
              {#each artistes as credit, i (credit.artist_id)}
                {#if i > 0}<span class="opacity-40">,</span>{/if}
                {#if credit.library_artist_id}
                  <a href={`/library/${libraryId}/artists/${credit.library_artist_id}`}
                     class="hover:text-green-500 transition-colors">{credit.name}</a>
                {:else}
                  <!-- Absent de cette bibliothèque : le lien mènerait au vide. -->
                  <span>{credit.name}</span>
                {/if}
              {/each}
            </span>
          {:else if track.artist}
            <a href={`/library/${libraryId}/artists/${track.library_artist_id}`}
               class="hover:text-green-500 transition-colors">
              {track.artist}
            </a>
          {/if}
          {#if track.album}
            <span class="opacity-40">•</span>
            <a href={`/library/${libraryId}/albums/${track.album_id}`}
               class="hover:text-green-500 transition-colors">
              {track.album}
            </a>
          {/if}
          {#if track.year}
            <span class="opacity-40">•</span>
            <span>{track.year}</span>
          {/if}
        </div>

        {#if track.genre}
          <span class="inline-block mt-2 px-2.5 py-0.5 rounded-full text-[10px] font-medium uppercase tracking-wider
                       bg-green-500/10 text-green-500 border border-green-500/20">
            {track.genre}
          </span>
        {/if}
      </div>

      <!-- ACTIONS -->
      <div class="flex items-center gap-3 mt-4">
        <button
          type="button"
          onclick={() => track && handlePlayTrack(track.path)}
          class="flex items-center gap-2 px-5 py-2 rounded-full text-sm font-semibold cursor-pointer
                 bg-green-500 text-black hover:bg-green-400
                 active:scale-[0.97] transition-all duration-150"
        >
          <Icon icon="lucide:play" width={15} />
          Lecture
        </button>

        <button
          onclick={() => track && liked.toggle(track.path)}
          class="favori p-2.5 rounded-full cursor-pointer transition-all duration-150
                 {isLiked
                   ? 'text-pink-500 bg-pink-500/10 hover:bg-pink-500/20'
                   : 'text-neutral-400 hover:text-pink-400 hover:bg-pink-500/10'}
                 border border-transparent {isLiked ? 'border-pink-500/20' : ''}"
          aria-label={isLiked ? 'Retirer des favoris' : 'Ajouter aux favoris'}
        >
          <Icon icon={isLiked ? "mynaui:heart-solid" : "lucide:heart"} width={18} />
        </button>

        <!-- Rating -->
        {#if track.id}
          <div class="flex items-center gap-2 px-3 py-2.5 rounded-full
                      bg-white/5 border border-white/8">
            <StarRating
              trackId={String(track.id)}
              value={track.rating}
              size={15}
              onchange={(r) => { if (track) track.rating = r; }}
            />
          </div>
        {/if}

        <button
          onclick={(e) => { contextMenu = { x: e.clientX, y: e.clientY }; }}
          class="p-2.5 rounded-full cursor-pointer text-neutral-400
                 hover:bg-white/10 transition-colors"
          aria-label="Actions"
        >
          <Icon icon="lucide:more-horizontal" width={18} />
        </button>
      </div>
    </div>
  </div>

  <!-- SÉPARATEUR -->
  <div class="h-px bg-linear-to-r from-transparent via-neutral-200/80 dark:via-neutral-700/30 to-transparent mb-6"></div>

  <!-- TAGS DÉTAILLÉS -->
  <div class="mb-8">
    <h2 class="text-[10px] font-semibold uppercase tracking-widest text-neutral-400 mb-4">Informations audio</h2>
    <div class="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 gap-3 sm:gap-4">
      {#each [
        { label: 'Durée', value: formatDuration(track.duration) },
        { label: 'Format', value: track.audio_format ?? track.extension?.toUpperCase() ?? '—' },
        { label: 'Bitrate', value: formatBitrate(track.bitrate) },
        { label: 'Sample Rate', value: formatSampleRate(track.sample_rate) },
        { label: 'Bits/Sample', value: track.bits_per_sample ? `${track.bits_per_sample} bits` : '—' },
        { label: 'Canaux', value: track.channels === 2 ? 'Stéréo' : track.channels === 1 ? 'Mono' : track.channels ? `${track.channels} ch` : '—' },
        { label: 'Taille', value: formatSize(track.file_size) },
        { label: 'Lectures', value: `${track.play_count ?? 0}` },
      ] as meta (meta.label)}
        <div class="px-4 py-3 rounded-xl bg-white/40 dark:bg-white/3
                    border border-neutral-200/40 dark:border-white/5">
          <p class="text-[10px] text-neutral-400 dark:text-neutral-500 uppercase tracking-wider mb-0.5">{meta.label}</p>
          <p class="text-sm font-medium text-neutral-700 dark:text-neutral-200">{meta.value}</p>
        </div>
      {/each}
    </div>
  </div>

  <!-- TAGS METADATA -->
  {#if track.album_artist || track.genre || track.year || track.track_number || track.disc_number}
    <div class="mb-8">
      <h2 class="text-[10px] font-semibold uppercase tracking-widest text-neutral-400 mb-4">Tags</h2>
      <div class="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 gap-3 sm:gap-4">
        {#if track.album_artist}
          <div class="px-4 py-3 rounded-xl bg-white/40 dark:bg-white/3 border border-neutral-200/40 dark:border-white/5">
            <p class="text-[10px] text-neutral-400 uppercase tracking-wider mb-0.5">Album Artist</p>
            <p class="text-sm font-medium text-neutral-700 dark:text-neutral-200">{track.album_artist}</p>
          </div>
        {/if}
        {#if track.track_number}
          <div class="px-4 py-3 rounded-xl bg-white/40 dark:bg-white/3 border border-neutral-200/40 dark:border-white/5">
            <p class="text-[10px] text-neutral-400 uppercase tracking-wider mb-0.5">Piste</p>
            <p class="text-sm font-medium text-neutral-700 dark:text-neutral-200">{track.track_number}</p>
          </div>
        {/if}
        {#if track.disc_number}
          <div class="px-4 py-3 rounded-xl bg-white/40 dark:bg-white/3 border border-neutral-200/40 dark:border-white/5">
            <p class="text-[10px] text-neutral-400 uppercase tracking-wider mb-0.5">Disque</p>
            <p class="text-sm font-medium text-neutral-700 dark:text-neutral-200">{track.disc_number}</p>
          </div>
        {/if}
      </div>
    </div>
  {/if}

  <!-- SÉPARATEUR -->
  <div class="h-px bg-linear-to-r from-transparent via-neutral-200/80 dark:via-neutral-700/30 to-transparent mb-6"></div>

  <!-- LIENS ALBUM / ARTISTE -->
  <div class="grid grid-cols-1 md:grid-cols-2 gap-4 mb-8">

    {#if track.album && track.album_id}
      <a href={`/library/${libraryId}/albums/${track.album_id}`}
         class="group flex items-center gap-4 px-5 py-4 rounded-xl
                bg-white/40 dark:bg-white/3
                border border-neutral-200/40 dark:border-white/5
                hover:bg-green-500/5 hover:border-green-500/20
                transition-all duration-200">
        <div class="w-12 h-12 rounded-lg overflow-hidden bg-neutral-200 dark:bg-neutral-800 shrink-0 flex items-center justify-center">
          {#if track.thumbnail_path}
            <CoverImg path={track.thumbnail_path} alt={track.album} size="1x"
                 class="w-full h-full object-cover" />
          {:else}
            <Icon icon="lucide:disc-album" width={20} class="text-neutral-400" />
          {/if}
        </div>
        <div class="flex-1 min-w-0">
          <p class="text-[10px] text-neutral-400 uppercase tracking-wider">Album</p>
          <p class="text-sm font-medium text-neutral-700 dark:text-neutral-200 truncate
                    group-hover:text-green-500 transition-colors">{track.album}</p>
        </div>
        <Icon icon="lucide:chevron-right" width="16"
              class="text-neutral-300 dark:text-neutral-600 group-hover:text-green-500 transition-colors shrink-0" />
      </a>
    {/if}

    {#if track.artist && track.library_artist_id}
      <a href={`/library/${libraryId}/artists/${track.library_artist_id}`}
         class="group flex items-center gap-4 px-5 py-4 rounded-xl
                bg-white/40 dark:bg-white/3
                border border-neutral-200/40 dark:border-white/5
                hover:bg-green-500/5 hover:border-green-500/20
                transition-all duration-200">
        <div class="w-12 h-12 rounded-full bg-neutral-200 dark:bg-neutral-800 shrink-0 flex items-center justify-center">
          <Icon icon="lucide:mic-2" width={20} class="text-neutral-400" />
        </div>
        <div class="flex-1 min-w-0">
          <p class="text-[10px] text-neutral-400 uppercase tracking-wider">Artiste</p>
          <p class="text-sm font-medium text-neutral-700 dark:text-neutral-200 truncate
                    group-hover:text-green-500 transition-colors">{track.artist}</p>
        </div>
        <Icon icon="lucide:chevron-right" width="16"
              class="text-neutral-300 dark:text-neutral-600 group-hover:text-green-500 transition-colors shrink-0" />
      </a>
    {/if}
  </div>

  <!-- FICHIER -->
  <div class="mb-6">
    <h2 class="text-[10px] font-semibold uppercase tracking-widest text-neutral-400 mb-3">Fichier</h2>
    <p class="text-xs text-neutral-500 dark:text-neutral-400 font-mono break-all
              px-4 py-3 rounded-xl bg-white/40 dark:bg-white/3
              border border-neutral-200/40 dark:border-white/5">
      {track.path}
    </p>
  </div>
</div>

{#if contextMenu && track}
  <TrackContextMenu
    track={track}
    x={contextMenu.x}
    y={contextMenu.y}
    libraryId={libraryId}
    showNavigation={true}
    onclose={() => contextMenu = null}
  />
{/if}
{/if}
