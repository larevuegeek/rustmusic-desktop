<script lang="ts">
// Carte de compositeur de la grille : photo s'il est aussi un artiste connu, sinon son initiale ; lecture au survol.
import Icon from "@iconify/svelte";
import { t } from "#lib/i18n";
import CoverImg from "#lib/components/ui/image/CoverImg.svelte";
import ArtistAvatar from "#lib/components/ui/image/ArtistAvatar.svelte";
import { handleComposerPlay } from "#lib/actions/queue/QueueAction";
import ComposerMenu from "./ComposerMenu.svelte";
import { composerHref, type ComposerListView } from "#lib/types/ui/library/composer/ComposerListView";

let { libraryId, composer }: { libraryId: number; composer: ComposerListView } = $props();

let menu = $state<{ x: number; y: number } | null>(null);
const countLabel = (n: number, un: string, plusieurs: string) => `${n} ${$t(n === 1 ? un : plusieurs)}`;

function play(e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    void handleComposerPlay(libraryId, composer.name);
}
</script>

<a href={composerHref(libraryId, composer.name)} class="card-artist group relative flex flex-col items-center gap-2.5 min-w-0 text-center cursor-pointer"
   oncontextmenu={(e) => { e.preventDefault(); menu = { x: e.clientX, y: e.clientY }; }}>
    <div class="relative w-full aspect-square">
        <div class="absolute inset-0 rounded-full overflow-hidden bg-(--rg-s2) shadow-[0_12px_18px_-10px_rgba(0,0,0,0.35)] dark:shadow-[0_12px_20px_-10px_rgba(0,0,0,0.7)]">
            {#if composer.thumbnail_path}
                <CoverImg path={composer.thumbnail_path} alt={composer.name} size="2x" class="w-full h-full object-cover" />
            {:else}
                <ArtistAvatar name={composer.name} deux class="w-full h-full text-[52px] tracking-[-0.02em]" />
            {/if}
            <div class="absolute inset-0 bg-black/0 group-hover:bg-black/25 transition-colors duration-150 pointer-events-none"></div>
        </div>
        <button type="button" class="absolute right-1 top-1 z-2 w-7.5 h-7.5 rounded-lg flex items-center justify-center cursor-pointer
                                    bg-black/55 text-white opacity-0 group-hover:opacity-100 focus-visible:opacity-100 transition-opacity hover:bg-black/75"
                title={$t("artists_view.more")} aria-label={$t("artists_view.more")}
                onclick={(e) => { e.preventDefault(); e.stopPropagation(); menu = { x: e.clientX, y: e.clientY }; }}>
            <Icon icon="material-symbols:more-horiz" width="18" />
        </button>
        <button type="button" class="absolute right-[7%] bottom-[7%] z-2 w-11 h-11 rounded-full flex items-center justify-center cursor-pointer
                                    bg-(--rg-g) text-(--rg-on-g) shadow-[0_6px_16px_rgba(0,0,0,0.5)]
                                    opacity-0 translate-y-1.5 group-hover:opacity-100 group-hover:translate-y-0 focus-visible:opacity-100 transition-all duration-150 hover:scale-105"
                title={$t("artists_view.play")} aria-label={`${$t("artists_view.play")} ${composer.name}`} onclick={play}>
            <Icon icon="material-symbols:play-arrow-rounded" width="28" />
        </button>
    </div>

    <div class="w-full text-[15px] font-bold leading-[1.3] truncate text-(--rg-tx)" title={composer.name}>{composer.name}</div>
    <div class="-mt-1.5 w-full truncate text-[13px] text-(--rg-mu)">
        {countLabel(composer.total_albums, "library_head.albums_one", "library_head.albums_n")} · {countLabel(composer.total_tracks, "library_head.tracks_one", "library_head.tracks_n")}
    </div>
</a>

{#if menu}<ComposerMenu {libraryId} name={composer.name} x={menu.x} y={menu.y} onclose={() => (menu = null)} />{/if}
