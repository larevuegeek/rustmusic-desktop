<script lang="ts">
import type { ArtistListView } from "$lib/types/ui/library/artist/ArtistListView";
import Icon from "@iconify/svelte";
import CoverImg from "$lib/components/ui/image/CoverImg.svelte";
import { preload, preloadArtistData } from "$lib/actions/preload/preloadAction";
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

<a href={`/library/${libraryId}/artists/${artist.id}`}
    class="flex flex-col items-center group text-center transition-all cursor-pointer duration-200 relative"
    use:preload={() => preloadArtistData(artist.id)}
        onclick={handleCardClick}
>

    <!-- Case de sélection, sur la vignette : une grille n'a pas de marge où
         loger une colonne de cases. -->
    {#if selection.active}
        <div class="absolute top-1 left-1 z-10 w-5 h-5 rounded flex items-center justify-center
                    transition-all duration-150
                    {isSelected
                      ? 'bg-emerald-500 text-white'
                      : 'bg-black/40 backdrop-blur-sm border border-white/40 text-transparent'}">
            <svg class="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor"
                 stroke-width="3" stroke-linecap="round">
                <path d="m4.5 12.75 6 6 9-13.5"/>
            </svg>
        </div>
    {/if}

    <div class="w-40 h-40 rounded-full overflow-hidden relative
                bg-neutral-200 dark:bg-neutral-700
                shadow-md
                group-hover:scale-105
                group-hover:shadow-xl
                transition-all duration-300
                flex items-center justify-center">

        <Icon icon="lucide:user" width={48} class="text-neutral-400" />

        <div class="absolute inset-0 transition-opacity duration-500 ease-in-out
                    {imagePath ? 'opacity-100' : 'opacity-0 pointer-events-none'}">
            
            {#if imagePath}
                <CoverImg
                    path={imagePath}
                    alt={artist.name}
                    size="2x"
                    class="w-full h-full object-cover"
                />
            {/if}
        </div>

    </div>

    <span class="mt-4 font-medium text-neutral-800 dark:text-neutral-200 truncate max-w-37.5">
        {artist.name}
    </span>

    <span class="text-xs text-neutral-500 dark:text-neutral-400">
        {artist.total_albums} albums • {artist.total_tracks} titres
    </span>
</a>