<script lang="ts">
import Icon from "@iconify/svelte";
import { t, currentLocale } from "$lib/i18n";
import { invoke } from "@tauri-apps/api/core";
import CoverImg from "$lib/components/ui/image/CoverImg.svelte";
import { libraryStore } from "$lib/stores/library/library.store";
import { profilSelector } from "$lib/stores/profil/profil.store";
import type { LibraryStats } from "$lib/types/ui/library/stats/LibraryStats";
import { formatBitrate } from "$lib/helper/tools/audioFormatTools";
import { tailleLisible } from "$lib/helper/tools/sizeTools";
import { dureeEcoute } from "$lib/helper/tools/dateTools";

let stats: LibraryStats | null = $state(null);
let isLoading = $state(true);

const profil = $derived($profilSelector.profilSelected);
const profilColor = $derived(profil?.color ?? '#22c55e');
const library = $derived($libraryStore.librarySelected);

$effect(() => {
  if (library?.id) loadStats(library.id as number);
});

async function loadStats(libraryId: number) {
  isLoading = true;
  try {
    stats = await invoke<LibraryStats>('get_library_stats', { libraryId });
  } catch (e) {
    console.error('Failed to load stats:', e);
  } finally {
    isLoading = false;
  }
}

// Couleurs pour les graphiques
const formatColors = ['#22c55e', '#3b82f6', '#a855f7', '#f97316', '#ef4444', '#eab308', '#06b6d4', '#ec4899'];
const qualityColors = { hires: '#22c55e', lossless: '#3b82f6', lossy: '#6b7280' };
let qualityTotal = $derived.by(() => {
  if (!stats) return 0;
  return stats.quality_hires + stats.quality_lossless + stats.quality_lossy;
});
</script>

<div class="py-6 px-4 md:px-10 scrollbar-app overflow-y-auto" style="height: calc(100vh - 250px);">

  <!-- Header -->
  <div class="flex items-center gap-3 mb-8">
    <button
      class="w-8 h-8 flex items-center justify-center rounded-lg cursor-pointer
             text-neutral-400 hover:text-neutral-700 dark:hover:text-neutral-200
             hover:bg-neutral-100 dark:hover:bg-neutral-800 transition-all"
      onclick={() => history.back()}
    >
      <Icon icon="lucide:arrow-left" width="16" />
    </button>
    <div>
      <h1 class="text-2xl font-bold tracking-tight text-neutral-900 dark:text-neutral-100">
        {$t('stats.title')}
      </h1>
      <div class="mt-1 h-0.5 w-10 rounded-full" style="background: {profilColor};"></div>
    </div>
  </div>

  {#if isLoading}
    <div class="flex items-center justify-center py-20">
      <Icon icon="lucide:loader-2" width="24" class="animate-spin text-neutral-400" />
    </div>
  {:else if stats}
    <div class="space-y-8">

      <!-- ═══ OVERVIEW ═══ -->
      <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
        <div class="p-5 rounded-2xl bg-neutral-100/60 dark:bg-white/4 border border-neutral-200/50 dark:border-white/6">
          <div class="flex items-center gap-2 text-neutral-400 mb-3">
            <Icon icon="lucide:music" width={14} />
            <span class="text-[10px] uppercase tracking-widest font-medium">{$t('stats.tracks')}</span>
          </div>
          <p class="text-3xl font-bold text-neutral-900 dark:text-white tabular-nums">{stats.total_tracks.toLocaleString($currentLocale)}</p>
        </div>

        <div class="p-5 rounded-2xl bg-neutral-100/60 dark:bg-white/4 border border-neutral-200/50 dark:border-white/6">
          <div class="flex items-center gap-2 text-neutral-400 mb-3">
            <Icon icon="lucide:disc-album" width={14} />
            <span class="text-[10px] uppercase tracking-widest font-medium">{$t('stats.albums')}</span>
          </div>
          <p class="text-3xl font-bold text-neutral-900 dark:text-white tabular-nums">{stats.total_albums.toLocaleString($currentLocale)}</p>
        </div>

        <div class="p-5 rounded-2xl bg-neutral-100/60 dark:bg-white/4 border border-neutral-200/50 dark:border-white/6">
          <div class="flex items-center gap-2 text-neutral-400 mb-3">
            <Icon icon="lucide:mic-2" width={14} />
            <span class="text-[10px] uppercase tracking-widest font-medium">{$t('stats.artists')}</span>
          </div>
          <p class="text-3xl font-bold text-neutral-900 dark:text-white tabular-nums">{stats.total_artists.toLocaleString($currentLocale)}</p>
        </div>

        <div class="p-5 rounded-2xl bg-neutral-100/60 dark:bg-white/4 border border-neutral-200/50 dark:border-white/6">
          <div class="flex items-center gap-2 text-neutral-400 mb-3">
            <Icon icon="lucide:clock" width={14} />
            <span class="text-[10px] uppercase tracking-widest font-medium">{$t('stats.total_duration')}</span>
          </div>
          <p class="text-3xl font-bold text-neutral-900 dark:text-white">{dureeEcoute(stats.total_duration_sec)}</p>
        </div>
      </div>

      <!-- ═══ SECOND ROW ═══ -->
      <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
        <div class="p-4 rounded-2xl bg-neutral-100/60 dark:bg-white/4 border border-neutral-200/50 dark:border-white/6">
          <span class="text-[10px] uppercase tracking-widest text-neutral-400 font-medium">{$t('stats.genres')}</span>
          <p class="text-2xl font-bold text-neutral-900 dark:text-white mt-1">{stats.total_genres}</p>
        </div>
        <div class="p-4 rounded-2xl bg-neutral-100/60 dark:bg-white/4 border border-neutral-200/50 dark:border-white/6">
          <span class="text-[10px] uppercase tracking-widest text-neutral-400 font-medium">{$t('stats.size')}</span>
          <p class="text-2xl font-bold text-neutral-900 dark:text-white mt-1">{tailleLisible(stats.total_size_bytes, $currentLocale)}</p>
        </div>
        <div class="p-4 rounded-2xl bg-neutral-100/60 dark:bg-white/4 border border-neutral-200/50 dark:border-white/6">
          <span class="text-[10px] uppercase tracking-widest text-neutral-400 font-medium">{$t('stats.avg_bitrate')}</span>
          <p class="text-2xl font-bold text-neutral-900 dark:text-white mt-1">{formatBitrate(stats.avg_bitrate)}</p>
        </div>
        <div class="p-4 rounded-2xl bg-neutral-100/60 dark:bg-white/4 border border-neutral-200/50 dark:border-white/6">
          <span class="text-[10px] uppercase tracking-widest text-neutral-400 font-medium">{$t('stats.plays')}</span>
          <p class="text-2xl font-bold text-neutral-900 dark:text-white mt-1">{stats.total_play_count.toLocaleString($currentLocale)}</p>
        </div>
      </div>

      <!-- ═══ CHARTS ROW ═══ -->
      <div class="grid grid-cols-1 md:grid-cols-2 gap-6">

        <!-- Répartition formats (donut) -->
        <div class="p-6 rounded-2xl bg-neutral-100/60 dark:bg-white/4 border border-neutral-200/50 dark:border-white/6">
          <h3 class="text-xs font-semibold uppercase tracking-widest text-neutral-400 mb-5">{$t('stats.audio_formats')}</h3>

          <div class="flex items-center gap-8">
            <!-- Donut SVG -->
            <svg viewBox="0 0 100 100" class="w-32 h-32 shrink-0 -rotate-90">
              {#each stats.formats as fmt, i}
                {@const total = stats.formats.reduce((s, f) => s + f.count, 0)}
                {@const offset = stats.formats.slice(0, i).reduce((s, f) => s + f.count, 0)}
                {@const pct = fmt.count / total}
                <circle
                  cx="50" cy="50" r="40"
                  fill="none"
                  stroke={formatColors[i % formatColors.length]}
                  stroke-width="12"
                  stroke-dasharray="{pct * 251.3} {251.3}"
                  stroke-dashoffset="{-(offset / total) * 251.3}"
                  class="transition-all duration-500"
                />
              {/each}
            </svg>

            <!-- Légende -->
            <div class="flex flex-col gap-2 min-w-0 flex-1">
              {#each stats.formats.slice(0, 6) as fmt, i}
                {@const total = stats.formats.reduce((s, f) => s + f.count, 0)}
                <div class="flex items-center gap-2">
                  <div class="w-2.5 h-2.5 rounded-full shrink-0" style="background: {formatColors[i % formatColors.length]};"></div>
                  <span class="text-xs text-neutral-700 dark:text-neutral-300 truncate">{fmt.name}</span>
                  <span class="text-[10px] text-neutral-400 ml-auto tabular-nums">{Math.round(fmt.count / total * 100)}%</span>
                </div>
              {/each}
            </div>
          </div>
        </div>

        <!-- Qualité audio (donut) -->
        <div class="p-6 rounded-2xl bg-neutral-100/60 dark:bg-white/4 border border-neutral-200/50 dark:border-white/6">
          <h3 class="text-xs font-semibold uppercase tracking-widest text-neutral-400 mb-5">{$t('stats.audio_quality')}</h3>

          <div class="flex items-center gap-8">
            <svg viewBox="0 0 100 100" class="w-32 h-32 shrink-0 -rotate-90">
              {#if qualityTotal > 0}
                <circle cx="50" cy="50" r="40" fill="none" stroke={qualityColors.hires} stroke-width="12"
                        stroke-dasharray="{(stats.quality_hires / qualityTotal) * 251.3} {251.3}"
                        stroke-dashoffset="0" />
                <circle cx="50" cy="50" r="40" fill="none" stroke={qualityColors.lossless} stroke-width="12"
                        stroke-dasharray="{(stats.quality_lossless / qualityTotal) * 251.3} {251.3}"
                        stroke-dashoffset="{-(stats.quality_hires / qualityTotal) * 251.3}" />
                <circle cx="50" cy="50" r="40" fill="none" stroke={qualityColors.lossy} stroke-width="12"
                        stroke-dasharray="{(stats.quality_lossy / qualityTotal) * 251.3} {251.3}"
                        stroke-dashoffset="{-((stats.quality_hires + stats.quality_lossless) / qualityTotal) * 251.3}" />
              {/if}
            </svg>

            <div class="flex flex-col gap-3">
              <div class="flex items-center gap-2">
                <div class="w-2.5 h-2.5 rounded-full" style="background: {qualityColors.hires};"></div>
                <span class="text-xs text-neutral-700 dark:text-neutral-300">{$t('stats.quality_hires')}</span>
                <span class="text-[10px] text-neutral-400 ml-2 tabular-nums">{stats.quality_hires}</span>
              </div>
              <div class="flex items-center gap-2">
                <div class="w-2.5 h-2.5 rounded-full" style="background: {qualityColors.lossless};"></div>
                <span class="text-xs text-neutral-700 dark:text-neutral-300">{$t('stats.quality_lossless')}</span>
                <span class="text-[10px] text-neutral-400 ml-2 tabular-nums">{stats.quality_lossless}</span>
              </div>
              <div class="flex items-center gap-2">
                <div class="w-2.5 h-2.5 rounded-full" style="background: {qualityColors.lossy};"></div>
                <span class="text-xs text-neutral-700 dark:text-neutral-300">{$t('stats.quality_lossy')}</span>
                <span class="text-[10px] text-neutral-400 ml-2 tabular-nums">{stats.quality_lossy}</span>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- ═══ BAR CHARTS ═══ -->
      <div class="grid grid-cols-1 md:grid-cols-2 gap-6">

        <!-- Top genres -->
        <div class="p-6 rounded-2xl bg-neutral-100/60 dark:bg-white/4 border border-neutral-200/50 dark:border-white/6">
          <h3 class="text-xs font-semibold uppercase tracking-widest text-neutral-400 mb-5">{$t('stats.top_genres')}</h3>
          <div class="flex flex-col gap-2.5">
            {#each stats.top_genres as genre, i}
              {@const max = stats.top_genres[0]?.count ?? 1}
              <div class="flex items-center gap-3">
                <span class="text-[10px] text-neutral-400 w-4 text-right tabular-nums">{i + 1}</span>
                <div class="flex-1 min-w-0">
                  <div class="flex items-center justify-between mb-1">
                    <span class="text-xs font-medium text-neutral-700 dark:text-neutral-200 truncate">{genre.name}</span>
                    <span class="text-[10px] text-neutral-400 tabular-nums shrink-0 ml-2">{genre.count}</span>
                  </div>
                  <div class="h-1.5 rounded-full bg-neutral-200/60 dark:bg-white/8 overflow-hidden">
                    <div class="h-full rounded-full transition-all duration-500"
                         style="width: {(genre.count / max) * 100}%; background: {profilColor};"></div>
                  </div>
                </div>
              </div>
            {/each}
          </div>
        </div>

        <!-- Top artistes -->
        <div class="p-6 rounded-2xl bg-neutral-100/60 dark:bg-white/4 border border-neutral-200/50 dark:border-white/6">
          <h3 class="text-xs font-semibold uppercase tracking-widest text-neutral-400 mb-5">{$t('stats.top_artists')}</h3>
          <div class="flex flex-col gap-2.5">
            {#each stats.top_artists as artist, i}
              {@const max = stats.top_artists[0]?.count ?? 1}
              <div class="flex items-center gap-3">
                <span class="text-[10px] text-neutral-400 w-4 text-right tabular-nums">{i + 1}</span>
                <div class="flex-1 min-w-0">
                  <div class="flex items-center justify-between mb-1">
                    <span class="text-xs font-medium text-neutral-700 dark:text-neutral-200 truncate">{artist.name}</span>
                    <span class="text-[10px] text-neutral-400 tabular-nums shrink-0 ml-2">{artist.count === 1 ? $t('home.track_one') : $t('home.tracks_n').replace('{n}', String(artist.count))}</span>
                  </div>
                  <div class="h-1.5 rounded-full bg-neutral-200/60 dark:bg-white/8 overflow-hidden">
                    <div class="h-full rounded-full transition-all duration-500"
                         style="width: {(artist.count / max) * 100}%; background: {profilColor};"></div>
                  </div>
                </div>
              </div>
            {/each}
          </div>
        </div>
      </div>

      <!-- ═══ TOP ÉCOUTES ═══ -->
      {#if stats.top_played.length > 0}
        <div class="p-6 rounded-2xl bg-neutral-100/60 dark:bg-white/4 border border-neutral-200/50 dark:border-white/6">
          <h3 class="text-xs font-semibold uppercase tracking-widest text-neutral-400 mb-5">{$t('stats.most_played')}</h3>
          <div class="flex flex-col gap-1">
            {#each stats.top_played as track, i}
              {@const max = stats.top_played[0]?.play_count ?? 1}
              <div class="flex items-center gap-3 p-2 rounded-lg hover:bg-neutral-200/40 dark:hover:bg-white/4 transition-colors">
                <span class="text-[11px] text-neutral-400 w-5 text-right tabular-nums font-medium">{i + 1}</span>

                {#if track.thumbnail_path}
                  <CoverImg
                    path={track.thumbnail_path}
                    alt=""
                    class="w-9 h-9 rounded object-cover shrink-0"
                  />
                {:else}
                  <div class="w-9 h-9 rounded bg-neutral-200 dark:bg-white/8 flex items-center justify-center shrink-0">
                    <Icon icon="lucide:music" width={14} class="text-neutral-400" />
                  </div>
                {/if}

                <div class="flex-1 min-w-0">
                  <div class="flex items-center justify-between">
                    <div class="min-w-0 mr-3">
                      <p class="text-sm font-medium text-neutral-800 dark:text-neutral-100 truncate" title={track.title}>{track.title}</p>
                      <p class="text-[11px] text-neutral-400 truncate">{track.artist}</p>
                    </div>
                    <span class="text-xs text-neutral-400 tabular-nums shrink-0">{$t(track.play_count > 1 ? 'stats.plays_n' : 'stats.plays_one').replace('{n}', String(track.play_count))}</span>
                  </div>
                  <div class="h-1 rounded-full bg-neutral-200/60 dark:bg-white/8 overflow-hidden mt-1.5">
                    <div class="h-full rounded-full transition-all duration-500"
                         style="width: {(track.play_count / max) * 100}%; background: {profilColor};"></div>
                  </div>
                </div>
              </div>
            {/each}
          </div>
        </div>
      {/if}

    </div>
  {/if}
</div>
