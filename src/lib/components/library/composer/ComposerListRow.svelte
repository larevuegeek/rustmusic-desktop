<script lang="ts">
// Ligne de compositeur de la liste : pastille (lecture au survol), nom, albums, titres, durée, actions.
import Icon from "@iconify/svelte";
import { t } from "#lib/i18n";
import CoverImg from "#lib/components/ui/image/CoverImg.svelte";
import ArtistAvatar from "#lib/components/ui/image/ArtistAvatar.svelte";
import { handleComposerAddToQueue, handleComposerPlay } from "#lib/actions/queue/QueueAction";
import { listenDuration } from "#lib/helper/tools/dateTools";
import { composerHref, type ComposerListView } from "#lib/types/ui/library/composer/ComposerListView";

let { libraryId, composer }: { libraryId: number; composer: ComposerListView } = $props();

function withoutNavigating(e: MouseEvent, action: () => void) {
    e.preventDefault();
    e.stopPropagation();
    action();
}

const act = "w-8.5 h-8.5 flex items-center justify-center rounded-[9px] cursor-pointer transition-colors text-(--rg-mu) hover:bg-(--rg-s2) hover:text-(--rg-tx)";
const chiffre = "text-right text-[13px] tabular-nums whitespace-nowrap text-(--rg-mu)";
</script>

<a
    class="group relative grid items-center gap-4 py-2 pl-2 pr-3 rounded-xl cursor-pointer transition-colors hover:bg-(--rg-carte)
           before:absolute before:top-0 before:right-3 before:left-19 before:h-px [a+&]:before:bg-(--rg-line)
           hover:before:opacity-0 [&:hover+a]:before:opacity-0
           grid-cols-[56px_minmax(0,1fr)_90px_90px_90px_110px] max-[900px]:grid-cols-[48px_minmax(0,1fr)_80px_40px]"
    href={composerHref(libraryId, composer.name)}
>
    <div class="relative w-14 h-14 max-[900px]:w-12 max-[900px]:h-12 rounded-full overflow-hidden bg-(--rg-s2) shadow-[0_4px_12px_rgba(0,0,0,0.2)] dark:shadow-[0_4px_12px_rgba(0,0,0,0.35)]">
        {#if composer.thumbnail_path}
            <CoverImg path={composer.thumbnail_path} alt={composer.name} size="1x" class="w-full h-full object-cover" />
        {:else}
            <ArtistAvatar name={composer.name} deux class="w-full h-full text-[18px]" />
        {/if}
        <button type="button" class="absolute inset-0 flex items-center justify-center bg-black/50 opacity-0 group-hover:opacity-100 focus-visible:opacity-100 transition-opacity cursor-pointer"
                title={$t("artists_view.play")} aria-label={`${$t("artists_view.play")} ${composer.name}`}
                onclick={(e) => withoutNavigating(e, () => handleComposerPlay(libraryId, composer.name))}>
            <span class="w-8 h-8 rounded-full flex items-center justify-center bg-(--rg-g) text-(--rg-on-g)">
                <Icon icon="material-symbols:play-arrow-rounded" width="20" />
            </span>
        </button>
    </div>

    <p class="min-w-0 truncate text-base font-semibold text-(--rg-tx)">{composer.name}</p>

    <span class={chiffre}><b class="font-medium text-(--rg-tx2)">{composer.total_albums}</b> {$t(composer.total_albums === 1 ? "library_head.albums_one" : "library_head.albums_n")}</span>
    <span class="{chiffre} max-[900px]:hidden"><b class="font-medium text-(--rg-tx2)">{composer.total_tracks}</b> {$t(composer.total_tracks === 1 ? "library_head.tracks_one" : "library_head.tracks_n")}</span>
    <span class="{chiffre} max-[900px]:hidden">{composer.total_duration ? listenDuration(composer.total_duration) : ""}</span>

    <div class="flex items-center justify-end gap-0.5">
        <button type="button" class="{act} opacity-0 group-hover:opacity-100 focus-visible:opacity-100 max-[900px]:hidden" title={$t("artists_view.enqueue")} aria-label={$t("artists_view.enqueue")}
                onclick={(e) => withoutNavigating(e, () => handleComposerAddToQueue(libraryId, composer.name))}>
            <Icon icon="material-symbols:queue-music-rounded" width="19" />
        </button>
        <span class="w-8.5 h-8.5 flex items-center justify-center text-(--rg-mu2) group-hover:text-(--rg-tx2)" title={$t("artists_view.open")}>
            <Icon icon="material-symbols:chevron-right-rounded" width="19" />
        </span>
    </div>
</a>
