<script lang="ts">
import Icon from "@iconify/svelte";
import { t, currentLocale } from "$lib/i18n";
import type { LibraryStats } from "$lib/types/ui/library/stats/LibraryStats";

let { nom, libraryId, stats, onajouter }: {
  nom: string | null | undefined;
  libraryId: number | null;
  stats: LibraryStats | null;
  onajouter: () => void;
} = $props();

function salutation(): string {
  const h = new Date().getHours();
  if (h < 6) return $t('home.greeting_night');
  if (h < 12) return $t('home.greeting_morning');
  if (h < 18) return $t('home.greeting_afternoon');
  return $t('home.greeting_evening');
}

const nombre = (n: number) => new Intl.NumberFormat($currentLocale).format(n);

// Chaque chiffre mène à la vue qui le détaille.
const chiffres = $derived(stats && libraryId ? [
  { n: stats.total_tracks, libelle: $t('home.stat_tracks'), vers: `/library/${libraryId}/tracks` },
  { n: stats.total_albums, libelle: $t('home.stat_albums'), vers: `/library/${libraryId}/albums` },
  { n: stats.total_artists, libelle: $t('home.stat_artists'), vers: `/library/${libraryId}/artists` },
] : []);
</script>

<!-- Deux lignes seulement : bouton face au titre, chiffres face au sous-titre. -->
<header class="grid grid-cols-1 @3xl:grid-cols-[minmax(0,1fr)_auto] items-baseline gap-x-8 gap-y-1.5">
  <h1 class="@3xl:col-start-1 @3xl:row-start-1 min-w-0 text-3xl @3xl:text-[34px] font-extrabold tracking-[-0.02em] leading-tight
             text-neutral-900 dark:text-neutral-50">
    {salutation()}{nom ? `, ${nom}` : ''}
  </h1>

  <p class="@3xl:col-start-1 @3xl:row-start-2 min-w-0 text-[17px] text-neutral-500 dark:text-[#9aa39e]">
    {$t('home.subtitle')}
  </p>

  {#if chiffres.length > 0}
    <div class="@3xl:col-start-2 @3xl:row-start-2 @3xl:justify-self-end mt-2 @3xl:mt-0 flex items-baseline gap-3.5">
      {#each chiffres as c, i (c.vers)}
        {#if i > 0}<span class="self-center w-px h-3.5 bg-neutral-300 dark:bg-white/15" aria-hidden="true"></span>{/if}
        <a href={c.vers} class="group flex items-baseline gap-1.5 whitespace-nowrap">
          <span class="text-[17px] font-extrabold tabular-nums tracking-[-0.01em] text-neutral-900 dark:text-neutral-50
                       group-hover:text-emerald-600 dark:group-hover:text-emerald-400 transition-colors">
            {nombre(c.n)}
          </span>
          <span class="text-[11px] font-bold uppercase tracking-widest text-neutral-500 dark:text-[#9aa39e]">
            {c.libelle}
          </span>
        </a>
      {/each}
    </div>
  {/if}

  <!-- Même famille que les boutons secondaires du hero ; seul le « + » porte l'accent. -->
  <button type="button" onclick={onajouter}
          class="@3xl:col-start-2 @3xl:row-start-1 self-center justify-self-start @3xl:justify-self-end mt-3 @3xl:mt-0
                 h-10 px-4 rounded-full text-sm font-semibold flex items-center gap-2 cursor-pointer whitespace-nowrap
                 bg-neutral-900/6 hover:bg-neutral-900/10 text-neutral-900
                 dark:bg-white/10 dark:hover:bg-white/16 dark:text-white transition-colors">
    <Icon icon="lucide:plus" width={16} class="text-[#16a34a] dark:text-[#22c55e]" /> {$t('home.add_music')}
  </button>
</header>
