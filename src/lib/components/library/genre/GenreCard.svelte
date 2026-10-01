<script lang="ts">
// Carte de genre : mosaïque de quatre pochettes teintée à la couleur du genre, nom, albums · titres, artistes phares.
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
    nom = genre.name,
    lien = null,
    special = false,
}: {
    libraryId: number;
    genre: GenreView;
    /** « Adele, The Weeknd, Michael Jackson ». */
    artistes?: string;
    /** Nom affiché (le pseudo-genre « Sans genre » n'a pas de vrai nom). */
    nom?: string;
    /** Destination ; par défaut la page du genre. */
    lien?: string | null;
    /** Pseudo-genre : ni lecture, ni sélection, ni menu. */
    special?: boolean;
} = $props();

let menu = $state<{ x: number; y: number } | null>(null);
const h = $derived(special ? 0 : teinte(genre.name));
const cible = $derived(lien ?? `/library/${libraryId}/genres/${encodeURIComponent(genre.name)}`);
const selection = $derived($selectionStore);
const isSelected = $derived(!special && selection.groupes.has(cleGroupe("genre", libraryId, genre.name)));
const nombre = (n: number) => n.toLocaleString($currentLocale);

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

const rond = "w-10 h-10 rounded-full flex items-center justify-center cursor-pointer shadow-[0_6px_16px_rgba(0,0,0,0.5)]";
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<a
    href={cible}
    class="group relative flex flex-col gap-3 min-w-0 cursor-pointer"
    style="--gc: {special ? 'var(--rg-mu)' : `oklch(0.7 0.17 ${h})`}"
    onclick={clic}
    oncontextmenu={(e) => { if (special) return; e.preventDefault(); menu = { x: e.clientX, y: e.clientY }; }}
>
    <div class="relative aspect-square rounded-[14px] overflow-hidden grid grid-cols-2 grid-rows-2 gap-0.5 isolate bg-(--rg-bg)
                shadow-[0_10px_28px_rgba(0,0,0,0.25)] dark:shadow-[0_10px_28px_rgba(0,0,0,0.4)] transition-shadow
                group-hover:shadow-[0_0_0_2px_var(--gc),0_14px_32px_rgba(0,0,0,0.4)]
                {isSelected ? 'shadow-[0_0_0_3px_var(--rg-g)]!' : ''}">
        {#each [0, 1, 2, 3] as k (k)}
            {#if genre.covers[k]}
                <CoverImg path={genre.covers[k]} alt="" size="1x" class="w-full h-full object-cover" />
            {:else}
                <!-- Case sans pochette : un dégradé dans la teinte du genre. -->
                <span style="background: linear-gradient({135 + k * 40}deg, oklch({0.55 - k * 0.06} {special ? 0 : 0.1} {(h + k * 23) % 360}), oklch(0.2 {special ? 0 : 0.04} {(h + k * 23) % 360}))"></span>
            {/if}
        {/each}
        <div class="absolute inset-0 z-1 opacity-90 pointer-events-none bg-[linear-gradient(180deg,transparent_35%,color-mix(in_oklch,var(--gc)_35%,#000_65%)_100%)]"></div>

        <div class="absolute left-3.5 right-3.5 bottom-3 z-2 flex flex-col gap-0.5">
            <span class="w-7 h-0.75 mb-2 rounded-full bg-(--gc)"></span>
            <span class="text-[22px] max-[900px]:text-lg font-extrabold tracking-[-0.02em] leading-[1.1] text-white line-clamp-2 [text-shadow:0_2px_12px_rgba(0,0,0,0.5)]" title={nom}>{nom}</span>
        </div>

        {#if selection.active && !special}
            <span class="absolute left-3 top-3 z-3 w-6 h-6 rounded-[7px] border-2 flex items-center justify-center
                         {isSelected ? 'bg-(--rg-g) border-(--rg-g) text-(--rg-on-g)' : 'bg-black/35 border-white/85 text-transparent'}">
                <Icon icon="material-symbols:check-rounded" width="16" />
            </span>
        {:else if !special}
            <div class="absolute right-3 top-3 z-3 flex gap-1.5 opacity-0 -translate-y-1 group-hover:opacity-100 group-hover:translate-y-0 focus-within:opacity-100 transition-all duration-150">
                <button type="button" class="{rond} bg-black/60 text-white hover:bg-black/80" title={$t("genres_view.more")} aria-label={$t("genres_view.more")}
                        onclick={(e) => sansNaviguer(e, () => (menu = { x: e.clientX, y: e.clientY }))}>
                    <Icon icon="material-symbols:more-horiz" width="20" />
                </button>
                <button type="button" class="{rond} bg-black/60 text-white hover:bg-black/80" title={$t("genres_view.shuffle")} aria-label={$t("genres_view.shuffle")}
                        onclick={(e) => sansNaviguer(e, () => handleGenrePlay(libraryId, genre.name, true))}>
                    <Icon icon="material-symbols:shuffle-rounded" width="20" />
                </button>
                <button type="button" class="{rond} bg-(--rg-g) text-(--rg-on-g)" title={$t("genres_view.play")} aria-label={`${$t("genres_view.play")} ${nom}`}
                        onclick={(e) => sansNaviguer(e, () => handleGenrePlay(libraryId, genre.name))}>
                    <Icon icon="material-symbols:play-arrow-rounded" width="24" />
                </button>
            </div>
        {/if}
    </div>

    <div class="flex flex-col gap-0.75 px-0.5 min-w-0">
        <div class="flex gap-1.5 whitespace-nowrap text-[13px] text-(--rg-mu)">
            <span><b class="font-semibold text-(--rg-tx2)">{nombre(genre.total_albums)}</b> {$t(genre.total_albums === 1 ? "library_head.albums_one" : "library_head.albums_n")}</span>
            <span>·</span>
            <span><b class="font-semibold text-(--rg-tx2)">{nombre(genre.total_tracks)}</b> {$t(genre.total_tracks === 1 ? "library_head.tracks_one" : "library_head.tracks_n")}</span>
        </div>
        {#if artistes}<div class="truncate text-[12.5px] text-(--rg-mu2)" title={artistes}>{artistes}</div>{/if}
    </div>
</a>

{#if menu}
    <CollectionContextMenu title={nom} type="genre" loadTracks={pistes} x={menu.x} y={menu.y} onclose={() => (menu = null)} {libraryId} />
{/if}
