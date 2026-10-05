<script lang="ts">
// Les mix de l'accueil : « Pour vous », vos mix, puis un par décennie. La page Mix les montre tous.
import Icon from "@iconify/svelte";
import { t } from "#lib/i18n";
import { playlistStore } from "#lib/stores/playlist/playlist.store";
import { renouveler } from "#lib/stores/mix/mix.store";
import { mixPourVous, mixDecennies, mixCrees } from "#lib/config/mixes";
import { actionsMix } from "#lib/actions/mix/MixAction";
import Carousel from "#lib/components/ui/carousel/Carousel.svelte";
import MixCard from "#lib/components/mix/MixCard.svelte";
import type { GenreMix } from "#lib/types/ui/library/genre/GenreMix";
import type { AlbumListView } from "#lib/types/ui/library/album/AlbumListView";

let { libraryId, genres, albums = [], hires }: { libraryId: number; genres: GenreMix[]; albums?: AlbumListView[]; hires: number } = $props();

const specs = $derived(mixPourVous(genres, hires, $t));
const perso = $derived(mixCrees($playlistStore.playlists, $t));
const decennies = $derived(mixDecennies(albums, $t));

const lien = "shrink-0 text-sm font-semibold text-emerald-600 dark:text-emerald-400 hover:underline cursor-pointer";
</script>

{#if specs.length > 0}
<section class="flex flex-col gap-4">
  <div class="flex items-baseline justify-between gap-4">
    <h2 class="text-2xl font-bold tracking-tight text-neutral-900 dark:text-neutral-100">{$t('home.for_you')}</h2>
    <div class="flex items-center gap-4">
      <span class="hidden @3xl:inline text-sm text-neutral-500 dark:text-[#9aa39e]">{$t('home.for_you_desc')}</span>
      <button type="button" onclick={() => renouveler(libraryId)}
              class="flex items-center gap-1.5 text-sm font-semibold text-emerald-600 dark:text-emerald-400
                     hover:text-emerald-500 dark:hover:text-emerald-300 cursor-pointer transition-colors">
        <Icon icon="lucide:refresh-cw" width={14} /> {$t('home.new_mixes')}
      </button>
      <a href="/library/{libraryId}/mixes" class={lien}>{$t('mix_view.all')}</a>
    </div>
  </div>

  <!-- Étroit : deux cartes horizontales par ligne ; large : quatre carrés. -->
  <div class="grid grid-cols-1 @xl:grid-cols-2 @4xl:grid-cols-4 gap-4 @4xl:gap-5">
    {#each specs as s (s.cle)}
      <MixCard {libraryId} cle={s.cle} source={s.source} type={s.type} nom={s.nom} aide={s.aide} h={s.h}
               variante="horizontale" actions={actionsMix(s, libraryId)} />
    {/each}
  </div>
</section>
{/if}

{#if perso.length > 0}
<section class="flex flex-col gap-4">
  <div class="flex items-baseline justify-between gap-4">
    <h2 class="text-2xl font-bold tracking-tight text-neutral-900 dark:text-neutral-100">{$t('mix_view.yours')}</h2>
    <span class="hidden @3xl:inline flex-1 text-sm text-neutral-500 dark:text-[#9aa39e]">{$t('mix_view.yours_desc')}</span>
    <a href="/library/{libraryId}/mixes" class={lien}>{$t('mix_view.all')}</a>
  </div>
  <Carousel axe="90px" class="gap-4 @4xl:gap-5 pt-1.5 pb-2">
    {#each perso as s (s.cle)}
      <div class="w-44 @4xl:w-48 shrink-0 snap-start">
        <MixCard {libraryId} cle={s.cle} source={s.source} type={s.type} nom={s.nom} aide={s.aide} h={s.h} actions={actionsMix(s, libraryId)} />
      </div>
    {/each}
  </Carousel>
</section>
{/if}

{#if decennies.length > 0}
<section class="flex flex-col gap-4">
  <div class="flex items-baseline justify-between gap-4">
    <h2 class="text-2xl font-bold tracking-tight text-neutral-900 dark:text-neutral-100">{$t('home.decades')}</h2>
    <span class="hidden @3xl:inline flex-1 text-sm text-neutral-500 dark:text-[#9aa39e]">{$t('home.decades_desc')}</span>
    <a href="/library/{libraryId}/years" class={lien}>{$t('home.all_years')}</a>
  </div>
  <!-- Carrousel à flèches, calées au milieu des pochettes. -->
  <Carousel axe="90px" class="gap-4 @4xl:gap-5 pt-1.5 pb-2">
    {#each decennies as s (s.cle)}
      <div class="w-40 @4xl:w-44 shrink-0 snap-start">
        <MixCard {libraryId} cle={s.cle} source={s.source} type={s.type} nom={s.nom} aide={s.aide} h={s.h}
                 variante="decennie" pre={s.pre} court={s.court} actions={actionsMix(s, libraryId)} />
      </div>
    {/each}
  </Carousel>
</section>
{/if}
