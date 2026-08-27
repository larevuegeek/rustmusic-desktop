<script lang="ts">
import { handleRemoveTrackItem } from "$lib/actions/library/LibraryAction";
import { handleSelectTrack, handlePlayTrack } from "$lib/actions/player/PlayerAction";
import TrackContextMenu from "$lib/components/ui/contextmenu/TrackContextMenu.svelte";
import { liked } from "$lib/stores/playlist/like.store";
import { selectionStore } from "$lib/stores/ui/selection.store";
import { settingsStore } from "$lib/stores/settings/settings.store";
import Icon from "@iconify/svelte";
import CoverImg from "$lib/components/ui/image/CoverImg.svelte";
import { dsdLabel, isDsdFormat } from "$lib/helper/tools/audioFormatTools";

let { libraryId, track, showAlbum = false }: { libraryId: any; track: any; showAlbum?: boolean } = $props();

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

function handleDblClick() {
    if (!selection.active) {
        handlePlayTrack(track.path);
    }
}
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<div
    class="row-album-track group flex items-center justify-between py-2 px-2 rounded-md transition-colors duration-150
           {isSelected
             ? 'bg-emerald-500/10 dark:bg-emerald-500/10'
             : 'hover:bg-neutral-100 dark:hover:bg-neutral-900'}
           {selection.active ? 'cursor-pointer' : 'cursor-default'}"
    ondblclick={handleDblClick}
    oncontextmenu={handleContextMenu}
    onclick={handleClick}
>

    <div class="flex items-center gap-3 min-w-0">

        <!-- Checkbox / Track number -->
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

        <!-- Cover mini -->
        {#if track.thumbnail_path}
            <button onclick={handleClick} class="shrink-0 cursor-pointer">
                <CoverImg path={track.thumbnail_path} alt="" size="1x"
                     class="w-12 h-12 rounded-md object-cover shadow-sm" />
            </button>
        {/if}

        <div class="flex flex-col items-stretch min-w-0">
            <button onclick={handleClick} class="text-left cursor-pointer min-w-0 w-full">
                <span class="block font-medium text-sm truncate transition-colors
                      {isSelected
                        ? 'text-emerald-600 dark:text-emerald-400'
                        : 'text-neutral-800 dark:text-neutral-200 group-hover:text-emerald-600 dark:group-hover:text-emerald-400'}"
                      title={track.title}>
                    {track.title}
                </span>
            </button>

            {#if showAlbum && track.album}
              <span class="text-[11px] text-neutral-500 dark:text-neutral-400 truncate mt-0.5">
                {track.album}{#if track.year} <span class="text-neutral-400 dark:text-neutral-500">• {track.year}</span>{/if}
              </span>
            {/if}

            <span class="flex items-center gap-1.5 text-[10px] text-neutral-400 dark:text-neutral-500 truncate mt-0.5">
                {#if isDsdFormat(track.audio_format)}
                    <span class="text-amber-500/90 dark:text-amber-300/90 font-semibold tracking-wide">
                        {dsdLabel(track.sample_rate)}
                    </span>
                {:else}
                    <span>{track.audio_format ?? track.extension?.toUpperCase() ?? "—"}</span>
                    {#if track.bits_per_sample}<span>• {track.bits_per_sample}bit</span>{/if}
                    {#if track.sample_rate}<span>• {Math.round(track.sample_rate / 1000)}kHz</span>{/if}
                {/if}
            </span>
        </div>
    </div>

    {#if !selection.active}
      <div class="flex items-center gap-4 text-xs text-neutral-500 dark:text-neutral-400 shrink-0">

          <button
              onclick={() => liked.toggle(track.path)}
              class="p-1 rounded-md cursor-pointer transition-colors
                     {isLiked
                       ? 'text-pink-500 hover:text-pink-400'
                       : 'text-neutral-300 dark:text-neutral-600 hover:text-pink-400 dark:hover:text-pink-400'}"
              aria-label={isLiked ? 'Retirer des favoris' : 'Ajouter aux favoris'}
          >
              <Icon icon={isLiked ? "mynaui:heart-solid" : "lucide:heart"} width={14} />
          </button>

          {#if track.duration}
              <span class="tabular-nums">
                  {Math.floor(track.duration / 60)}:{String(Math.floor(track.duration % 60)).padStart(2, '0')}
              </span>
          {/if}

          <div class="opacity-0 group-hover:opacity-100 transition-opacity">
              <button
                  onclick={(e) => { contextMenu = { x: e.clientX, y: e.clientY }; }}
                  class="p-2 rounded-md cursor-pointer text-neutral-500 dark:text-neutral-400
                         hover:bg-black/5 dark:hover:bg-white/10"
                  aria-label="Actions"
              >
                  <Icon icon="uit:ellipsis-v" width={20} height={20} />
              </button>
          </div>
      </div>
    {/if}
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
