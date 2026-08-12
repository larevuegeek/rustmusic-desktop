<script lang="ts">
import { onMount } from "svelte";
import { recent } from "$lib/stores/recent/recent.store";
import Icon from "@iconify/svelte";
import { formatTime } from "$lib/helper/tools/dateTools";
import { liked } from "$lib/stores/playlist/like.store";
import TrackContextMenu from "$lib/components/ui/contextmenu/TrackContextMenu.svelte";
import { handleSelectTrack, handlePlayTrack } from "$lib/actions/player/PlayerAction";
import { handleRemoveRecentItem } from "$lib/actions/recent/RecentAction";
import CoverImg from "$lib/components/ui/image/CoverImg.svelte";
import { mapRecentFile } from "$lib/mapper/recent/mapRecentFile";
import { t } from "$lib/i18n";
import type { RecentFileListView } from "$lib/types/ui/recent/RecentFileListView";

// Le menu est **le même** que partout ailleurs dans l'app, ouvert de deux
// façons : clic droit sur la ligne, ou bouton « … ». En brancher un second,
// plus pauvre, sur le bouton donnerait deux menus différents sur la même
// ligne — et priverait cette section de l'édition des tags, des likes et des
// playlists, que l'ancienne liste déroulante ne proposait pas.
let contextMenu = $state<{ x: number; y: number; track: RecentFileListView } | null>(null);

function openMenu(event: MouseEvent, track: RecentFileListView) {
  event.preventDefault();
  contextMenu = { x: event.clientX, y: event.clientY, track };
}

onMount(() => {
  recent.refreshRecent();
});
</script>

<div class="grid grid-cols-1 sm:grid-cols-2 gap-2 w-full">
{#each $recent as recentFileView, index (recentFileView.id ?? index)}
  {@const recentFile = mapRecentFile(recentFileView)}
  {@const coverPath = recentFile.library?.thumbnail_path}

  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div ondblclick={() => handlePlayTrack(recentFile.path)}
       oncontextmenu={(e) => openMenu(e, recentFileView)}
       class="group w-full hover:bg-neutral-100 dark:hover:bg-neutral-900 rounded-xl transition-colors duration-150"
       role="listitem">
    <div class="flex gap-3 px-3 py-2.5 items-center">
      <!-- Cover -->
      <button onclick={() => handleSelectTrack(recentFile.path)}
              class="shrink-0 cursor-pointer">
        <div class="w-14 h-14 sm:w-16 sm:h-16 rounded-lg overflow-hidden
                    bg-neutral-200 dark:bg-neutral-800 shadow-sm">
          {#if coverPath}
            <CoverImg path={coverPath} alt="cover" class="w-full h-full object-cover" />
          {:else}
            <img src="/images/no-cd.png" alt="cover" class="w-full h-full object-cover" />
          {/if}
        </div>
      </button>

      <!-- Infos -->
      <div class="flex-1 min-w-0">
        <button onclick={() => handleSelectTrack(recentFile.path)} class="cursor-pointer text-left w-full">
          <div class="text-sm font-semibold text-neutral-800 dark:text-neutral-200 truncate">
            {recentFile.library?.title ?? "Titre inconnu"}
          </div>
        </button>
        <div class="text-xs text-neutral-500 dark:text-neutral-400 truncate">
          {recentFile.library?.artist ?? "Artiste inconnu"}
        </div>
        <div class="hidden sm:flex items-center gap-1 text-[10px] text-neutral-400 dark:text-neutral-500 mt-1">
          <span class="truncate">{recentFile.library?.album ?? ""}</span>
          {#if recentFile.library?.audio_format}
            <span>• {recentFile.library.audio_format}</span>
          {/if}
          <span>• {formatTime(recentFile.library?.duration as number)}</span>
        </div>
      </div>

      <!-- Actions -->
      <div class="flex items-center gap-1 shrink-0 opacity-0 group-hover:opacity-100 transition-opacity">
        <button
          class="p-1.5 rounded-md cursor-pointer transition-colors
                 {$liked.paths.has(recentFile.path)
                   ? 'text-pink-500'
                   : 'text-neutral-400 hover:text-pink-400'}"
          onclick={() => liked.toggle(recentFile.path)}
          aria-label="Liker"
        >
          <Icon icon={$liked.paths.has(recentFile.path) ? "mynaui:heart-solid" : "mynaui:heart"} width="15" />
        </button>

        <button
          class="p-1.5 rounded-md cursor-pointer transition-colors
                 text-neutral-400 hover:text-neutral-700 dark:hover:text-neutral-200
                 hover:bg-neutral-200/60 dark:hover:bg-white/8"
          onclick={(e) => openMenu(e, recentFileView)}
          aria-label="Actions"
        >
          <Icon icon="lucide:more-horizontal" width="15" />
        </button>
      </div>
    </div>
  </div>
{/each}
</div>
{#if $recent.length == 0}
  <div class="relative flex flex-col items-center justify-center py-16 px-6 text-center
              rounded-2xl border border-dashed border-neutral-200/60 dark:border-neutral-800/60
              bg-neutral-50/50 dark:bg-neutral-900/30 overflow-hidden">

    <!-- Glow subtil -->
    <div class="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-64 h-64 rounded-full
                bg-emerald-500/5 blur-[80px] pointer-events-none"></div>

    <!-- Icône animée -->
    <div class="relative mb-5">
      <div class="absolute inset-0 rounded-2xl bg-emerald-500/10 blur-xl scale-150 animate-pulse"></div>
      <div class="relative w-16 h-16 rounded-2xl
                  bg-neutral-100 dark:bg-neutral-800
                  border border-neutral-200/80 dark:border-neutral-700/50
                  flex items-center justify-center
                  shadow-sm">
        <svg class="w-7 h-7 text-emerald-500/60" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="12" cy="12" r="10" />
          <polyline points="12 6 12 12 16 14" />
        </svg>
      </div>
    </div>

    <h3 class="text-base font-semibold text-neutral-700 dark:text-neutral-200 mb-1.5">
      Aucun morceau écouté
    </h3>

    <p class="text-sm text-neutral-400 dark:text-neutral-500 max-w-xs leading-relaxed mb-6">
      Lancez un titre pour qu'il apparaisse ici. Votre historique d'écoute se construit au fil de vos découvertes.
    </p>

    <!-- Faux waveform décoratif -->
    <div class="flex items-end gap-0.75 h-8 opacity-20">
      {#each [3, 5, 8, 12, 16, 20, 16, 24, 18, 14, 20, 12, 8, 14, 10, 6, 12, 8, 4, 6, 10, 14, 8, 4] as h}
        <div class="w-0.75 rounded-full bg-emerald-500 dark:bg-emerald-400"
             style="height: {h}px;"></div>
      {/each}
    </div>
  </div>
{/if}

{#if contextMenu}
  <!-- « Supprimer » retire de l'historique, pas du disque : d'où un libellé
       explicite plutôt que le « Supprimer » générique du menu. -->
  <TrackContextMenu
    track={contextMenu.track}
    x={contextMenu.x}
    y={contextMenu.y}
    showDelete={true}
    deleteLabel={$t('home.remove_from_history')}
    ondelete={() => handleRemoveRecentItem(mapRecentFile(contextMenu!.track))}
    onclose={() => contextMenu = null}
  />
{/if}