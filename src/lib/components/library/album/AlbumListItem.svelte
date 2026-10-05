<script lang="ts">
// Carte d'album de la grille : pochette (badge Hi-Res, menu, lecture au survol), titre, artiste · année, titres.
import type { AlbumListView } from "#lib/types/ui/library/album/AlbumListView";
import type { TrackListView } from "#lib/types/ui/library/track/TrackListView";
import CollectionContextMenu from "#lib/components/ui/contextmenu/CollectionContextMenu.svelte";
import { goto } from "$app/navigation";
import Icon from "@iconify/svelte";
import { invoke } from "@tauri-apps/api/core";
import { t } from "#lib/i18n";
import CoverImg from "#lib/components/ui/image/CoverImg.svelte";
import { preload, preloadAlbumData } from "#lib/actions/preload/preloadAction";
import { handleAlbumEnqueue } from "#lib/actions/queue/QueueAction";
import { selectionStore } from "#lib/stores/ui/selection.store";
import { cleGroupe, toggleGroupSelection } from "#lib/helper/tools/selectionGroups";
import { etiquetteHiRes } from "#lib/helper/tools/audioFormatTools";

let { libraryId, album, onopen }: {
    libraryId: number;
    album: AlbumListView;
    /** Remplace l'ouverture de la page de l'album (le lien reste pour le clic milieu). */
    onopen?: () => void;
} = $props();

let contextMenu = $state<{ x: number; y: number } | null>(null);

const selection = $derived($selectionStore);
const isSelected = $derived(selection.groupes.has(cleGroupe("album", libraryId, album.id)));
const hiRes = $derived(etiquetteHiRes(album.max_bits, album.max_sample_rate));
const titres = $derived(album.total_tracks === 1 ? $t("home.track_one") : $t("home.tracks_n").replace("{n}", String(album.total_tracks)));

// En mode sélection, la carte coche au lieu de naviguer ; le lien reste un lien (survol, clic milieu, préchargement).
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

function menu(e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    contextMenu = { x: e.clientX, y: e.clientY };
}

function lire(e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    handleAlbumEnqueue(album.id);
}

async function loadAlbumTracks() {
    const tracks = await invoke<TrackListView[]>("get_tracks_by_album", { libraryId, libraryAlbumId: album.id });
    return tracks ?? [];
}
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<a
    class="card-album group relative flex flex-col gap-2.5 min-w-0 cursor-pointer"
    href={`/library/${libraryId}/albums/${album.id}`}
    onclick={handleCardClick}
    oncontextmenu={menu}
    use:preload={() => preloadAlbumData(libraryId, album.id)}
>
    <!-- Ombre resserrée : `content-visibility` de la carte coupe ce qui dépasse sur les côtés. -->
    <div class="relative aspect-square rounded-[10px] overflow-hidden bg-(--rg-s2) shadow-[0_12px_18px_-10px_rgba(0,0,0,0.35)] dark:shadow-[0_12px_20px_-10px_rgba(0,0,0,0.7)]
">
        {#if album.cover_url}
            <CoverImg path={album.cover_url} alt={album.title} size="2x" class="w-full h-full object-cover" />
        {:else}
            <!-- Pas de pochette : hachures et invitation à la récupérer. -->
            <div class="absolute inset-0 flex flex-col items-center justify-center gap-1.5 text-xs font-semibold text-(--rg-mu)
                        bg-[repeating-linear-gradient(135deg,var(--rg-s2)_0_10px,var(--rg-carte)_10px_20px)] shadow-[inset_0_0_0_1px_var(--rg-bd)] rounded-[10px]">
                <Icon icon="material-symbols:image-search-rounded" width="28" />
                {$t("albums_view.missing_cover")}
            </div>
        {/if}

        <div class="absolute inset-0 bg-linear-to-b from-transparent from-50% to-black/55 opacity-0 group-hover:opacity-100 transition-opacity duration-150 pointer-events-none"></div>

        {#if selection.active}
            <!-- Cadre posé par-dessus l'image : un anneau extérieur serait coupé par la carte. -->
            {#if isSelected}<span class="absolute inset-0 z-2 rounded-[10px] ring-3 ring-inset ring-(--rg-g) pointer-events-none"></span>{/if}
            <span class="absolute left-2 top-2 z-3 w-6 h-6 rounded-[7px] border-2 flex items-center justify-center
                         {isSelected ? 'bg-(--rg-g) border-(--rg-g) text-(--rg-on-g)' : 'bg-black/35 border-white/80 text-transparent'}">
                <Icon icon="material-symbols:check-rounded" width="16" />
            </span>
        {:else}
            {#if hiRes}
                <span class="absolute left-2 top-2 z-2 px-1.5 py-0.5 rounded-[5px] text-[10px] font-bold tracking-[0.03em] bg-black/65 text-[#f3d38a]">{hiRes}</span>
            {/if}
            <button type="button" class="absolute right-2 top-2 z-2 w-7.5 h-7.5 rounded-lg flex items-center justify-center cursor-pointer
                                        bg-black/55 text-white opacity-0 group-hover:opacity-100 focus-visible:opacity-100 transition-opacity hover:bg-black/75"
                    title={$t("albums_view.more")} aria-label={$t("albums_view.more")} onclick={menu}>
                <Icon icon="material-symbols:more-horiz" width="18" />
            </button>
            <button type="button" class="absolute right-2.5 bottom-2.5 z-2 w-11 h-11 rounded-full flex items-center justify-center cursor-pointer
                                        bg-(--rg-g) text-(--rg-on-g) shadow-[0_6px_16px_rgba(0,0,0,0.5)]
                                        opacity-0 translate-y-1.5 group-hover:opacity-100 group-hover:translate-y-0 focus-visible:opacity-100 transition-all duration-150 hover:scale-105"
                    title={$t("albums_view.play")} aria-label={`${$t("albums_view.play")} ${album.title}`} onclick={lire}>
                <Icon icon="material-symbols:play-arrow-rounded" width="28" />
            </button>
        {/if}
    </div>

    <div class="text-[15px] font-bold leading-[1.3] line-clamp-2 text-(--rg-tx)" title={album.title}>{album.title}</div>
    <div class="-mt-1.5 flex gap-1.5 min-w-0 text-[13px] text-(--rg-mu)">
        <span class="truncate text-(--rg-tx2)">{album.artist || $t("albums_view.unknown_artist")}</span>
        {#if album.year}<span class="shrink-0">· {album.year}</span>{/if}
    </div>
    <div class="-mt-2 text-xs text-(--rg-mu2)">{titres}</div>
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
