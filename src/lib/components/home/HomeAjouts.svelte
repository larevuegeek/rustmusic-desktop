<script lang="ts">
import Icon from "@iconify/svelte";
import { goto } from "$app/navigation";
import { t } from "$lib/i18n";
import { dateToYear } from "$lib/helper/tools/dateTools";
import CoverImg from "$lib/components/ui/image/CoverImg.svelte";
import type { AlbumListView } from "$lib/types/ui/library/album/AlbumListView";

let { libraryId, albums }: { libraryId: number; albums: AlbumListView[] } = $props();

// Les plus récents d'abord ; `created_at` est une date ISO, l'ordre des chaînes suffit.
const affiches = $derived(
  [...albums].sort((a, b) => (b.created_at ?? '').localeCompare(a.created_at ?? '')).slice(0, 6)
);
</script>

{#if affiches.length > 0}
<section class="flex flex-col gap-4">
  <div class="flex items-baseline justify-between gap-4">
    <h2 class="text-2xl font-bold tracking-tight text-neutral-900 dark:text-neutral-100">{$t('home.recently_added')}</h2>
    <button type="button" onclick={() => goto(`/library/${libraryId}/albums`)}
            class="text-sm font-semibold text-emerald-600 dark:text-emerald-400 hover:underline cursor-pointer">
      {$t('home.see_all')}
    </button>
  </div>

  <div class="grid grid-cols-3 @3xl:grid-cols-4 @4xl:grid-cols-6 gap-[18px]">
    {#each affiches as album (album.id)}
      <button type="button" onclick={() => goto(`/library/${libraryId}/albums/${album.id}`)}
              class="group flex flex-col gap-2 text-left cursor-pointer min-w-0">
        <div class="aspect-square rounded-[10px] overflow-hidden bg-neutral-200 dark:bg-neutral-800
                    border border-neutral-200 dark:border-[#1f2522]">
          {#if album.cover_url}
            <CoverImg path={album.cover_url} alt={album.title} size="2x"
                      class="w-full h-full object-cover transition-transform duration-300 group-hover:scale-105" />
          {:else}
            <div class="w-full h-full flex items-center justify-center">
              <Icon icon="lucide:disc-album" width={32} class="text-neutral-400" />
            </div>
          {/if}
        </div>
        <span class="text-[15px] font-semibold truncate text-neutral-900 dark:text-neutral-100">{album.title}</span>
        <span class="text-[13px] -mt-1.5 truncate text-neutral-500 dark:text-[#9aa39e]">
          {[album.artist, album.year ? dateToYear(String(album.year)) : ''].filter(Boolean).join(' · ')}
        </span>
      </button>
    {/each}
  </div>
</section>
{/if}
