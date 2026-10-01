<script lang="ts">
// Titre d'une page d'album en vue grille : numéro (lecture au survol), jaquette, titre, invités, durée, j'aime.
import Icon from "@iconify/svelte";
import { t } from "$lib/i18n";
import { handleSelectTrack, handlePlayTrack } from "$lib/actions/player/PlayerAction";
import { versFileDAttente } from "$lib/mapper/queue/mapQueueTrack";
import TrackContextMenu from "$lib/components/ui/contextmenu/TrackContextMenu.svelte";
import CoverImg from "$lib/components/ui/image/CoverImg.svelte";
import { liked } from "$lib/stores/playlist/like.store";
import { lecture } from "$lib/stores/player/lecture.store";
import { selectionStore } from "$lib/stores/ui/selection.store";
import { settingsStore } from "$lib/stores/settings/settings.store";
import { minutesSecondes } from "$lib/helper/tools/dateTools";
import type { TrackListView } from "$lib/types/ui/library/track/TrackListView";

let {
    libraryId,
    track,
    tracks = [],
    jaquette = true,
    artisteAlbum = null,
    position = null,
    sousTitre = null,
    numero = null,
    marque = false,
}: {
    libraryId: number;
    track: TrackListView;
    /** La liste affichée, pour que la lecture enchaîne après ce titre. */
    tracks?: TrackListView[];
    jaquette?: boolean;
    /** L'artiste de l'album : un autre interprète s'affiche sous le titre. */
    artisteAlbum?: string | null;
    /** Rang dans l'ordre du disque, affiché quand les tags n'ont pas de numéro. */
    position?: number | null;
    /** Remplace l'interprète sous le titre (« Album · 2012 »). */
    sousTitre?: string | null;
    /** Numéro affiché à la place de celui de la piste (rang dans une liste à plat). */
    numero?: number | null;
    /** Le titre de la page (fiche d'un morceau) : fond marqué. */
    marque?: boolean;
} = $props();

let contextMenu = $state<{ x: number; y: number } | null>(null);
const aime = $derived($liked.paths.has(track.path));
const selection = $derived($selectionStore);
const coche = $derived(selection.active && selection.ids.has(track.id));
const unClic = $derived($settingsStore.single_click_play === "true");
const enCours = $derived($lecture.path === track.path);
const joue = $derived(enCours && $lecture.status === "playing");
const invite = $derived(sousTitre ?? (track.artist && track.artist !== artisteAlbum ? track.artist : null));

function lire() {
    handlePlayTrack(track.path, versFileDAttente(tracks));
}

// Toute la ligne : cocher, lire ou précharger, selon le mode et le réglage « un clic ».
function clic(e: MouseEvent) {
    if (selection.active) {
        if (e.shiftKey) selectionStore.selectRange(track.id);
        else selectionStore.toggle(track.id, track);
    } else if (unClic) lire();
    else handleSelectTrack(track.path, versFileDAttente(tracks));
}

const ib = "w-8.5 h-8.5 shrink-0 flex items-center justify-center rounded-[9px] cursor-pointer transition-colors text-(--rg-mu) hover:bg-(--rg-s2) hover:text-(--rg-tx)";
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<div
    data-piste={track.id}
    class="row-album-track group relative grid items-center gap-3.5 h-16 px-3 rounded-[10px] transition-colors
           {jaquette ? 'grid-cols-[32px_46px_minmax(0,1fr)_48px_72px]' : 'grid-cols-[32px_minmax(0,1fr)_48px_72px]'}
           {coche ? 'bg-(--rg-creux-on)' : marque ? 'bg-(--rg-carte) shadow-[inset_0_0_0_1px_var(--rg-bd)]' : 'hover:bg-(--rg-carte)'}
           {selection.active ? 'cursor-pointer select-none' : 'cursor-default'}"
    ondblclick={() => { if (!selection.active && !unClic) lire(); }}
    onclick={clic}
    oncontextmenu={(e) => { e.preventDefault(); contextMenu = { x: e.clientX, y: e.clientY }; }}
>
    <!-- Numéro ; au survol, lecture ; en cours, égaliseur ; en sélection, case. -->
    <div class="relative w-8 h-8 flex items-center justify-center">
        {#if selection.active}
            <span class="w-5 h-5 rounded-md border-2 flex items-center justify-center
                         {coche ? 'bg-(--rg-g) border-(--rg-g) text-(--rg-on-g)' : 'border-(--rg-bd2) text-transparent'}">
                <Icon icon="material-symbols:check-rounded" width="14" />
            </span>
        {:else}
            <span class="text-[15px] tabular-nums group-hover:invisible {enCours ? 'hidden' : ''} {track.track_number == null && numero == null ? 'text-(--rg-mu2)' : 'text-(--rg-mu)'}"
                  title={numero == null && track.track_number == null && position ? $t("album_view.no_number") : undefined}>{numero ?? track.track_number ?? position ?? "—"}</span>
            {#if enCours}
                <span class="mini-eq flex items-end gap-0.5 h-3.5 group-hover:hidden" style="--eq: var(--rg-g)" data-pause={joue ? undefined : ""}><i></i><i></i><i></i></span>
            {/if}
            <button type="button" class="absolute inset-0 hidden group-hover:flex items-center justify-center cursor-pointer text-(--rg-tx)"
                    title={$t("tracks_view.play")} aria-label={$t("tracks_view.play")} onclick={(e) => { e.stopPropagation(); lire(); }} ondblclick={(e) => e.stopPropagation()}>
                <Icon icon="material-symbols:play-arrow-rounded" width="26" />
            </button>
        {/if}
    </div>

    {#if jaquette}
        <div class="w-11.5 h-11.5 rounded-[7px] overflow-hidden shadow-[0_4px_12px_rgba(0,0,0,0.2)] dark:shadow-[0_4px_12px_rgba(0,0,0,0.35)]">
            {#if track.thumbnail_path}
                <CoverImg path={track.thumbnail_path} alt="" size="1x" class="w-full h-full object-cover" />
            {:else}
                <div class="w-full h-full flex items-center justify-center text-(--rg-mu2) shadow-[inset_0_0_0_1px_var(--rg-bd)]
                            bg-[repeating-linear-gradient(135deg,var(--rg-s2)_0_6px,var(--rg-carte)_6px_12px)]">
                    <Icon icon="material-symbols:music-note-rounded" width="18" />
                </div>
            {/if}
        </div>
    {/if}

    <div class="min-w-0 flex flex-col gap-0.5">
        <p class="truncate text-[15px] font-semibold {enCours ? 'text-(--rg-g)' : 'text-(--rg-tx)'}" title={track.title}>{track.title}</p>
        {#if invite}
            <p class="truncate text-[12.5px] text-(--rg-mu)" title={invite}>{invite}</p>
        {/if}
    </div>

    <span class="text-right text-sm tabular-nums text-(--rg-mu)">{track.duration ? minutesSecondes(track.duration) : ""}</span>

    <div class="flex justify-end gap-0.5 {selection.active ? 'invisible' : ''}">
        <button type="button" class="{ib} {aime ? 'text-(--rg-g)! opacity-100' : 'opacity-0 group-hover:opacity-100 focus-visible:opacity-100'}"
                title={$t("tracks_view.like")} aria-label={$t("tracks_view.like")} aria-pressed={aime}
                onclick={(e) => { e.stopPropagation(); liked.toggle(track.path); }} ondblclick={(e) => e.stopPropagation()}>
            <Icon icon={aime ? "material-symbols:favorite-rounded" : "material-symbols:favorite-outline-rounded"} width="19" />
        </button>
        <button type="button" class="{ib} {contextMenu ? 'opacity-100' : 'opacity-0 group-hover:opacity-100 focus-visible:opacity-100'}"
                title={$t("tracks_view.more")} aria-label={$t("tracks_view.more")}
                onclick={(e) => { e.stopPropagation(); const r = (e.currentTarget as HTMLElement).getBoundingClientRect(); contextMenu = { x: r.right - 250, y: r.bottom + 6 }; }}
                ondblclick={(e) => e.stopPropagation()}>
            <Icon icon="material-symbols:more-horiz" width="19" />
        </button>
    </div>
</div>

{#if contextMenu}
    <TrackContextMenu {track} x={contextMenu.x} y={contextMenu.y} {libraryId} showNavigation={true} onclose={() => (contextMenu = null)} />
{/if}
