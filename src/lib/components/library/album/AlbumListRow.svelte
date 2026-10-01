<script lang="ts">
import type { TrackListView } from "$lib/types/ui/library/track/TrackListView";
// Ligne d'album de la liste : pochette (lecture au survol), titre, artiste · année, titres, durée, actions.
import type { AlbumListView } from "$lib/types/ui/library/album/AlbumListView";
import CollectionContextMenu from "$lib/components/ui/contextmenu/CollectionContextMenu.svelte";
import { goto } from "$app/navigation";
import Icon from "@iconify/svelte";
import { invoke } from "@tauri-apps/api/core";
import { t } from "$lib/i18n";
import { dureeEcoute } from "$lib/helper/tools/dateTools";
import CoverImg from "$lib/components/ui/image/CoverImg.svelte";
import { handleAlbumAddToQueue, handleAlbumEnqueue } from "$lib/actions/queue/QueueAction";
import { selectionStore } from "$lib/stores/ui/selection.store";
import { cleGroupe, toggleGroupSelection } from "$lib/helper/tools/selectionGroups";

let { libraryId, album, onopen }: {
    libraryId: number;
    album: AlbumListView;
    /** Remplace l'ouverture de la page de l'album (le lien reste pour le clic milieu). */
    onopen?: () => void;
} = $props();

let contextMenu = $state<{ x: number; y: number } | null>(null);

const selection = $derived($selectionStore);
const isSelected = $derived(selection.groupes.has(cleGroupe("album", libraryId, album.id)));
const compilation = $derived(album.album_type === "compilation");

// « 1 h 05 min », « 42 min » ; `$t` lu pour suivre la langue.
const duree = $derived.by(() => {
    void $t;
    return dureeEcoute(album.total_duration ?? 0);
});

// En mode sélection, cocher au lieu de naviguer ; le lien reste un lien.
function handleCardClick(e: MouseEvent) {
    if (!selection.active) {
        if (onopen) {
            e.preventDefault();
            onopen();
        }
        return;
    }
    e.preventDefault();
    e.stopPropagation();
    toggleGroupSelection("album", libraryId, album.id);
}

function sansNaviguer(e: MouseEvent, action: () => void) {
    e.preventDefault();
    e.stopPropagation();
    action();
}

async function loadAlbumTracks() {
    return (await invoke<TrackListView[]>("get_tracks_by_album", { libraryId, libraryAlbumId: album.id })) ?? [];
}

const act = "w-8.5 h-8.5 flex items-center justify-center rounded-[9px] cursor-pointer transition-colors text-(--rg-mu) hover:bg-(--rg-s2) hover:text-(--rg-tx)";
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- Le trait de séparation est un ::before toujours absolu : dans la grille, un ::before
     en flux (créé par le survol) deviendrait une cellule et décalerait toute la ligne. -->
<a
    class="group relative grid items-center gap-4 py-2 pl-2 pr-3 rounded-xl cursor-pointer transition-colors
           before:absolute before:top-0 before:right-3 before:h-px [a+&]:before:bg-(--rg-line)
           hover:before:opacity-0 [&:hover+a]:before:opacity-0
           {selection.active
             ? 'grid-cols-[20px_64px_minmax(0,1fr)_90px_90px_110px] max-[900px]:grid-cols-[20px_56px_minmax(0,1fr)_80px_40px] before:left-30'
             : 'grid-cols-[64px_minmax(0,1fr)_90px_90px_110px] max-[900px]:grid-cols-[56px_minmax(0,1fr)_80px_40px] before:left-21'}
           {isSelected ? 'bg-(--rg-creux-on)' : 'hover:bg-(--rg-carte)'}"
    href={`/library/${libraryId}/albums/${album.id}`}
    oncontextmenu={(e) => { e.preventDefault(); contextMenu = { x: e.clientX, y: e.clientY }; }}
    onclick={handleCardClick}
>
    {#if selection.active}
        <span class="w-5 h-5 rounded-md border-2 flex items-center justify-center
                     {isSelected ? 'bg-(--rg-g) border-(--rg-g) text-(--rg-on-g)' : 'border-(--rg-bd2) text-transparent'}">
            <Icon icon="material-symbols:check-rounded" width="14" />
        </span>
    {/if}

    <div class="relative w-16 h-16 max-[900px]:w-14 max-[900px]:h-14 rounded-lg overflow-hidden shadow-[0_4px_12px_rgba(0,0,0,0.2)] dark:shadow-[0_4px_12px_rgba(0,0,0,0.35)] bg-(--rg-s2)">
        {#if album.cover_url}
            <CoverImg path={album.cover_url} alt={album.title} size="1x" class="w-full h-full object-cover" />
        {:else}
            <div class="absolute inset-0 flex items-center justify-center text-(--rg-mu2) shadow-[inset_0_0_0_1px_var(--rg-bd)] rounded-lg
                        bg-[repeating-linear-gradient(135deg,var(--rg-s2)_0_8px,var(--rg-carte)_8px_16px)]">
                <Icon icon="material-symbols:image-search-rounded" width="20" />
            </div>
        {/if}
        {#if !selection.active}
            <button type="button" class="absolute inset-0 flex items-center justify-center bg-black/50 opacity-0 group-hover:opacity-100 focus-visible:opacity-100 transition-opacity cursor-pointer"
                    title={$t("albums_view.play")} aria-label={`${$t("albums_view.play")} ${album.title}`} onclick={(e) => sansNaviguer(e, () => handleAlbumEnqueue(album.id))}>
                <span class="w-8.5 h-8.5 rounded-full flex items-center justify-center bg-(--rg-g) text-(--rg-on-g)">
                    <Icon icon="material-symbols:play-arrow-rounded" width="22" />
                </span>
            </button>
        {/if}
    </div>

    <div class="min-w-0">
        <p class="flex items-center gap-2 min-w-0 text-base font-semibold text-(--rg-tx)">
            <span class="truncate">{album.title}</span>
            {#if compilation}
                <span class="shrink-0 px-1.75 py-0.5 rounded-[5px] border text-[10px] font-bold uppercase tracking-[0.06em] bg-(--rg-s2) border-(--rg-bd) text-(--rg-mu)">{$t("albums_view.compilation")}</span>
            {/if}
        </p>
        <p class="mt-0.75 truncate text-sm text-(--rg-mu)">
            <span class="text-(--rg-tx2)">{album.artist || $t("albums_view.unknown_artist")}</span>{#if album.year}{" · "}{album.year}{/if}
        </p>
    </div>

    <span class="text-right text-[13px] tabular-nums whitespace-nowrap text-(--rg-mu)"><b class="font-medium text-(--rg-tx2)">{album.total_tracks}</b> {album.total_tracks === 1 ? $t("library_head.tracks_one") : $t("library_head.tracks_n")}</span>
    <span class="text-right text-[13px] tabular-nums whitespace-nowrap text-(--rg-mu) max-[900px]:hidden">{duree}</span>

    <div class="flex items-center justify-end gap-0.5 {selection.active ? 'invisible' : ''}">
        <button type="button" class="{act} opacity-0 group-hover:opacity-100 focus-visible:opacity-100 max-[900px]:hidden" title={$t("albums_view.enqueue")} aria-label={$t("albums_view.enqueue")}
                onclick={(e) => sansNaviguer(e, () => handleAlbumAddToQueue(album.id))}>
            <Icon icon="material-symbols:queue-music-rounded" width="19" />
        </button>
        <button type="button" class="{act} opacity-0 group-hover:opacity-100 focus-visible:opacity-100 max-[900px]:hidden" title={$t("albums_view.more")} aria-label={$t("albums_view.more")}
                onclick={(e) => sansNaviguer(e, () => (contextMenu = { x: e.clientX, y: e.clientY }))}>
            <Icon icon="material-symbols:more-horiz" width="19" />
        </button>
        <span class="w-8.5 h-8.5 flex items-center justify-center text-(--rg-mu2) group-hover:text-(--rg-tx2)" title={$t("albums_view.open")}>
            <Icon icon="material-symbols:chevron-right-rounded" width="19" />
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
        onclose={() => (contextMenu = null)}
        onedittags={() => goto(`/library/${libraryId}/tags?album=${album.id}`)}
        albumId={album.id}
        artistName={album.artist}
        {libraryId}
    />
{/if}
