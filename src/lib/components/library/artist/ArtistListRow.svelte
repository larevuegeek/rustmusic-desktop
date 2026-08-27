<script lang="ts">
import type { ArtistListView } from "$lib/types/ui/library/artist/ArtistListView";
import Icon from "@iconify/svelte";
import CoverImg from "$lib/components/ui/image/CoverImg.svelte";
import { artistImageReadyStore } from "$lib/stores/library/artistImageReady.store";

let { libraryId, artist }: { libraryId: number; artist: ArtistListView } = $props();

let imagePath = $state<string | null>(null);

$effect(() => {
    const live = artistImageReadyStore.get(artist.id, $artistImageReadyStore);
    imagePath = live ?? artist.thumbnail_path ?? null;
});

import { selectionStore } from "$lib/stores/ui/selection.store";
import { cleGroupe, toggleGroupSelection } from "$lib/helper/tools/selectionGroups";

const selection = $derived($selectionStore);
const cleSel = $derived(cleGroupe('artist', libraryId, artist.id));
const isSelected = $derived(selection.groupes.has(cleSel));

/**
 * En mode sélection, cocher au lieu de naviguer.
 *
 * Le lien reste un lien — survol, clic milieu, préchargement — mais la
 * navigation est retenue tant que la sélection est active.
 */
function handleCardClick(e: MouseEvent) {
    if (!selection.active) return;
    e.preventDefault();
    e.stopPropagation();
    toggleGroupSelection('artist', libraryId, artist.id);
}
</script>

<a
    class="group flex items-center gap-4 px-3 py-2.5 rounded-xl cursor-pointer
           hover:bg-neutral-50 dark:hover:bg-white/4 transition-colors duration-100"
    href={`/library/${libraryId}/artists/${artist.id}`}
    onclick={handleCardClick}
>

    <!-- Case de sélection : en tête de ligne, là où la colonne existe déjà. -->
    {#if selection.active}
        <div class="w-5 h-5 rounded shrink-0 flex items-center justify-center
                    transition-all duration-150
                    {isSelected
                      ? 'bg-emerald-500 text-white'
                      : 'bg-white dark:bg-white/5 border border-neutral-300 dark:border-white/15 text-transparent'}">
            <svg class="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor"
                 stroke-width="3" stroke-linecap="round">
                <path d="m4.5 12.75 6 6 9-13.5"/>
            </svg>
        </div>
    {/if}
    <!-- Avatar -->
    <div class="w-10 h-10 rounded-full shrink-0 overflow-hidden relative
                bg-neutral-100 dark:bg-neutral-800
                group-hover:ring-2 group-hover:ring-green-500/30 transition-all
                flex items-center justify-center">

        <!-- Icône placeholder -->
        <Icon icon="lucide:mic-2" width={16}
              class="text-neutral-400 group-hover:text-green-500 transition-colors" />

        <!-- Image par dessus en fadeIn -->
        {#if imagePath}
        <div class="absolute inset-0">
            <CoverImg path={imagePath} alt={artist.name} size="1x"
                 class="w-full h-full object-cover" />
        </div>
        {/if}
    </div>

    <!-- Nom -->
    <div class="flex-1 min-w-0">
        <p class="text-sm font-medium text-neutral-800 dark:text-neutral-200 truncate">
            {artist.name}
        </p>
        <p class="text-xs text-neutral-400 truncate">
            {artist.total_tracks} titre{artist.total_tracks !== 1 ? 's' : ''}
            • {artist.total_albums} album{artist.total_albums !== 1 ? 's' : ''}
        </p>
    </div>

    <Icon icon="lucide:chevron-right" width="14"
          class="text-neutral-300 dark:text-neutral-600
                 group-hover:text-neutral-500 transition-colors shrink-0" />
</a>
