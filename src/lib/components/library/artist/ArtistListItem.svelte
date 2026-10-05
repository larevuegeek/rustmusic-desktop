<script lang="ts">
// Carte d'artiste de la grille : portrait rond (lecture et menu au survol), nom, albums · titres.
import type { ArtistListView } from "#lib/types/ui/library/artist/ArtistListView";
import Icon from "@iconify/svelte";
import { t } from "#lib/i18n";
import CoverImg from "#lib/components/ui/image/CoverImg.svelte";
import LibraryItemMenu from "#lib/components/ui/contextmenu/LibraryItemMenu.svelte";
import { preload, preloadArtistData } from "#lib/actions/preload/preloadAction";
import { handleArtistPlay } from "#lib/actions/queue/QueueAction";
import { artistImageReadyStore } from "#lib/stores/library/artistImageReady.store";
import { selectionStore } from "#lib/stores/ui/selection.store";
import { cleGroupe, toggleGroupSelection } from "#lib/helper/tools/selectionGroups";

let { libraryId, artist }: { libraryId: number; artist: ArtistListView } = $props();

let menu = $state<{ x: number; y: number } | null>(null);
// Le portrait peut arriver pendant que la page est ouverte (récupération Deezer).
const portrait = $derived(artistImageReadyStore.get(artist.id, $artistImageReadyStore) ?? artist.thumbnail_path ?? null);

const selection = $derived($selectionStore);
const isSelected = $derived(selection.groupes.has(cleGroupe("artist", libraryId, artist.id)));
const nb = (n: number, un: string, plusieurs: string) => `${n} ${$t(n === 1 ? un : plusieurs)}`;

// En mode sélection, cocher au lieu de naviguer ; le lien reste un lien.
function handleCardClick(e: MouseEvent) {
    if (!selection.active) return;
    e.preventDefault();
    e.stopPropagation();
    toggleGroupSelection("artist", libraryId, artist.id);
}

function sansNaviguer(e: MouseEvent, action: () => void) {
    e.preventDefault();
    e.stopPropagation();
    action();
}
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<a
    href={`/library/${libraryId}/artists/${artist.id}`}
    class="card-artist group relative flex flex-col items-center gap-2.5 min-w-0 text-center cursor-pointer"
    use:preload={() => preloadArtistData(artist.id)}
    onclick={handleCardClick}
    oncontextmenu={(e) => { e.preventDefault(); menu = { x: e.clientX, y: e.clientY }; }}
>
    <div class="relative w-full aspect-square">
        <!-- Ombre resserrée : `content-visibility` de la carte coupe ce qui dépasse sur les côtés. -->
        <div class="absolute inset-0 rounded-full overflow-hidden bg-(--rg-s2) shadow-[0_12px_18px_-10px_rgba(0,0,0,0.35)] dark:shadow-[0_12px_20px_-10px_rgba(0,0,0,0.7)]">
            {#if portrait}
                <CoverImg path={portrait} alt={artist.name} size="2x" class="w-full h-full object-cover" />
            {:else}
                <div class="absolute inset-0 rounded-full flex flex-col items-center justify-center gap-1.5 text-[11px] font-semibold text-(--rg-mu)
                            bg-[repeating-linear-gradient(135deg,var(--rg-s2)_0_10px,var(--rg-carte)_10px_20px)] shadow-[inset_0_0_0_1px_var(--rg-bd)]">
                    <Icon icon="material-symbols:person-outline-rounded" width="34" />
                    {$t("artists_view.missing_portrait")}
                </div>
            {/if}
            <div class="absolute inset-0 bg-black/0 group-hover:bg-black/25 transition-colors duration-150 pointer-events-none"></div>
            {#if isSelected}<span class="absolute inset-0 rounded-full ring-3 ring-inset ring-(--rg-g) pointer-events-none"></span>{/if}
        </div>

        {#if selection.active}
            <span class="absolute left-1 top-1 z-3 w-6 h-6 rounded-[7px] border-2 flex items-center justify-center
                         {isSelected ? 'bg-(--rg-g) border-(--rg-g) text-(--rg-on-g)' : 'bg-black/35 border-white/80 text-transparent'}">
                <Icon icon="material-symbols:check-rounded" width="16" />
            </span>
        {:else}
            <button type="button" class="absolute right-1 top-1 z-2 w-7.5 h-7.5 rounded-lg flex items-center justify-center cursor-pointer
                                        bg-black/55 text-white opacity-0 group-hover:opacity-100 focus-visible:opacity-100 transition-opacity hover:bg-black/75"
                    title={$t("artists_view.more")} aria-label={$t("artists_view.more")} onclick={(e) => sansNaviguer(e, () => (menu = { x: e.clientX, y: e.clientY }))}>
                <Icon icon="material-symbols:more-horiz" width="18" />
            </button>
            <button type="button" class="absolute right-[7%] bottom-[7%] z-2 w-11 h-11 rounded-full flex items-center justify-center cursor-pointer
                                        bg-(--rg-g) text-(--rg-on-g) shadow-[0_6px_16px_rgba(0,0,0,0.5)]
                                        opacity-0 translate-y-1.5 group-hover:opacity-100 group-hover:translate-y-0 focus-visible:opacity-100 transition-all duration-150 hover:scale-105"
                    title={$t("artists_view.play")} aria-label={`${$t("artists_view.play")} ${artist.name}`} onclick={(e) => sansNaviguer(e, () => handleArtistPlay(libraryId, artist.id))}>
                <Icon icon="material-symbols:play-arrow-rounded" width="28" />
            </button>
        {/if}
    </div>

    <div class="w-full text-[15px] font-bold leading-[1.3] truncate text-(--rg-tx)" title={artist.name}>{artist.name}</div>
    <div class="-mt-1.5 w-full truncate text-[13px] text-(--rg-mu)">
        {nb(artist.total_albums, "library_head.albums_one", "library_head.albums_n")} · {nb(artist.total_tracks, "library_head.tracks_one", "library_head.tracks_n")}
    </div>
</a>

{#if menu}
    <LibraryItemMenu kind="artist" id={artist.id} title={artist.name} {libraryId} x={menu.x} y={menu.y} onclose={() => (menu = null)} />
{/if}
