<script lang="ts">
import type { AlbumListView } from "$lib/types/ui/library/album/AlbumListView";
import type { TrackListView } from "$lib/types/ui/library/track/TrackListView";
import CollectionContextMenu from "$lib/components/ui/contextmenu/CollectionContextMenu.svelte";
import { goto } from "$app/navigation";
import Icon from "@iconify/svelte";
import { invoke } from "@tauri-apps/api/core";
import CoverImg from "$lib/components/ui/image/CoverImg.svelte";
import { preload, preloadAlbumData } from "$lib/actions/preload/preloadAction";
import { handleAlbumEnqueue } from "$lib/actions/queue/QueueAction";
import { selectionStore } from "$lib/stores/ui/selection.store";
import { cleGroupe, toggleGroupSelection } from "$lib/helper/tools/selectionGroups";

let { libraryId, album }: { libraryId: number; album: AlbumListView } = $props();

let contextMenu = $state<{ x: number; y: number } | null>(null);

const selection = $derived($selectionStore);
const cle = $derived(cleGroupe('album', libraryId, album.id));
const isSelected = $derived(selection.groupes.has(cle));

/**
 * En mode sélection, la carte coche au lieu de naviguer.
 *
 * Le lien reste un lien — c'est ce qui donne le survol, le clic milieu et le
 * préchargement — mais on empêche la navigation tant que la sélection est
 * active. Basculer entre `<a>` et `<div>` selon le mode ferait perdre tout ça.
 */
function handleCardClick(e: MouseEvent) {
    if (!selection.active) return;
    e.preventDefault();
    e.stopPropagation();
    toggleGroupSelection('album', libraryId, album.id);
}

function handleContextMenu(e: MouseEvent) {
    e.preventDefault();
    contextMenu = { x: e.clientX, y: e.clientY };
}

function handlePlay(e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    handleAlbumEnqueue(album.id);
}

function handleMenuClick(e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    contextMenu = { x: e.clientX, y: e.clientY };
}

async function loadAlbumTracks() {
    const tracks = await invoke<TrackListView[]>('get_tracks_by_album', {
        libraryId,
        libraryAlbumId: album.id
    });
    return tracks ?? [];
}
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<a
    class="card-album group flex flex-col cursor-pointer relative"
    href={`/library/${libraryId}/albums/${album.id}`}
    onclick={handleCardClick}
    oncontextmenu={handleContextMenu}
    use:preload={() => preloadAlbumData(libraryId, album.id)}
>

    <!-- Case de sélection.
         Sur la pochette et non à côté : une grille de cartes n'a pas de marge
         où loger une colonne de cases, et le coin haut-gauche est le seul
         endroit qu'aucune vignette n'utilise. Elle n'apparaît qu'en mode
         sélection, pour ne pas encombrer la navigation ordinaire. -->
    {#if selection.active}
        <div class="absolute top-2 left-2 z-10 w-5 h-5 rounded flex items-center justify-center
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

    <!-- COVER -->
    <div class="aspect-square rounded-lg overflow-hidden
                bg-neutral-200 dark:bg-neutral-800
                shadow-sm group-hover:shadow-md
                transition-all duration-200 relative">

        {#if album.cover_url}
        <CoverImg
            path={album.cover_url}
            alt={album.title}
            size="2x"
            class="w-full h-full object-cover group-hover:scale-105 transition-transform duration-300"
        />
        {:else}
        <div class="w-full h-full flex items-center justify-center">
            <Icon icon="lucide:disc-album" width={32} class="text-neutral-400" />
        </div>
        {/if}

        <!-- Overlay hover -->
        <div class="absolute inset-0 bg-linear-to-t from-black/80 via-black/20 to-transparent
                    opacity-0 group-hover:opacity-100 transition-opacity duration-300"></div>

        <!-- Bouton Play -->
        <button
            type="button"
            class="absolute bottom-2 left-2 w-8 h-8 rounded-full
                   bg-emerald-400 text-black
                   flex items-center justify-center
                   opacity-0 group-hover:opacity-100
                   scale-75 group-hover:scale-100
                   transition-all duration-300 ease-out
                   hover:brightness-110
                   active:scale-90
                   cursor-pointer
                   shadow-[0_2px_10px_rgba(52,211,153,0.4)]"
            onclick={handlePlay}
            aria-label="Lire {album.title}"
        >
            <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor">
                <path d="M8 5.14v14l11-7-11-7z"/>
            </svg>
        </button>

        <!-- Bouton Menu -->
        <button
            type="button"
            class="absolute bottom-2 right-2 w-7 h-7 rounded-full
                   bg-white/10 group-hover:backdrop-blur-md text-white/60
                   flex items-center justify-center
                   opacity-0 group-hover:opacity-100
                   scale-75 group-hover:scale-100
                   transition-all duration-300 ease-out
                   hover:bg-white/20 hover:text-white
                   active:scale-90
                   cursor-pointer"
            onclick={handleMenuClick}
            aria-label="Actions"
        >
            <Icon icon="lucide:ellipsis" width={13} />
        </button>
    </div>

    <!-- TEXT -->
    <div class="mt-3 flex flex-col gap-1 min-w-0">
        <span class="font-medium text-neutral-800 dark:text-neutral-200 truncate">
        {album.title}
        </span>

        <span class="text-xs text-neutral-500 dark:text-neutral-400 truncate">
        {album.artist ?? "Unknown Artist"}
        {#if album.year}
            • {album.year}
        {/if}
        </span>

        <span class="text-[11px] text-neutral-400 dark:text-neutral-500 uppercase tracking-wide">
        {album.album_type}
        • {album.total_tracks} titres
        </span>
    </div>
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
