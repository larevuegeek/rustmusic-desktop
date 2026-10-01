<script lang="ts">
// Ligne de genre : mosaïque, nom (pastille de couleur), artistes phares, part de la bibliothèque, actions.
import Icon from "@iconify/svelte";
import { invoke } from "@tauri-apps/api/core";
import { t, currentLocale } from "$lib/i18n";
import CoverImg from "$lib/components/ui/image/CoverImg.svelte";
import CollectionContextMenu from "$lib/components/ui/contextmenu/CollectionContextMenu.svelte";
import { handleGenrePlay } from "$lib/actions/queue/QueueAction";
import { selectionStore } from "$lib/stores/ui/selection.store";
import { cleGroupe, toggleGroupSelection } from "$lib/helper/tools/selectionGroups";
import { teinte } from "$lib/helper/tools/teinte";
import type { GenreView } from "$lib/types/ui/library/genre/GenreView";
import type { TrackListView } from "$lib/types/ui/library/track/TrackListView";

let {
    libraryId,
    genre,
    artistes = "",
    part = 0,
    barre = 0,
    nom = genre.name,
    lien = null,
    special = false,
}: {
    libraryId: number;
    genre: GenreView;
    artistes?: string;
    /** Part des titres de la bibliothèque, en %. */
    part?: number;
    /** Longueur de la barre, relative au plus gros genre (0–100). */
    barre?: number;
    nom?: string;
    lien?: string | null;
    special?: boolean;
} = $props();

let menu = $state<{ x: number; y: number } | null>(null);
const h = $derived(special ? 0 : teinte(genre.name));
const cible = $derived(lien ?? `/library/${libraryId}/genres/${encodeURIComponent(genre.name)}`);
const selection = $derived($selectionStore);
const isSelected = $derived(!special && selection.groupes.has(cleGroupe("genre", libraryId, genre.name)));
const nombre = (n: number, d = 0) => n.toLocaleString($currentLocale, { maximumFractionDigits: d });

function clic(e: MouseEvent) {
    if (!selection.active || special) return;
    e.preventDefault();
    e.stopPropagation();
    toggleGroupSelection("genre", libraryId, genre.name);
}

function sansNaviguer(e: MouseEvent, action: () => void) {
    e.preventDefault();
    e.stopPropagation();
    action();
}

async function pistes() {
    return (await invoke<TrackListView[]>("get_tracks_by_genre", { libraryId, genre: genre.name })) ?? [];
}

const act = "w-8.5 h-8.5 flex items-center justify-center rounded-[9px] cursor-pointer transition-colors text-(--rg-mu) hover:bg-(--rg-s2) hover:text-(--rg-tx)";
</script>

<!-- Trait de séparation en ::before toujours absolu : dans une grille, un ::before en flux deviendrait une cellule. -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<a
    href={cible}
    class="group relative grid items-center gap-4.5 py-2 pl-2 pr-3 rounded-xl cursor-pointer transition-colors
           before:absolute before:top-0 before:right-3 before:h-px [a+&]:before:bg-(--rg-line)
           hover:before:opacity-0 [&:hover+a]:before:opacity-0
           {selection.active
             ? 'grid-cols-[20px_56px_minmax(0,1.3fr)_minmax(0,1.4fr)_180px_100px] max-[900px]:grid-cols-[20px_56px_minmax(0,1fr)_120px_40px] before:left-28'
             : 'grid-cols-[56px_minmax(0,1.3fr)_minmax(0,1.4fr)_180px_100px] max-[900px]:grid-cols-[56px_minmax(0,1fr)_120px_40px] before:left-19'}
           {isSelected ? 'bg-(--rg-creux-on)' : 'hover:bg-(--rg-carte)'}"
    style="--gc: {special ? 'var(--rg-mu)' : `oklch(0.7 0.17 ${h})`}"
    onclick={clic}
    oncontextmenu={(e) => { if (special) return; e.preventDefault(); menu = { x: e.clientX, y: e.clientY }; }}
>
    {#if selection.active}
        <span class="w-5 h-5 rounded-md border-2 flex items-center justify-center {special ? 'invisible' : ''}
                     {isSelected ? 'bg-(--rg-g) border-(--rg-g) text-(--rg-on-g)' : 'border-(--rg-bd2) text-transparent'}">
            <Icon icon="material-symbols:check-rounded" width="14" />
        </span>
    {/if}

    <div class="w-14 h-14 rounded-[9px] overflow-hidden grid grid-cols-2 grid-rows-2 gap-px bg-(--rg-bg) shadow-[0_4px_12px_rgba(0,0,0,0.2)] dark:shadow-[0_4px_12px_rgba(0,0,0,0.35)]">
        {#each [0, 1, 2, 3] as k (k)}
            {#if genre.covers[k]}
                <CoverImg path={genre.covers[k]} alt="" size="1x" class="w-full h-full object-cover" />
            {:else}
                <span style="background: linear-gradient({135 + k * 40}deg, oklch({0.55 - k * 0.06} {special ? 0 : 0.1} {(h + k * 23) % 360}), oklch(0.2 {special ? 0 : 0.04} {(h + k * 23) % 360}))"></span>
            {/if}
        {/each}
    </div>

    <p class="flex items-center gap-2.5 min-w-0 text-base font-bold text-(--rg-tx)">
        <span class="w-2 h-2 shrink-0 rounded-full bg-(--gc)"></span>
        <span class="truncate">{nom}</span>
    </p>

    <p class="truncate text-[13px] text-(--rg-mu) max-[900px]:hidden" title={artistes}>{artistes}</p>

    <div class="flex flex-col gap-1.5">
        <div class="flex justify-between gap-2 text-[12.5px] tabular-nums text-(--rg-mu)">
            <span class="truncate"><b class="font-semibold text-(--rg-tx2)">{nombre(genre.total_tracks)}</b> {$t(genre.total_tracks === 1 ? "library_head.tracks_one" : "library_head.tracks_n")} · {genre.total_albums} {$t(genre.total_albums === 1 ? "library_head.albums_one" : "library_head.albums_n")}</span>
            <span class="shrink-0 max-[900px]:hidden">{$t("genres_view.share").replace("{p}", nombre(part, part < 1 ? 1 : 0))}</span>
        </div>
        <div class="h-1 rounded-full overflow-hidden bg-(--rg-s2)"><i class="block h-full rounded-full bg-(--gc)" style="width: {barre}%"></i></div>
    </div>

    <div class="flex items-center justify-end gap-0.5 {selection.active ? 'invisible' : ''}">
        {#if !special}
            <button type="button" class="{act} opacity-0 group-hover:opacity-100 focus-visible:opacity-100 max-[900px]:hidden" title={$t("genres_view.shuffle")} aria-label={$t("genres_view.shuffle")}
                    onclick={(e) => sansNaviguer(e, () => handleGenrePlay(libraryId, genre.name, true))}>
                <Icon icon="material-symbols:shuffle-rounded" width="19" />
            </button>
            <button type="button" class="{act} {menu ? 'opacity-100' : 'opacity-0 group-hover:opacity-100'} focus-visible:opacity-100 max-[900px]:hidden" title={$t("genres_view.more")} aria-label={$t("genres_view.more")}
                    onclick={(e) => sansNaviguer(e, () => (menu = { x: e.clientX, y: e.clientY }))}>
                <Icon icon="material-symbols:more-horiz" width="19" />
            </button>
        {/if}
        <span class="w-8.5 h-8.5 flex items-center justify-center text-(--rg-mu2) group-hover:text-(--rg-tx2)" title={$t("genres_view.open")}>
            <Icon icon="material-symbols:chevron-right-rounded" width="19" />
        </span>
    </div>
</a>

{#if menu}
    <CollectionContextMenu title={nom} type="genre" loadTracks={pistes} x={menu.x} y={menu.y} onclose={() => (menu = null)} {libraryId} />
{/if}
