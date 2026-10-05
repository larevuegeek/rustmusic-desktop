<script lang="ts">
import Icon from "@iconify/svelte";
import { player } from "#lib/stores/player/player.store";
import { queueState } from "#lib/stores/queue/queueState.store";
import { lyricsPanelOpened, closeLyricsPanel } from "#lib/stores/lyrics/lyricsPanel.store";
import { getLyrics, refreshLyrics, type Lyrics } from "#lib/services/lyrics/lyrics.service";
import { parseLrc, findActiveLineIndex, type LrcLine } from "#lib/helper/lyrics/lrcParser";
import { playerService } from "#lib/services/player/player.service";
import { resolveCoverSrc } from "#lib/helper/tools/coverHelper";
import PochetteFloue from "#lib/components/ui/image/PochetteFloue.svelte";
import NowPlayingCard from "#lib/components/player/NowPlayingCard.svelte";
import { t } from "#lib/i18n";

// Libellé de la source, comme le mini-lecteur.
const SOURCES: Record<string, string> = { sidecar: "mini.src_sidecar", lrclib: "mini.src_lrclib", manual: "mini.src_manual" };

type Status = "idle" | "loading" | "ready" | "empty" | "error";

let status = $state<Status>("idle");
let lyrics = $state<Lyrics | null>(null);
let error = $state<string | null>(null);
let isRefreshing = $state(false);

let pathFile = $derived($player?.pathFile ?? null);
let currentMs = $derived(($player?.jsPosition ?? 0) * 1000);

let syncedLines = $state<LrcLine[]>([]);

let scrollContainer = $state<HTMLDivElement | null>(null);
let lineRefs: Record<number, HTMLElement> = {};
let lastScrolledIndex = -1;

let activeIndex = $derived(
    syncedLines.length > 0 ? findActiveLineIndex(syncedLines, currentMs) : -1
);

// ─── Fix carré noir WebKitGTK/AMD ───────────────────────────────
// Le panneau est `fixed top-0 right-0` et reste monté (simplement translaté
// hors écran via `translate-x-full`) quand il est fermé. Sur WebKitGTK ≥ 2.44
// + GPU AMD/Mesa, sa couche `backdrop-filter` off-screen est peinte comme un
// rectangle NOIR dans le coin haut-droit du viewport (couche de compositing
// mal placée). On retire donc le panneau du rendu (`display:none`) quand il
// est fermé — mais après l'animation de fermeture (400ms) pour préserver le
// slide-out.
// `panelRendered` pilote `display` (montage/démontage visuel), `slideIn` pilote
// le `translate-x`. On les découple : à l'ouverture on monte d'abord le panneau
// hors écran (translate-x-full), puis — au frame suivant — on déclenche le
// slide-in ; à la fermeture on slide out immédiatement puis on démonte après
// l'animation. Sinon `display:none → flex` casse la transition d'entrée.
let panelRendered = $state(false);
let slideIn = $state(false);
let closeTimer: ReturnType<typeof setTimeout> | undefined;
let rafId = 0;
$effect(() => {
    if ($lyricsPanelOpened) {
        clearTimeout(closeTimer);
        panelRendered = true;
        cancelAnimationFrame(rafId);
        rafId = requestAnimationFrame(() =>
            rafId = requestAnimationFrame(() => { slideIn = true; })
        );
    } else {
        slideIn = false;
        closeTimer = setTimeout(() => { panelRendered = false; }, 450);
    }
    return () => { clearTimeout(closeTimer); cancelAnimationFrame(rafId); };
});

// ─── Cover blurrée pour le background ───
// On prend la cover en cascade :
// 1. Tag embarqué du fichier (data URI, toujours dispo si le fichier a une pochette)
// 2. Cover de la queue (URL asset:// si miniature sauvegardée)
// 3. Fallback gradient
let currentQueueTrack = $derived($queueState.tracks[$queueState.currentIndex] ?? null);
let embeddedCover = $derived($player?.audioFile?.tags?.attached_images?.[0]?.image_src ?? null);
let bgCoverSrc = $state<string | null>(null);
// Pochette du disque : fond pré-flouté par le backend (voir PochetteFloue).
let bgCoverChemin = $state<string | null>(null);

// Couleur de gradient de fallback dérivée du chemin (stable par track)
let fallbackGradient = $derived.by(() => {
    if (!pathFile) return "from-emerald-900/40 via-violet-900/30 to-blue-900/40";
    const hash = Array.from(pathFile).reduce((acc, c) => acc + c.charCodeAt(0), 0);
    const palettes = [
        "from-emerald-700/40 via-teal-900/30 to-cyan-900/40",
        "from-violet-700/40 via-purple-900/30 to-fuchsia-900/40",
        "from-rose-700/40 via-pink-900/30 to-red-900/40",
        "from-amber-700/40 via-orange-900/30 to-red-900/40",
        "from-blue-700/40 via-indigo-900/30 to-purple-900/40",
        "from-emerald-700/40 via-green-900/30 to-lime-900/40",
    ];
    return palettes[hash % palettes.length];
});

$effect(() => {
    const queueCover = currentQueueTrack?.cover;
    const embedded = embeddedCover;

    bgCoverChemin = null;

    // Priorité 1 : cover de la queue, fichier sur disque → fond pré-flouté
    if (queueCover && queueCover !== '/images/no-cd.png' && !queueCover.startsWith('data:') && !queueCover.startsWith('http')) {
        bgCoverChemin = queueCover;
        bgCoverSrc = null;
        return;
    }

    // Pochette distante : flou CSS, faute de fichier à pré-flouter
    if (queueCover && queueCover.startsWith('http')) {
        let vivant = true;
        resolveCoverSrc(queueCover, "2x").then((url) => {
            if (vivant) bgCoverSrc = url;
        });
        return () => { vivant = false; };
    }

    // Priorité 2 : cover embarquée dans le fichier audio (data URI)
    if (embedded && embedded !== '/images/no-cd.png') {
        bgCoverSrc = embedded;
        return;
    }

    // Sinon → fallback gradient
    bgCoverSrc = null;
});

// ─── Pré-fetch silencieux au changement de track ───
$effect(() => {
    if (!pathFile) return;
    getLyrics(pathFile).catch(() => { /* silencieux */ });
});

// ─── Fetch visible quand on ouvre le panel ou que le track change ───
$effect(() => {
    if (!$lyricsPanelOpened) return;
    if (!pathFile) {
        status = "idle";
        lyrics = null;
        syncedLines = [];
        return;
    }
    fetchLyrics(pathFile);
});

async function fetchLyrics(path: string) {
    status = "loading";
    error = null;
    lyrics = null;
    syncedLines = [];
    lastScrolledIndex = -1;
    try {
        const result = await getLyrics(path);
        // Morceau changé entre-temps : ces paroles ne sont plus les bonnes.
        if (path !== pathFile) return;
        applyLyrics(result);
    } catch (e) {
        if (path !== pathFile) return;
        status = "error";
        error = String(e ?? $t("lyrics.unknown_error"));
    }
}

async function handleRefresh() {
    if (!pathFile || isRefreshing) return;
    isRefreshing = true;
    error = null;
    const path = pathFile;
    try {
        const result = await refreshLyrics(path);
        if (path !== pathFile) return;
        applyLyrics(result);
    } catch (e) {
        status = "error";
        error = String(e ?? $t("lyrics.unknown_error"));
    } finally {
        isRefreshing = false;
    }
}

function applyLyrics(result: Lyrics | null) {
    lyrics = result;
    if (!result || result.source === "none" || (!result.plain && !result.synced)) {
        status = "empty";
        return;
    }
    if (result.synced) {
        syncedLines = parseLrc(result.synced);
    }
    status = "ready";
}

// ─── Auto-scroll : ne dépend que de activeIndex ───
$effect(() => {
    const idx = activeIndex;
    if (status !== "ready" || syncedLines.length === 0 || idx < 0) return;
    if (!scrollContainer) return;

    const el = lineRefs[idx];
    if (!el) {
        lastScrolledIndex = idx;
        return;
    }

    const isSeek = Math.abs(idx - lastScrolledIndex) > 3 || lastScrolledIndex === -1;
    lastScrolledIndex = idx;

    el.scrollIntoView({
        behavior: isSeek ? "instant" : "smooth",
        block: "center",
    });
});

function handleSeekToLine(timeMs: number) {
    playerService.seekTo(timeMs / 1000);
}
</script>

<!-- Backdrop -->
<button
    type="button"
    aria-label={$t("lyrics.close")}
    class="fixed inset-0 z-40 bg-black/30 dark:bg-black/50 backdrop-blur-md
           transition-all duration-400 ease-[cubic-bezier(0.16,1,0.3,1)]"
    class:opacity-100={$lyricsPanelOpened}
    class:opacity-0={!$lyricsPanelOpened}
    class:pointer-events-auto={$lyricsPanelOpened}
    class:pointer-events-none={!$lyricsPanelOpened}
    onclick={closeLyricsPanel}
></button>

<!-- Panel -->
<aside
    class="fixed right-0 top-0 z-50 h-dvh w-full sm:w-lg lg:w-xl 2xl:w-2xl
           overflow-hidden flex flex-col
           shadow-[-12px_0_60px_rgba(0,0,0,0.5)]
           transform transition-transform duration-400 ease-[cubic-bezier(0.16,1,0.3,1)]"
    class:translate-x-0={slideIn}
    class:translate-x-full={!slideIn}
    class:hidden={!panelRendered}
    aria-hidden={!$lyricsPanelOpened}
>
    <!-- Background : cover floutée OU gradient de fallback -->
    <div class="absolute inset-0 -z-10 overflow-hidden">
        {#if bgCoverChemin}
            <!-- Pré-floutée : un filtre CSS se recalculait à chaque ligne de paroles. -->
            <PochetteFloue path={bgCoverChemin} saturation={1.5} flou={6} rayon={60}
                class="absolute -inset-10 w-[calc(100%+80px)] h-[calc(100%+80px)] object-cover scale-110 opacity-90" />
        {:else if bgCoverSrc}
            <img
                src={bgCoverSrc}
                alt=""
                class="absolute -inset-10 w-[calc(100%+80px)] h-[calc(100%+80px)] object-cover blur-[60px] saturate-[1.5] scale-110 opacity-90"
            />
        {:else}
            <div class="absolute inset-0 bg-linear-to-br {fallbackGradient}"></div>
            <div class="absolute -top-20 -left-20 w-96 h-96 rounded-full bg-white/10 blur-3xl"></div>
            <div class="absolute -bottom-20 -right-20 w-96 h-96 rounded-full bg-black/30 blur-3xl"></div>
        {/if}
    </div>

    <!-- Voile sombre uniforme pour la lisibilité -->
    <div class="absolute inset-0 bg-black/55 backdrop-blur-xl pointer-events-none"></div>

    <!-- Léger gradient haut/bas pour les zones header/footer -->
    <div class="absolute inset-0 bg-linear-to-b from-black/30 via-transparent to-black/40 pointer-events-none"></div>

    <!-- Header -->
    <div class="relative z-20 flex items-center justify-between px-6 py-4
                border-b border-white/8">
        <div class="flex items-center gap-3 min-w-0">
            <div class="w-9 h-9 rounded-lg bg-white/10 backdrop-blur-md flex items-center justify-center shrink-0">
                <Icon icon="lucide:mic-vocal" width={17} class="text-white" />
            </div>
            <div class="min-w-0">
                <div class="text-sm font-semibold text-white">{$t("player.lyrics")}</div>
                {#if status === "ready" && lyrics}
                    <div class="flex items-center gap-1.5 text-[10px] text-white/60 mt-0.5">
                        {#if syncedLines.length > 0}
                            <span class="px-1.5 py-px rounded bg-emerald-500/20 text-emerald-300 font-medium">
                                {$t("lyrics.synced")}
                            </span>
                        {/if}
                        <span>{SOURCES[lyrics.source] ? $t(SOURCES[lyrics.source]) : lyrics.source}</span>
                    </div>
                {/if}
            </div>
        </div>

        <div class="flex items-center gap-1 shrink-0">
            {#if status === "ready" || status === "empty"}
                <button
                    type="button"
                    class="w-8 h-8 rounded-md flex items-center justify-center cursor-pointer
                           text-white/60 hover:text-white hover:bg-white/10 transition-colors"
                    onclick={handleRefresh}
                    disabled={isRefreshing}
                    aria-label={$t("lyrics.search_again")}
                    title={$t("lyrics.search_again")}
                >
                    <Icon
                        icon="lucide:refresh-cw"
                        width={14}
                        class={isRefreshing ? "animate-spin" : ""}
                    />
                </button>
            {/if}
            <button
                type="button"
                class="w-8 h-8 rounded-md flex items-center justify-center cursor-pointer
                       text-white/60 hover:text-white hover:bg-white/10 transition-colors"
                onclick={closeLyricsPanel}
                aria-label={$t("common.close")}
            >
                <Icon icon="lucide:x" width={16} />
            </button>
        </div>
    </div>

    <!-- Now playing card (même visuel que QueuePanel, variante blur pour fond sombre) -->
    {#if pathFile}
        <div class="relative z-10 px-6 pt-4 pb-2">
            <NowPlayingCard variant="blur" />
        </div>
    {/if}

    <!-- Content -->
    <div
        class="relative z-10 flex-1 overflow-y-auto scrollbar-app px-6"
        bind:this={scrollContainer}
    >
        {#if !pathFile || status === "idle"}
            <div class="flex flex-col items-center justify-center h-full text-center">
                <Icon icon="lucide:music" width={36} class="text-white/30 mb-3" />
                <p class="text-sm text-white/60">{$t("lyrics.no_track")}</p>
            </div>

        {:else if status === "loading"}
            <div class="flex flex-col items-center justify-center h-full text-center">
                <div class="relative mb-4">
                    <div class="w-12 h-12 rounded-full bg-white/10 backdrop-blur-md flex items-center justify-center">
                        <Icon icon="lucide:loader-2" width={20} class="animate-spin text-white" />
                    </div>
                    <div class="absolute inset-0 rounded-full bg-emerald-500/30 blur-xl animate-pulse"></div>
                </div>
                <p class="text-sm text-white/80 font-medium mb-1">{$t("lyrics.searching")}</p>
                <p class="text-xs text-white/40">{$t("lyrics.searching_desc")}</p>
            </div>

        {:else if status === "error"}
            <div class="flex flex-col items-center justify-center h-full text-center px-4">
                <div class="w-12 h-12 rounded-full bg-red-500/20 flex items-center justify-center mb-3">
                    <Icon icon="lucide:wifi-off" width={20} class="text-red-300" />
                </div>
                <p class="text-sm font-medium text-white mb-1">{$t("lyrics.offline")}</p>
                <p class="text-xs text-white/50 mb-5">{error}</p>
                <button
                    class="text-xs px-4 py-2 rounded-full cursor-pointer
                           bg-white/15 backdrop-blur-md text-white hover:bg-white/25
                           border border-white/15 transition-colors"
                    onclick={() => pathFile && fetchLyrics(pathFile)}
                >
                    {$t("common.retry")}
                </button>
            </div>

        {:else if status === "empty"}
            <div class="flex flex-col items-center justify-center h-full text-center px-4">
                <div class="w-12 h-12 rounded-full bg-white/10 flex items-center justify-center mb-3">
                    <Icon icon="lucide:file-x" width={20} class="text-white/60" />
                </div>
                <p class="text-sm font-medium text-white mb-1">{$t("lyrics.empty")}</p>
                <p class="text-xs text-white/50 mb-5 max-w-65">
                    {$t("lyrics.empty_desc")}
                </p>
                <button
                    class="text-xs px-4 py-2 rounded-full cursor-pointer
                           bg-white/15 backdrop-blur-md text-white hover:bg-white/25
                           border border-white/15 transition-colors"
                    onclick={handleRefresh}
                >
                    {$t("lyrics.search_again")}
                </button>
            </div>

        {:else if syncedLines.length > 0}
            <!-- Mode karaoké synchronisé Apple Music style -->
            <div class="flex flex-col gap-1">
                <!-- Petit padding haut, juste pour respirer -->
                <div class="h-12"></div>
                {#each syncedLines as line, i (i)}
                    <button
                        type="button"
                        bind:this={lineRefs[i]}
                        onclick={() => handleSeekToLine(line.timeMs)}
                        class:lyric-active={i === activeIndex}
                        class:lyric-past={i < activeIndex}
                        class:lyric-future={i > activeIndex}
                        class="lyric-line block w-full text-left cursor-pointer px-2 py-3 rounded-lg
                               text-5xl leading-[1.1] font-black tracking-tight
                               transition-all duration-700 ease-out
                               hover:bg-white/5"
                    >
                        {line.text || "♪"}
                    </button>
                {/each}
                <!-- Padding bas -->
                <div class="h-[55vh]"></div>
            </div>

        {:else if lyrics?.plain}
            <!-- Mode statique non-synchronisé -->
            <div class="text-xl leading-relaxed font-semibold text-white/85
                        whitespace-pre-line py-12 tracking-tight">
                {lyrics.plain}
            </div>
        {/if}
    </div>
</aside>
