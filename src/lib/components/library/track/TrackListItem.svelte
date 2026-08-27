<script lang="ts">
import { handleRemoveTrackItem } from "$lib/actions/library/LibraryAction";
import { handleSelectTrack, handlePlayTrack } from "$lib/actions/player/PlayerAction";
import TrackContextMenu from "$lib/components/ui/contextmenu/TrackContextMenu.svelte";
import { liked } from "$lib/stores/playlist/like.store";
import { selectionStore } from "$lib/stores/ui/selection.store";
import { settingsStore } from "$lib/stores/settings/settings.store";
import Icon from "@iconify/svelte";
import CoverImg from "$lib/components/ui/image/CoverImg.svelte";
import BadgeQualityAudio from "../common/badge/BadgeQualityAudio.svelte";
import StarRating from "$lib/components/ui/rating/StarRating.svelte";
import { formatBitrate, isDsdFormat } from "$lib/helper/tools/audioFormatTools";

let { libraryId, track }: { libraryId: number; track: any } = $props();

let contextMenu = $state<{ x: number; y: number } | null>(null);
let isLiked = $derived($liked.paths.has(track.path));
let selection = $derived($selectionStore);
let isSelected = $derived(selection.active && selection.ids.has(track.id));
let singleClickPlay = $derived(settingsStore.get('single_click_play') === 'true');

function handleContextMenu(e: MouseEvent) {
    e.preventDefault();
    contextMenu = { x: e.clientX, y: e.clientY };
}

// Appelée depuis toute la ligne, pas seulement le titre.
//
// Le gabarit gardait cet appel derrière `if (selection.active)` : hors mode
// sélection, cliquer une ligne ne déclenchait donc rien, et le réglage
// « simple clic = lecture » restait inatteignable. C'est ici que le choix se
// fait — cocher, lire, ou précharger.
function handleClick(e?: MouseEvent) {
    if (selection.active) {
        // Maj étend depuis le dernier élément cliqué, comme dans un
        // explorateur de fichiers. Sans elle, cocher trente morceaux demande
        // trente clics.
        if (e?.shiftKey) selectionStore.selectRange(track.id);
        else selectionStore.toggle(track.id, track);
    } else if (singleClickPlay) {
        handlePlayTrack(track.path);
    } else {
        handleSelectTrack(track.path);
    }
}
</script>          
<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<div
  class="row-track flex items-center justify-between py-3 px-3 rounded-md
          transition-colors duration-150
          {isSelected ? 'bg-emerald-500/10 dark:bg-emerald-500/10' : 'hover:bg-neutral-100 dark:hover:bg-neutral-900'}
          {selection.active ? 'cursor-pointer' : ''}"
  ondblclick={() => { if (!selection.active) handlePlayTrack(track.path); }}
  onclick={handleClick}
  oncontextmenu={handleContextMenu}
>
<!-- LEFT -->
<div class="flex items-center gap-4 min-w-0">

    <!-- CHECKBOX / TRACK NUMBER -->
    {#if selection.active}
      <button
        type="button"
        class="w-6 h-6 rounded shrink-0 flex items-center justify-center cursor-pointer
               transition-all duration-150
               {isSelected
                 ? 'bg-emerald-500 text-white'
                 : 'bg-white dark:bg-white/5 border border-neutral-300 dark:border-white/15 text-transparent hover:border-emerald-500 dark:hover:border-emerald-500/40'}"
        onclick={(e) => { e.stopPropagation(); selectionStore.toggle(track.id, track); }}
      >
        {#if isSelected}
          <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round">
            <path d="m4.5 12.75 6 6 9-13.5"/>
          </svg>
        {/if}
      </button>
    {:else}
      <div class="w-6 text-xs text-neutral-400 text-right shrink-0">
          {track.track_number ? String(track.track_number).padStart(2, "0") : "—"}
      </div>
    {/if}

    <!-- THUMBNAIL -->
     <button onclick={handleClick} class="cursor-pointer">
        <div class="w-22 h-22 rounded-md overflow-hidden 
                    bg-neutral-200 dark:bg-neutral-700 
                    flex items-center justify-center shrink-0">
        {#if track.thumbnail_path}
            <CoverImg
            path={track.thumbnail_path}
            alt="Cover"
            size="1x"
            class="w-full h-full object-cover"
            />
        {:else}
            <Icon icon="lucide:music" width={18} class="text-neutral-400" />
        {/if}
        </div>
    </button>

    <!-- TEXT -->
    <div class="flex flex-col items-stretch min-w-0">

        <!-- TITLE -->
        <button onclick={() => handleSelectTrack(track.path)} class="text-left cursor-pointer min-w-0 w-full">
            <span class="block font-medium text-neutral-800 dark:text-neutral-200 truncate"
                  title={track.title}>
            {track.title}
            </span>
        </button>

        <!-- ARTIST / ALBUM -->
        <div class="text-xs text-neutral-500 dark:text-neutral-400 truncate">
            <a href={`/library/${libraryId}/artists/${track.artist_id}`}>
            {track.artist ?? "Unknown Artist"} 
            </a>
        </div>
        <div class="text-xs text-neutral-500 dark:text-neutral-400 truncate my-1">
            <a href={`/library/${libraryId}/albums/${track.album_id}`}>
                <span class="font-semibold">{track.album ?? "Unknown Album"}</span>
            </a>
            {#if track.year}
            • {track.year}
            {/if}
        </div>

        <!-- FORMAT LINE -->
        <span class="text-[11px] text-neutral-400 dark:text-neutral-500 truncate tracking-wide">
            
            <BadgeQualityAudio track={track} size="sm" />

            {#if !isDsdFormat(track.audio_format)}
                {track.audio_format ?? track.extension?.toUpperCase() ?? "—"}

                {#if track.bits_per_sample}
                • {track.bits_per_sample}bit
                {/if}

                {#if track.sample_rate}
                • {Math.round(track.sample_rate / 1000)}kHz
                {/if}
            {/if}
        </span>
    </div>
</div>

<!-- RIGHT -->
<div class="flex items-center gap-6 text-sm text-neutral-500 dark:text-neutral-400 shrink-0">

    <div class="hidden md:flex">
        <StarRating trackId={String(track.id)} value={track.rating} size={13} onchange={(r) => track.rating = r} />
    </div>

    {#if track.bitrate}
    <span class="text-xs">
        {formatBitrate(track.bitrate)}
    </span>
    {/if}

    {#if track.duration}
    <span class="tabular-nums">
        {Math.floor(track.duration / 60)}:
        {String(Math.floor(track.duration % 60)).padStart(2, '0')}
    </span>
    {/if}

    <button
        onclick={() => liked.toggle(track.path)}
        class="p-1.5 rounded-md cursor-pointer transition-colors
               {isLiked
                 ? 'text-pink-500 hover:text-pink-400'
                 : 'text-neutral-300 dark:text-neutral-600 hover:text-pink-400 dark:hover:text-pink-400'}"
        aria-label={isLiked ? 'Retirer des favoris' : 'Ajouter aux favoris'}
    >
        <Icon icon={isLiked ? "mynaui:heart-solid" : "lucide:heart"} width={15} />
    </button>

    <button
        onclick={(e) => { contextMenu = { x: e.clientX, y: e.clientY }; }}
        class="p-2 rounded-md cursor-pointer text-neutral-500 dark:text-neutral-400
               hover:bg-black/5 dark:hover:bg-white/10"
        aria-label="Actions"
    >
        <Icon icon="uit:ellipsis-v" width={24} height={24} />
    </button>
</div>
</div>

{#if contextMenu}
  <TrackContextMenu
    {track}
    x={contextMenu.x}
    y={contextMenu.y}
    {libraryId}
    showNavigation={true}
    showDelete={true}
    onclose={() => contextMenu = null}
    ondelete={() => handleRemoveTrackItem(track)}
  />
{/if}