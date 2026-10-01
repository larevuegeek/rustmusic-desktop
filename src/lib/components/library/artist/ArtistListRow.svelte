<script lang="ts">
// Ligne d'artiste de la liste : portrait (lecture au survol), nom, albums, titres, durée, actions.
import type { ArtistListView } from "$lib/types/ui/library/artist/ArtistListView";
import Icon from "@iconify/svelte";
import { t } from "$lib/i18n";
import CoverImg from "$lib/components/ui/image/CoverImg.svelte";
import LibraryItemMenu from "$lib/components/ui/contextmenu/LibraryItemMenu.svelte";
import { handleArtistAddToQueue, handleArtistPlay } from "$lib/actions/queue/QueueAction";
import { artistImageReadyStore } from "$lib/stores/library/artistImageReady.store";
import { selectionStore } from "$lib/stores/ui/selection.store";
import { cleGroupe, toggleGroupSelection } from "$lib/helper/tools/selectionGroups";
import { dureeEcoute } from "$lib/helper/tools/dateTools";

let { libraryId, artist }: { libraryId: number; artist: ArtistListView } = $props();

let menu = $state<{ x: number; y: number } | null>(null);
const portrait = $derived(artistImageReadyStore.get(artist.id, $artistImageReadyStore) ?? artist.thumbnail_path ?? null);

const selection = $derived($selectionStore);
const isSelected = $derived(selection.groupes.has(cleGroupe("artist", libraryId, artist.id)));

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

const act = "w-8.5 h-8.5 flex items-center justify-center rounded-[9px] cursor-pointer transition-colors text-(--rg-mu) hover:bg-(--rg-s2) hover:text-(--rg-tx)";
const chiffre = "text-right text-[13px] tabular-nums whitespace-nowrap text-(--rg-mu)";
</script>

<!-- Trait de séparation en ::before toujours absolu : dans une grille, un ::before en flux deviendrait une cellule. -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<a
    class="group relative grid items-center gap-4 py-2 pl-2 pr-3 rounded-xl cursor-pointer transition-colors
           before:absolute before:top-0 before:right-3 before:h-px [a+&]:before:bg-(--rg-line)
           hover:before:opacity-0 [&:hover+a]:before:opacity-0
           {selection.active
             ? 'grid-cols-[20px_56px_minmax(0,1fr)_90px_90px_90px_110px] max-[900px]:grid-cols-[20px_48px_minmax(0,1fr)_80px_40px] before:left-28'
             : 'grid-cols-[56px_minmax(0,1fr)_90px_90px_90px_110px] max-[900px]:grid-cols-[48px_minmax(0,1fr)_80px_40px] before:left-19'}
           {isSelected ? 'bg-(--rg-creux-on)' : 'hover:bg-(--rg-carte)'}"
    href={`/library/${libraryId}/artists/${artist.id}`}
    onclick={handleCardClick}
    oncontextmenu={(e) => { e.preventDefault(); menu = { x: e.clientX, y: e.clientY }; }}
>
    {#if selection.active}
        <span class="w-5 h-5 rounded-md border-2 flex items-center justify-center
                     {isSelected ? 'bg-(--rg-g) border-(--rg-g) text-(--rg-on-g)' : 'border-(--rg-bd2) text-transparent'}">
            <Icon icon="material-symbols:check-rounded" width="14" />
        </span>
    {/if}

    <div class="relative w-14 h-14 max-[900px]:w-12 max-[900px]:h-12 rounded-full overflow-hidden bg-(--rg-s2) shadow-[0_4px_12px_rgba(0,0,0,0.2)] dark:shadow-[0_4px_12px_rgba(0,0,0,0.35)]">
        {#if portrait}
            <CoverImg path={portrait} alt={artist.name} size="1x" class="w-full h-full object-cover" />
        {:else}
            <div class="absolute inset-0 rounded-full flex items-center justify-center text-(--rg-mu2) shadow-[inset_0_0_0_1px_var(--rg-bd)]
                        bg-[repeating-linear-gradient(135deg,var(--rg-s2)_0_8px,var(--rg-carte)_8px_16px)]">
                <Icon icon="material-symbols:person-outline-rounded" width="22" />
            </div>
        {/if}
        {#if !selection.active}
            <button type="button" class="absolute inset-0 flex items-center justify-center bg-black/50 opacity-0 group-hover:opacity-100 focus-visible:opacity-100 transition-opacity cursor-pointer"
                    title={$t("artists_view.play")} aria-label={`${$t("artists_view.play")} ${artist.name}`} onclick={(e) => sansNaviguer(e, () => handleArtistPlay(libraryId, artist.id))}>
                <span class="w-8 h-8 rounded-full flex items-center justify-center bg-(--rg-g) text-(--rg-on-g)">
                    <Icon icon="material-symbols:play-arrow-rounded" width="20" />
                </span>
            </button>
        {/if}
    </div>

    <p class="min-w-0 truncate text-base font-semibold text-(--rg-tx)">{artist.name}</p>

    <span class={chiffre}><b class="font-medium text-(--rg-tx2)">{artist.total_albums}</b> {$t(artist.total_albums === 1 ? "library_head.albums_one" : "library_head.albums_n")}</span>
    <span class="{chiffre} max-[900px]:hidden"><b class="font-medium text-(--rg-tx2)">{artist.total_tracks}</b> {$t(artist.total_tracks === 1 ? "library_head.tracks_one" : "library_head.tracks_n")}</span>
    <span class="{chiffre} max-[900px]:hidden">{artist.total_duration ? dureeEcoute(artist.total_duration) : ""}</span>

    <div class="flex items-center justify-end gap-0.5 {selection.active ? 'invisible' : ''}">
        <button type="button" class="{act} opacity-0 group-hover:opacity-100 focus-visible:opacity-100 max-[900px]:hidden" title={$t("artists_view.enqueue")} aria-label={$t("artists_view.enqueue")}
                onclick={(e) => sansNaviguer(e, () => handleArtistAddToQueue(libraryId, artist.id))}>
            <Icon icon="material-symbols:queue-music-rounded" width="19" />
        </button>
        <button type="button" class="{act} opacity-0 group-hover:opacity-100 focus-visible:opacity-100 max-[900px]:hidden" title={$t("artists_view.more")} aria-label={$t("artists_view.more")}
                onclick={(e) => sansNaviguer(e, () => (menu = { x: e.clientX, y: e.clientY }))}>
            <Icon icon="material-symbols:more-horiz" width="19" />
        </button>
        <span class="w-8.5 h-8.5 flex items-center justify-center text-(--rg-mu2) group-hover:text-(--rg-tx2)" title={$t("artists_view.open")}>
            <Icon icon="material-symbols:chevron-right-rounded" width="19" />
        </span>
    </div>
</a>

{#if menu}
    <LibraryItemMenu kind="artist" id={artist.id} title={artist.name} {libraryId} x={menu.x} y={menu.y} onclose={() => (menu = null)} />
{/if}
