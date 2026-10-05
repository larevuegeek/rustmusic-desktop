<script lang="ts">
import Icon from "@iconify/svelte";
import { player } from "#lib/stores/player/player.store";
import { queueState } from "#lib/stores/queue/queueState.store";
import { formatTime } from "#lib/helper/tools/dateTools";
import CoverImg from "#lib/components/ui/image/CoverImg.svelte";
import { t } from "#lib/i18n";

type Variant = "default" | "blur";

let { variant = "default" }: { variant?: Variant } = $props();

let status = $derived($player?.status ?? "idle");
let currentIndex = $derived($queueState.currentIndex);
let currentTrack = $derived($queueState.tracks[currentIndex] ?? null);

// Variantes de style :
//  - "default" → fond blanc/dark neutre (QueuePanel)
//  - "blur"    → semi-transparent sur fond sombre flouté (LyricsPanel)
let cardClasses = $derived(
    variant === "blur"
        ? "bg-white/8 backdrop-blur-md border-white/12 shadow-[0_2px_12px_rgba(0,0,0,0.3)]"
        : "bg-white dark:bg-white/4 border-black/6 dark:border-white/6 shadow-[0_1px_3px_rgba(0,0,0,0.04),0_4px_16px_rgba(0,0,0,0.03)] dark:shadow-[0_2px_12px_rgba(0,0,0,0.3)]"
);

let titleClasses = $derived(
    variant === "blur"
        ? "text-white"
        : "text-neutral-900 dark:text-white"
);

let artistClasses = $derived(
    variant === "blur"
        ? "text-white/70"
        : "text-neutral-500 dark:text-neutral-400"
);

let durationClasses = $derived(
    variant === "blur"
        ? "text-white/60"
        : "text-neutral-400 dark:text-neutral-500"
);

let pauseIconClasses = $derived(
    variant === "blur"
        ? "text-white/60"
        : "text-neutral-400 dark:text-neutral-500"
);

let coverShadow = $derived(
    variant === "blur"
        ? "shadow-[0_2px_12px_rgba(0,0,0,0.4)] ring-1 ring-white/10"
        : "shadow-[0_2px_8px_rgba(0,0,0,0.1)] dark:shadow-[0_2px_12px_rgba(0,0,0,0.4)] ring-1 ring-black/4 dark:ring-white/6"
);

let glowClasses = $derived(
    variant === "blur"
        ? "from-emerald-400/15 via-emerald-400/5 to-transparent"
        : "from-emerald-500/8 via-emerald-500/4 to-transparent"
);
</script>

{#if currentTrack}
    <div>
        <!-- Section label -->
        <div class="mb-3 flex items-center gap-2.5">
            <div class="relative flex h-2.5 w-2.5 items-center justify-center">
                <span
                    class="absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-40
                           {status === 'playing' ? 'animate-ping' : ''}"
                ></span>
                <span
                    class="relative inline-flex h-2 w-2 rounded-full bg-emerald-500
                           shadow-[0_0_6px_rgba(16,185,129,0.6)]"
                ></span>
            </div>
            <h3 class="text-[11px] font-bold uppercase tracking-[0.15em] text-emerald-600 dark:text-emerald-400">
                {$t("home.now_playing")}
            </h3>
        </div>

        <!-- Current track card -->
        <div
            class="group relative flex items-center gap-4 rounded-2xl p-3.5 border
                   transition-all duration-400 hover:shadow-[0_2px_8px_rgba(0,0,0,0.06),0_8px_24px_rgba(0,0,0,0.06)]
                   dark:hover:shadow-[0_4px_20px_rgba(0,0,0,0.4)]
                   {cardClasses}"
        >
            <!-- Glow effect behind card on hover -->
            <div
                class="absolute -inset-1 rounded-2xl opacity-0 group-hover:opacity-100
                       bg-linear-to-r {glowClasses}
                       blur-xl transition-opacity duration-500 pointer-events-none"
            ></div>

            <!-- Cover art -->
            <div
                class="relative h-16 w-16 shrink-0 overflow-hidden rounded-md
                       bg-neutral-100 dark:bg-neutral-800 {coverShadow}"
            >
                <CoverImg
                    path={currentTrack.cover}
                    alt={currentTrack.title}
                    class="h-full w-full object-cover transition-transform duration-700 ease-out group-hover:scale-105"
                />
                <div
                    class="absolute inset-0 rounded-xl shadow-[inset_0_0_12px_rgba(0,0,0,0.08)]
                           dark:shadow-[inset_0_0_12px_rgba(0,0,0,0.2)]"
                ></div>
            </div>

            <!-- Track info -->
            <div class="relative min-w-0 flex-1 flex flex-col justify-center gap-0.5">
                <span
                    class="truncate text-[14px] font-bold leading-tight {titleClasses}"
                    title={currentTrack.title}
                >
                    {currentTrack.title}
                </span>
                <span class="truncate text-[12px] font-medium {artistClasses}">
                    {currentTrack.artist ?? "Artiste inconnu"}
                </span>
            </div>

            <!-- Right side: visualizer + duration -->
            <div class="shrink-0 flex flex-col items-end justify-center gap-2.5 pr-1">
                {#if status === "playing"}
                    <div class="flex items-end gap-0.75 h-5">
                        <span class="w-0.75 rounded-full bg-emerald-500 animate-[eq-bar-1_0.8s_ease-in-out_infinite]"></span>
                        <span class="w-0.75 rounded-full bg-emerald-500 animate-[eq-bar-2_0.6s_ease-in-out_infinite_0.1s]"></span>
                        <span class="w-0.75 rounded-full bg-emerald-500 animate-[eq-bar-3_0.7s_ease-in-out_infinite_0.2s]"></span>
                        <span class="w-0.75 rounded-full bg-emerald-500/60 animate-[eq-bar-1_0.9s_ease-in-out_infinite_0.15s]"></span>
                    </div>
                {:else if status === "paused"}
                    <Icon icon="mynaui:pause" width="18" height="18" class={pauseIconClasses} />
                {/if}
                <span class="text-[11px] font-semibold tabular-nums {durationClasses}">
                    {formatTime(currentTrack.duration ?? 0)}
                </span>
            </div>
        </div>
    </div>
{/if}
