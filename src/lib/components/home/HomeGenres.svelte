<script lang="ts">
import { goto } from "$app/navigation";
import { t, currentLocale } from "$lib/i18n";
import { teinte } from "$lib/helper/tools/teinte";
import Icon from "@iconify/svelte";
import { iconeGenre } from "$lib/helper/tools/iconeGenre";
import type { GenreView } from "$lib/types/ui/library/genre/GenreView";

let { libraryId, genres }: { libraryId: number; genres: GenreView[] } = $props();

const affiches = $derived(genres.slice(0, 6));
const nombre = (n: number) => new Intl.NumberFormat($currentLocale).format(n);
</script>

{#if affiches.length > 0}
<section class="flex flex-col gap-4">
  <div class="flex items-baseline justify-between gap-4">
    <h2 class="text-2xl font-bold tracking-tight text-neutral-900 dark:text-neutral-100">{$t('home.explore_genres')}</h2>
    <button type="button" onclick={() => goto(`/library/${libraryId}/genres`)}
            class="text-sm font-semibold text-emerald-600 dark:text-emerald-400 hover:underline cursor-pointer">
      {$t('home.all_genres')}
    </button>
  </div>

  <div class="grid grid-cols-2 @xl:grid-cols-3 @6xl:grid-cols-6 gap-3.5">
    {#each affiches as g (g.name)}
      <button type="button" onclick={() => goto(`/library/${libraryId}/genres/${encodeURIComponent(g.name)}`)}
              class="genre-tuile group relative h-33 flex items-start rounded-2xl overflow-hidden text-left cursor-pointer
                     transition-transform duration-300 hover:-translate-y-1"
              style="--h: {teinte(g.name)}">

        <!-- L'icône du genre, baignée dans la lueur du coin -->
        <Icon icon={iconeGenre(g.name)} width={88} height={88} aria-hidden="true"
              class="genre-icone absolute right-2 -bottom-2 -rotate-12
                     transition-transform duration-300 group-hover:-rotate-3 group-hover:scale-110" />

        <span class="relative flex flex-col gap-1 p-4 pr-16">
          <span class="genre-titre text-[19px] font-extrabold tracking-[-0.01em] leading-tight line-clamp-2">{g.name}</span>
          <span class="genre-compte text-[12.5px] font-semibold">
            {$t('home.genre_count').replace('{n}', nombre(g.total_tracks))}
          </span>
        </span>
      </button>
    {/each}
  </div>
</section>
{/if}
