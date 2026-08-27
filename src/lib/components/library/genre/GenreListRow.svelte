<script lang="ts">
  import { goto } from "$app/navigation";
  import Icon from "@iconify/svelte";
  import CoverImg from "$lib/components/ui/image/CoverImg.svelte";
  import type { GenreView } from "$lib/types/ui/library/genre/GenreView";
  import { selectionStore } from "$lib/stores/ui/selection.store";
  import { cleGroupe, toggleGroupSelection } from "$lib/helper/tools/selectionGroups";

  /**
   * Un genre en ligne, pour la vue tableau.
   *
   * La grille montre une mosaïque de pochettes, belle mais avare : on n'y lit
   * que le nom. Cette ligne rend ce qu'on vient y chercher quand on cherche
   * vraiment — le nombre d'albums et de titres, alignés et comparables d'un
   * genre à l'autre.
   */
  let {
    libraryId,
    genre,
    color,
  }: {
    libraryId: number;
    genre: GenreView;
    /** Teinte déterministe calculée par la page, pour rester cohérente avec la grille. */
    color: string;
  } = $props();

  const selection = $derived($selectionStore);
  const cleSel = $derived(cleGroupe("genre", libraryId, genre.name));
  const isSelected = $derived(selection.groupes.has(cleSel));

  function handleClick() {
    if (selection.active) {
      toggleGroupSelection("genre", libraryId, genre.name);
      return;
    }
    goto(`/library/${libraryId}/genres/${encodeURIComponent(genre.name)}`);
  }
</script>

<button
  type="button"
  class="w-full group flex items-center gap-4 px-3 py-2.5 rounded-xl cursor-pointer text-left
         hover:bg-neutral-50 dark:hover:bg-white/4 transition-colors duration-100"
  onclick={handleClick}
>
  {#if selection.active}
    <div
      class="w-5 h-5 rounded shrink-0 flex items-center justify-center transition-all duration-150
             {isSelected
               ? 'bg-emerald-500 text-white'
               : 'bg-white dark:bg-white/5 border border-neutral-300 dark:border-white/15 text-transparent'}"
    >
      <svg class="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor"
           stroke-width="3" stroke-linecap="round">
        <path d="m4.5 12.75 6 6 9-13.5"/>
      </svg>
    </div>
  {/if}

  <!-- Vignette : la première pochette du genre, ou sa teinte à défaut. -->
  <div
    class="w-12 h-12 rounded-lg overflow-hidden shrink-0 flex items-center justify-center"
    style="background: {color}22;"
  >
    {#if genre.covers.length >= 1}
      <CoverImg path={genre.covers[0]} alt={genre.name} size="1x"
                class="w-full h-full object-cover" />
    {:else}
      <Icon icon="lucide:tag" width="18" style="color: {color};" />
    {/if}
  </div>

  <div class="min-w-0 flex-1">
    <div class="text-sm font-medium truncate text-neutral-800 dark:text-neutral-200">
      {genre.name}
    </div>
  </div>

  <!-- Les chiffres à chasse fixe et à largeur fixe : c'est ce qui les rend
       comparables d'une ligne à l'autre sans avoir à les lire un par un. -->
  <div class="hidden sm:block w-24 text-right text-xs tabular-nums
              text-neutral-500 dark:text-neutral-400 shrink-0">
    {genre.total_albums} album{genre.total_albums !== 1 ? 's' : ''}
  </div>
  <div class="w-24 text-right text-xs tabular-nums
              text-neutral-500 dark:text-neutral-400 shrink-0">
    {genre.total_tracks} titre{genre.total_tracks !== 1 ? 's' : ''}
  </div>
</button>
