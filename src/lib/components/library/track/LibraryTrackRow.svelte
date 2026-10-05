<script lang="ts">
// Ligne de l'onglet Morceaux : colonnes au choix, habillées comme la maquette (compact ou détaillé).
import { goto } from "$app/navigation";
import Icon from "@iconify/svelte";
import { t, currentLocale } from "#lib/i18n";
import { handleSelectTrack, handlePlayTrack } from "#lib/actions/player/PlayerAction";
import { versFileDAttente } from "#lib/mapper/queue/mapQueueTrack";
import TrackContextMenu from "#lib/components/ui/contextmenu/TrackContextMenu.svelte";
import CoverImg from "#lib/components/ui/image/CoverImg.svelte";
import StarRating from "#lib/components/ui/rating/StarRating.svelte";
import { liked } from "#lib/stores/playlist/like.store";
import { lecture } from "#lib/stores/player/lecture.store";
import { selectionStore } from "#lib/stores/ui/selection.store";
import { settingsStore } from "#lib/stores/settings/settings.store";
import type { TrackColumn } from "#lib/config/trackColumns";
import { formatBitrate, formatCourt, palierPiste } from "#lib/helper/tools/audioFormatTools";
import { minutesSecondes } from "#lib/helper/tools/dateTools";
import type { TrackListView } from "#lib/types/ui/library/track/TrackListView";

let {
    libraryId,
    track,
    tracks = [],
    columns = [],
    souple = -1,
    detaille = false,
    artisteSousTitre = true,
    rang = null,
}: {
    libraryId: number;
    track: TrackListView;
    /** La liste affichée, pour que la lecture enchaîne après ce morceau. */
    tracks?: TrackListView[];
    columns?: TrackColumn[];
    /** Index de la colonne qui absorbe la place (le titre) ; les largeurs viennent des variables `--cw-i` du tableau. */
    souple?: number;
    detaille?: boolean;
    /** L'artiste sous le titre (la colonne Artiste n'est alors pas affichée). */
    artisteSousTitre?: boolean;
    /** Affiché quand les tags n'ont pas de numéro. */
    rang?: number | null;
} = $props();

let contextMenu = $state<{ x: number; y: number } | null>(null);
const aime = $derived($liked.paths.has(track.path));
const selection = $derived($selectionStore);
const coche = $derived(selection.active && selection.ids.has(track.id));
const unClic = $derived($settingsStore.single_click_play === "true");
const enCours = $derived($lecture.path === track.path);
const joue = $derived(enCours && $lecture.status === "playing");

const palier = $derived(palierPiste(track.audio_format, track.bits_per_sample, track.sample_rate));
const PALIERS = {
    hires: { cle: "tracks_view.hires", classes: "text-amber-700 border-amber-300 bg-amber-50 dark:text-[#e8c46a] dark:border-[#4a3d1c] dark:bg-[#231d0e]" },
    lossless: { cle: "tracks_view.lossless", classes: "text-emerald-700 border-(--rg-gbd) bg-(--rg-gbg) dark:text-[#6ee7a0] dark:bg-[#0f2016]" },
    lossy: { cle: "tracks_view.lossy", classes: "text-(--rg-mu) border-(--rg-bd) bg-(--rg-carte) dark:border-[#2a312d]" },
};
// « FLAC · 24 bit · 96 kHz », ou le débit pour un format avec perte.
const fiche = $derived.by(() => {
    const f = (track.audio_format ?? "").toUpperCase();
    if (track.bits_per_sample && track.bits_per_sample > 1 && track.sample_rate) {
        const khz = (track.sample_rate / 1000).toLocaleString($currentLocale, { maximumFractionDigits: 1, useGrouping: false });
        return `${f} · ${track.bits_per_sample} bit · ${khz} kHz`;
    }
    return track.bitrate ? `${f} · ${formatBitrate(track.bitrate)}` : formatCourt(track.audio_format, track.bits_per_sample, track.sample_rate);
});

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

function aller(e: MouseEvent, cible: string | null) {
    e.stopPropagation();
    if (cible) goto(cible);
}

const lienArtiste = $derived(track.library_artist_id ? `/library/${libraryId}/artists/${track.library_artist_id}` : null);
const lienAlbum = $derived(track.album_id ? `/library/${libraryId}/albums/${track.album_id}` : null);
const lien = "text-(--rg-tx2) hover:text-(--rg-tx) hover:underline underline-offset-2 cursor-pointer";
const ib = "w-8 h-8 shrink-0 flex items-center justify-center rounded-lg cursor-pointer transition-colors text-(--rg-mu) hover:bg-(--rg-s2) hover:text-(--rg-tx)";
</script>

{#snippet badge()}
    <span class="shrink-0 px-1.5 py-0.5 rounded-[5px] border text-[10.5px] font-bold tracking-[0.04em] {PALIERS[palier].classes}">{$t(PALIERS[palier].cle)}</span>
{/snippet}

<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- Trait de séparation en ::after absolu, masqué autour de la ligne survolée ou cochée. -->
<div
    data-piste={track.id}
    class="ligne-morceau group relative flex items-center gap-3.5 px-2.5 rounded-[10px] transition-colors scroll-mt-11
           {detaille ? 'h-23 row-track-detail' : 'h-14.5 row-track'}
           after:absolute after:left-2.5 after:right-2.5 after:top-0 after:h-px [.ligne-morceau+&]:after:bg-(--rg-line)
           hover:after:opacity-0 [&:hover+.ligne-morceau]:after:opacity-0
           {coche ? 'bg-(--rg-creux-on) after:opacity-0' : 'hover:bg-(--rg-carte)'}
           {selection.active ? 'cursor-pointer' : 'cursor-default'}"
    ondblclick={() => { if (!selection.active && !unClic) lire(); }}
    onclick={clic}
    oncontextmenu={(e) => { e.preventDefault(); contextMenu = { x: e.clientX, y: e.clientY }; }}
>
    {#each columns as col, i (col.key)}
        {@const w = i === souple ? `flex: 1 1 var(--cw-${i}); min-width: ${col.minWidth ?? 0}px` : `width: var(--cw-${i})`}
        {#if col.widget === "index"}
            <!-- Numéro ; au survol, lecture ; en cours, égaliseur ; en sélection, case. -->
            <div class="relative shrink-0 flex items-center justify-center" style={w}>
                {#if selection.active}
                    <span class="w-5 h-5 rounded-md border-2 flex items-center justify-center
                                 {coche ? 'bg-(--rg-g) border-(--rg-g) text-(--rg-on-g)' : 'border-(--rg-bd2) text-transparent'}">
                        <Icon icon="material-symbols:check-rounded" width="14" />
                    </span>
                {:else}
                    <span class="text-[13px] tabular-nums text-(--rg-mu2) group-hover:invisible {enCours ? 'hidden' : ''}"
                          title={track.track_number == null && rang ? $t("album_view.no_number") : undefined}>{track.track_number ?? rang ?? "—"}</span>
                    {#if enCours}
                        <span class="mini-eq flex items-end gap-0.5 h-3.5 group-hover:hidden" style="--eq: var(--rg-g)" data-pause={joue ? undefined : ""}><i></i><i></i><i></i></span>
                    {/if}
                    <button type="button" class="absolute inset-0 hidden group-hover:flex items-center justify-center cursor-pointer text-(--rg-tx)"
                            title={$t("tracks_view.play")} aria-label={$t("tracks_view.play")} onclick={(e) => { e.stopPropagation(); lire(); }} ondblclick={(e) => e.stopPropagation()}>
                        <Icon icon="material-symbols:play-arrow-rounded" width="24" />
                    </button>
                {/if}
            </div>

        {:else if col.widget === "cover"}
            <div class="shrink-0 rounded-md overflow-hidden shadow-[0_3px_10px_rgba(0,0,0,0.25)] dark:shadow-[0_3px_10px_rgba(0,0,0,0.35)]
                        {detaille ? 'h-17' : 'h-10'}" style={w}>
                {#if track.thumbnail_path}
                    <CoverImg path={track.thumbnail_path} alt="" size="1x" class="w-full h-full object-cover" />
                {:else}
                    <div class="w-full h-full flex items-center justify-center text-(--rg-mu2) shadow-[inset_0_0_0_1px_var(--rg-bd)]
                                bg-[repeating-linear-gradient(135deg,var(--rg-s2)_0_6px,var(--rg-carte)_6px_12px)]">
                        <Icon icon="material-symbols:image-search-rounded" width="16" />
                    </div>
                {/if}
            </div>

        {:else if col.widget === "title"}
            <div class={i === souple ? "min-w-0" : "shrink-0 min-w-0"} style={w}>
                <p class="truncate font-semibold {detaille ? 'text-base' : 'text-[15px]'} {enCours ? 'text-(--rg-g)' : 'text-(--rg-tx)'}" title={track.title}>{track.title}</p>
                {#if artisteSousTitre && track.artist}
                    <p class="mt-0.5 truncate text-[13px] text-(--rg-mu)">
                        <!-- svelte-ignore a11y_no_static_element_interactions -->
                        <span class={lienArtiste ? lien : "text-(--rg-tx2)"} onclick={(e) => aller(e, lienArtiste)}>{track.artist}</span>
                    </p>
                {/if}
                {#if detaille}
                    <div class="mt-1.5 flex items-center gap-2 min-w-0 text-xs text-(--rg-mu2)">
                        {#if track.album}
                            <span class="min-w-0 truncate text-(--rg-mu)">
                                <!-- svelte-ignore a11y_no_static_element_interactions -->
                                <span class={lienAlbum ? lien : ""} onclick={(e) => aller(e, lienAlbum)}>{track.album}</span>{#if track.year}{" · "}{track.year}{/if}
                            </span>
                        {/if}
                        {@render badge()}
                        <span class="whitespace-nowrap">{fiche}</span>
                    </div>
                {/if}
            </div>

        {:else if col.widget === "quality"}
            <span class="shrink-0 inline-flex items-center gap-1.5 whitespace-nowrap overflow-hidden" style={w}>
                {@render badge()}
                <span class="text-xs text-(--rg-mu2) truncate">{formatCourt(track.audio_format, track.bits_per_sample, track.sample_rate)}</span>
            </span>

        {:else if col.widget === "rating"}
            <div class="shrink-0 flex" style={w}>
                <StarRating trackId={track.id} value={track.rating} size={16} ton="or" videsAuSurvol />
            </div>

        {:else if col.key === "album"}
            <span class="shrink-0 truncate text-sm text-(--rg-mu)" style={w} title={track.album ?? ""}>
                <!-- svelte-ignore a11y_no_static_element_interactions -->
                {#if track.album}<span class={lienAlbum ? lien : ""} onclick={(e) => aller(e, lienAlbum)}>{track.album}</span>{/if}
            </span>

        {:else if col.key === "duration"}
            <span class="shrink-0 text-right text-[13px] tabular-nums text-(--rg-mu)" style={w}>{track.duration ? minutesSecondes(track.duration) : ""}</span>

        {:else}
            <span class="shrink-0 truncate text-[13px] text-(--rg-mu) {col.align === 'right' ? 'text-right' : ''} {col.numeric ? 'tabular-nums' : ''}"
                  style={w} title={col.value?.(track) ?? ""}>{col.value?.(track) ?? ""}</span>
        {/if}
    {/each}

    <!-- Remplissage si aucune colonne n'absorbe la place : même règle que l'en-tête. -->
    {#if souple < 0}
        <div class="flex-1 min-w-0"></div>
    {/if}

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

{#if contextMenu}
    <TrackContextMenu {track} x={contextMenu.x} y={contextMenu.y} {libraryId} showNavigation={true} onclose={() => (contextMenu = null)} />
{/if}
