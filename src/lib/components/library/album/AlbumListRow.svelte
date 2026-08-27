<script lang="ts">
import type { AlbumListView } from "$lib/types/ui/library/album/AlbumListView";
import CollectionContextMenu from "$lib/components/ui/contextmenu/CollectionContextMenu.svelte";
import { goto } from "$app/navigation";
import Icon from "@iconify/svelte";
import { invoke } from "@tauri-apps/api/core";
import CoverImg from "$lib/components/ui/image/CoverImg.svelte";

let { libraryId, album }: { libraryId: number; album: AlbumListView } = $props();

let contextMenu = $state<{ x: number; y: number } | null>(null);

async function loadAlbumTracks() {
    return await invoke('get_tracks_by_album', { libraryId, libraryAlbumId: album.id }) as any[];
}

import { selectionStore } from "$lib/stores/ui/selection.store";
import { cleGroupe, toggleGroupSelection } from "$lib/helper/tools/selectionGroups";

const selection = $derived($selectionStore);
const cleSel = $derived(cleGroupe('album', libraryId, album.id));
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
    toggleGroupSelection('album', libraryId, album.id);
}
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<a
    class="group flex items-center gap-4 px-3 py-2.5 rounded-xl cursor-pointer
           hover:bg-neutral-50 dark:hover:bg-white/4 transition-colors duration-100"
    href={`/library/${libraryId}/albums/${album.id}`}
    oncontextmenu={(e) => { e.preventDefault(); contextMenu = { x: e.clientX, y: e.clientY }; }}
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
    <!-- Cover -->
    <div class="w-12 h-12 rounded-lg overflow-hidden shrink-0
                bg-neutral-200 dark:bg-neutral-800 shadow-sm">
        {#if album.cover_url}
            <CoverImg path={album.cover_url} alt={album.title} size="1x"
                 class="w-full h-full object-cover" />
        {:else}
            <div class="w-full h-full flex items-center justify-center">
                <Icon icon="lucide:disc-album" width={18} class="text-neutral-400" />
            </div>
        {/if}
    </div>

    <!-- Infos -->
    <div class="flex-1 min-w-0">
        <p class="text-sm font-medium text-neutral-800 dark:text-neutral-200 truncate">
            {album.title}
        </p>
        <p class="text-xs text-neutral-500 dark:text-neutral-400 truncate">
            {album.artist ?? "Artiste inconnu"}
            {#if album.year} • {album.year}{/if}
        </p>
    </div>

    <!-- Stats -->
    <div class="hidden sm:flex items-center gap-4 text-[11px] text-neutral-400 shrink-0">
        <span>{album.total_tracks} titre{album.total_tracks !== 1 ? 's' : ''}</span>
        <span class="uppercase">{album.album_type}</span>
    </div>

    <!-- Chevron -->
    <Icon icon="lucide:chevron-right" width="14"
          class="text-neutral-300 dark:text-neutral-600
                 group-hover:text-neutral-500 transition-colors shrink-0" />
</a>

{#if contextMenu}
    <CollectionContextMenu
        title={album.title}
        type="album"
        loadTracks={loadAlbumTracks}
        x={contextMenu.x}
        y={contextMenu.y}
        onclose={() => contextMenu = null}
        onedittags={() => goto(`/library/${libraryId}/tags?album=${album.id}`)}
        albumId={album.id}
        artistName={album.artist}
    />
{/if}
