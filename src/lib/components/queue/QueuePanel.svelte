<script lang="ts">
import Icon from "@iconify/svelte";
import { lecture } from "#lib/stores/player/lecture.store";
import { queueState } from "#lib/stores/queue/queueState.store";
import { queuePanelOpened, closeQueuePanel } from "#lib/stores/queue/queueUi.store";
import { formatTime } from "#lib/helper/tools/dateTools";
import CoverImg from "#lib/components/ui/image/CoverImg.svelte";
import { playerService } from "#lib/services/player/player.service";
import NowPlayingCard from "#lib/components/player/NowPlayingCard.svelte";
import { t, currentLocale } from "#lib/i18n";

function onKeyDown(e: KeyboardEvent) {
    if (e.key === "Escape") closeQueuePanel();
}

type PlayerStatus = "ready" | "playing" | "paused" | "ended" | "idle";

let status = $derived($lecture.status);

// `label` : clé de traduction, résolue à l'affichage.
const statusMap: Record<PlayerStatus, { label: string; class: string }> = {
  playing: {
    label: "queue_panel.status_playing",
    class: "bg-emerald-500/15 text-emerald-600 dark:text-emerald-400"
  },
  paused: {
    label: "home.paused",
    class: "bg-neutral-500/10 text-neutral-500 dark:text-neutral-400"
  },
  ended: {
    label: "queue_panel.status_ended",
    class: "bg-neutral-500/10 text-neutral-500 dark:text-neutral-400"
  },
  idle: {
    label: "player.inactive",
    class: "bg-neutral-500/10 text-neutral-500 dark:text-neutral-400"
  },
  ready: {
    label: "queue_panel.status_ready",
    class: "bg-neutral-500/10 text-neutral-500 dark:text-neutral-400"
  }
};

let statusConfig = $derived(statusMap[status]);
let statusLabel = $derived(statusConfig.label);
let statusClass = $derived(statusConfig.class);

let queueTrackList = $derived($queueState.tracks);
let currentIndex = $derived($queueState.currentIndex);
let currentTrack = $derived(queueTrackList[currentIndex] ?? null);

let upcomingTracks = $derived(
    currentIndex > -1 ? queueTrackList.slice(currentIndex + 1) : queueTrackList
);

let totalTracks = $derived(queueTrackList.length);

// Contenu rendu seulement panneau ouvert (et le temps de la fermeture) : la carte en cours suit la position.
let monte = $state(false);
$effect(() => {
  if ($queuePanelOpened) { monte = true; return; }
  const id = setTimeout(() => { monte = false; affiches = LOT; }, 450);
  return () => clearTimeout(id);
});

// Une file de milliers de titres : on en montre un lot, la suite à la demande.
const LOT = 150;
let affiches = $state(LOT);
const visibles = $derived(upcomingTracks.slice(0, affiches));

// --- Drag & Drop (pointer events) ---
let dragFrom = $state<number | null>(null);
let dragOver = $state<number | null>(null);
let isDragging = $derived(dragFrom !== null);

function toReal(localIdx: number) {
  return (currentIndex > -1 ? currentIndex + 1 : 0) + localIdx;
}

function handleGrab(e: PointerEvent, i: number) {
  if (e.button !== 0) return;
  e.preventDefault();
  dragFrom = i;
  (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
}

function handleMove(e: PointerEvent) {
  if (dragFrom === null) return;
  const el = document.elementFromPoint(e.clientX, e.clientY);
  if (!el) return;
  const row = el.closest('[data-qi]') as HTMLElement | null;
  if (row) {
    const idx = Number(row.dataset.qi);
    if (!isNaN(idx)) dragOver = idx;
  }
}

function handleRelease() {
  if (dragFrom !== null && dragOver !== null && dragFrom !== dragOver) {
    queueState.reorderTracks(toReal(dragFrom), toReal(dragOver));
  }
  dragFrom = null;
  dragOver = null;
}
</script>

<svelte:window on:keydown={onKeyDown} />

<!-- ==================== BACKDROP ==================== -->
<button
  type="button"
  aria-label={$t("queue_panel.close")}
  class="fixed inset-0 z-40
         bg-black/30 dark:bg-black/50
         backdrop-blur-md
         transition-all duration-400 ease-[cubic-bezier(0.16,1,0.3,1)]"
  class:opacity-100={$queuePanelOpened}
  class:opacity-0={!$queuePanelOpened}
  class:pointer-events-auto={$queuePanelOpened}
  class:pointer-events-none={!$queuePanelOpened}
  onclick={closeQueuePanel}
></button>

<!-- ==================== PANEL ==================== -->
<aside
  class="
    fixed right-0 top-0 z-50 h-dvh
    w-full sm:w-110
    bg-white dark:bg-[#0c0c0e]
    border-l border-black/4 dark:border-white/6
    shadow-[-8px_0_40px_rgba(0,0,0,0.08)] dark:shadow-[-8px_0_60px_rgba(0,0,0,0.5)]
    transform transition-transform duration-400 ease-[cubic-bezier(0.16,1,0.3,1)]
    overflow-hidden
  "
  class:translate-x-0={$queuePanelOpened}
  class:translate-x-full={!$queuePanelOpened}
  aria-hidden={!$queuePanelOpened}
  inert={!$queuePanelOpened}
>

  <!-- Ambient glow (visible only when playing) -->
  {#if status === "playing" && currentTrack}
    <div class="absolute top-0 left-0 right-0 h-72 pointer-events-none opacity-30 dark:opacity-20
                bg-linear-to-b from-emerald-500/20 via-emerald-500/5 to-transparent
                transition-opacity duration-1000"></div>
  {/if}

  <!-- ==================== HEADER ==================== -->
  <div class="relative top-0 z-20 flex items-center justify-between px-6 py-5
              bg-white/80 dark:bg-[#0c0c0e]/80 backdrop-blur-xl
              border-b border-black/4 dark:border-white/6">

    <!-- Shimmer accent line -->
    {#if monte}
      <div class="absolute bottom-0 left-0 right-0 h-px
                  bg-linear-to-r from-transparent via-emerald-500/30 to-transparent
                  animate-[shimmer_4s_ease-in-out_infinite]"
           style="background-size: 200% 100%;"></div>
    {/if}

    <div class="flex flex-col gap-1">
      <h2 class="text-[15px] font-bold tracking-tight text-neutral-900 dark:text-white">
        {$t("queue_panel.title")}
      </h2>
      <div class="flex items-center gap-2">
        <span class="text-[11px] font-semibold text-neutral-400 dark:text-neutral-500 tabular-nums">
          {$t(totalTracks > 1 ? "queue_panel.tracks_n" : "queue_panel.tracks_one").replace("{n}", totalTracks.toLocaleString($currentLocale))}
        </span>
        <!-- Status pill -->
        <span class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-bold uppercase tracking-wider {statusClass}">
          {#if status === "playing"}
            <span class="relative flex h-1.5 w-1.5">
              <span class="absolute inline-flex h-full w-full animate-ping rounded-full bg-emerald-400 opacity-60"></span>
              <span class="relative inline-flex h-1.5 w-1.5 rounded-full bg-emerald-500"></span>
            </span>
          {/if}
          {$t(statusLabel)}
        </span>
      </div>
    </div>

    <div class="flex items-center gap-1.5">
      {#if currentTrack}
        <button
          type="button"
          onclick={() => playerService.handleTogglePlay()}
          class="group flex items-center justify-center h-9 w-9
                 rounded-xl
                 bg-emerald-500/10 dark:bg-emerald-500/10
                 hover:bg-emerald-500/20 dark:hover:bg-emerald-500/20
                 border border-emerald-500/20 hover:border-emerald-500/30
                 transition-all duration-200 cursor-pointer"
          aria-label={status === 'playing' ? $t("home.pause_short") : $t("home.play")}
          title={status === 'playing' ? $t("home.pause_short") : $t("home.play")}
        >
          <Icon icon={status === 'playing' ? 'lucide:pause' : 'lucide:play'}
            width="15" height="15"
            class="text-emerald-500 {status !== 'playing' ? 'ml-0.5' : ''}" />
        </button>
      {/if}
      {#if queueTrackList.length > 0}
        <button
          type="button"
          onclick={() => queueState.clear()}
          class="group flex items-center justify-center h-9 w-9
                 rounded-xl
                 bg-neutral-100 dark:bg-white/5
                 hover:bg-red-50 dark:hover:bg-red-500/15
                 border border-transparent hover:border-red-200/50 dark:hover:border-red-500/20
                 transition-all duration-200"
          aria-label={$t("queue_panel.clear")}
          title={$t("queue_panel.clear")}
        >
          <Icon icon="mynaui:trash" width="16" height="16"
            class="text-neutral-400 group-hover:text-red-500 dark:group-hover:text-red-400
                   transition-colors duration-200 cursor-pointer" />
        </button>
      {/if}
      <button
        onclick={closeQueuePanel}
        aria-label={$t("common.close")}
        title={$t("common.close")}
        class="group flex items-center justify-center h-9 w-9
               rounded-xl
               bg-neutral-100 dark:bg-white/5
               hover:bg-neutral-200 dark:hover:bg-white/10
               border border-transparent hover:border-neutral-200/50 dark:hover:border-white/8
               transition-all duration-200"
      >
        <Icon icon="mynaui:x" width="18" height="18"
          class="text-neutral-500 dark:text-neutral-400
                 group-hover:text-neutral-700 dark:group-hover:text-neutral-200
                 group-hover:rotate-90
                 transition-all duration-300 ease-[cubic-bezier(0.34,1.56,0.64,1)]" />
      </button>
    </div>
  </div>

  <!-- ==================== CONTENT ==================== -->
  <div class="scrollbar-app relative w-full h-[calc(100dvh-85px)] overflow-y-auto px-4 pb-16">

    <!-- ===== CURRENT TRACK ===== -->
    {#if monte && currentTrack}
      <div class="mt-6 mb-8 px-2">
        <NowPlayingCard variant="default" />
      </div>
    {/if}

    <!-- ===== UPCOMING TRACKS ===== -->
    {#if monte && upcomingTracks.length > 0}
      <div class="sticky top-0 z-10 mb-1
                  flex items-center justify-between
                  bg-white/90 dark:bg-[#0c0c0e]/90 backdrop-blur-xl
                  px-2 py-3">
        <div class="flex items-center gap-2.5">
          <div class="flex items-center justify-center h-6 w-6 rounded-lg
                      bg-neutral-100 dark:bg-white/5">
            <Icon icon="mynaui:list" width="14" height="14" class="text-neutral-400 dark:text-neutral-500" />
          </div>
          <h3 class="text-[11px] font-bold uppercase tracking-[0.15em] text-neutral-500 dark:text-neutral-400">
            {$t("queue_panel.up_next")}
          </h3>
        </div>
        <span class="text-[11px] font-semibold tabular-nums text-neutral-300 dark:text-neutral-600">
          {$t(upcomingTracks.length > 1 ? "queue_panel.upcoming_n" : "queue_panel.upcoming_one").replace("{n}", upcomingTracks.length.toLocaleString($currentLocale))}
        </span>
      </div>

      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        class="flex flex-col w-full gap-0.5"
        onpointermove={handleMove}
        onpointerup={handleRelease}
      >
        {#each visibles as queueTrack, i (queueTrack.queueId)}
          <div
            data-qi={i}
            class="group relative flex items-center gap-3
                   rounded-xl p-2.5
                   transition-all duration-300 ease-out
                   hover:bg-black/3 dark:hover:bg-white/4
                   {dragFrom === i
                     ? 'opacity-20 scale-[0.97] blur-[1px]' : ''}
                   {isDragging && dragOver === i && dragFrom !== i
                     ? 'translate-y-1 bg-emerald-500/5!' : ''}
                   {isDragging && dragFrom !== i && dragOver !== i
                     ? 'opacity-60' : ''}"
            style="animation: fadeSlideIn 0.3s ease-out {Math.min(i * 0.04, 0.4)}s both;"
          >

            <!-- Drop indicator line -->
            {#if isDragging && dragOver === i && dragFrom !== i}
              <div class="absolute -top-px left-3 right-3 h-0.5 rounded-full
                          bg-emerald-500 shadow-[0_0_8px_rgba(16,185,129,0.6)]"></div>
            {/if}

            <!-- Drag handle -->
            <div
              class="absolute left-0 z-10 flex h-full w-7 cursor-grab items-center justify-center
                     opacity-0 group-hover:opacity-100
                     transition-all duration-200
                     {isDragging ? 'opacity-100!' : ''}
                     active:cursor-grabbing touch-none"
              onpointerdown={(e) => handleGrab(e, i)}
            >
                <Icon icon="radix-icons:drag-handle-dots-2" width="14" height="14"
                  class="pointer-events-none transition-colors duration-200
                       text-neutral-300 dark:text-neutral-600
                       {isDragging && dragFrom === i ? 'text-emerald-400! scale-110' : ''}" />
            </div>

            <!-- Track number (fades to nothing on hover / drag) -->
            <span class="w-5 text-center text-[11px] font-semibold tabular-nums
                         text-neutral-300 dark:text-neutral-600
                         transition-opacity duration-200 group-hover:opacity-0
                         {isDragging ? 'opacity-0' : ''} ml-2">
              {i + 1}
            </span>

            <!-- Cover -->
            <div class="h-11 w-11 shrink-0 overflow-hidden rounded-sm
                        bg-neutral-100 dark:bg-neutral-800
                        ring-1 ring-black/4 dark:ring-white/6
                        transition-shadow duration-200
                        group-hover:shadow-[0_2px_8px_rgba(0,0,0,0.08)]
                        dark:group-hover:shadow-[0_2px_8px_rgba(0,0,0,0.3)]">
              <CoverImg path={queueTrack.cover} alt={queueTrack.title}
                   class="h-full w-full object-cover
                          transition-transform duration-500 ease-out group-hover:scale-[1.06]" />
            </div>

            <!-- Info -->
            <div class="min-w-0 flex-1 flex flex-col justify-center gap-0.5">
              <span class="truncate text-[13px] font-semibold
                           text-neutral-800 dark:text-neutral-200
                           group-hover:text-neutral-900 dark:group-hover:text-white
                           transition-colors duration-200"
                    title={queueTrack.title}>
                {queueTrack.title}
              </span>
              <span class="truncate text-[11px] font-medium text-neutral-400 dark:text-neutral-500">
                {queueTrack.artist ?? $t("common.unknown_artist")}
              </span>
            </div>

            <!-- Actions -->
            <div class="flex shrink-0 items-center gap-2">
              <span class="w-10 text-right text-[11px] font-semibold tabular-nums
                           text-neutral-300 dark:text-neutral-600
                           transition-opacity duration-200 group-hover:opacity-0">
                {formatTime(queueTrack.duration ?? 0)}
              </span>
              <button
                type="button"
                onclick={() => queueState.removeTrack(queueTrack.queueId)}
                class="flex h-7 w-7 items-center justify-center
                       rounded-lg
                       opacity-0 group-hover:opacity-100
                       bg-neutral-100 dark:bg-white/6
                       hover:bg-red-50 hover:text-red-500
                       dark:hover:bg-red-500/15 dark:hover:text-red-400
                       transition-all duration-200 cursor-pointer"
                title={$t("queue_panel.remove")}
              >
                <Icon icon="mynaui:trash" width="13" height="13" />
              </button>
            </div>
          </div>
        {/each}
      </div>
      {#if upcomingTracks.length > affiches}
        <button type="button" class="mt-2 w-full rounded-xl py-2.5 text-[12px] font-semibold cursor-pointer
                                     text-neutral-500 dark:text-neutral-400 hover:bg-black/3 dark:hover:bg-white/4 transition-colors"
                onclick={() => (affiches += LOT)}>
          {$t("queue_panel.show_more").replace("{n}", Math.min(LOT, upcomingTracks.length - affiches).toLocaleString($currentLocale))}
        </button>
      {/if}
    {/if}

    <!-- ===== EMPTY STATES ===== -->
    {#if upcomingTracks.length === 0}
      {#if !currentTrack}
        <!-- Completely empty -->
        <div class="flex h-72 flex-col items-center justify-center text-center mt-12">
          <div class="relative mb-6">
            <div class="flex h-24 w-24 items-center justify-center rounded-3xl
                        bg-neutral-50 dark:bg-white/3
                        border border-neutral-100 dark:border-white/5
                        shadow-[0_4px_24px_rgba(0,0,0,0.03)]
                        dark:shadow-[0_4px_24px_rgba(0,0,0,0.2)]">
              <Icon icon="mynaui:music" width="40" height="40"
                class="text-neutral-300 dark:text-neutral-600" />
            </div>
            <!-- Decorative rings -->
            <div class="absolute -inset-3 rounded-[28px] border border-neutral-100/60 dark:border-white/3"></div>
            <div class="absolute -inset-6 rounded-4xl border border-neutral-100/30 dark:border-white/2"></div>
          </div>
          <h3 class="text-[15px] font-bold text-neutral-800 dark:text-neutral-200">
            {$t("queue_panel.empty_title")}
          </h3>
          <p class="mt-2.5 text-[13px] leading-relaxed text-neutral-400 dark:text-neutral-500 max-w-55">
            {$t("queue_panel.empty_desc")}
          </p>
        </div>
      {:else}
        <!-- Has current track but no upcoming -->
        <div class="flex h-52 flex-col items-center justify-center text-center mt-8">
          <div class="flex h-14 w-14 items-center justify-center rounded-2xl mb-4
                      bg-neutral-50 dark:bg-white/3
                      border border-neutral-100 dark:border-white/5">
            <Icon icon="mynaui:music" width="24" height="24" class="text-neutral-300 dark:text-neutral-400" />
          </div>
          <h3 class="text-[13px] font-bold text-neutral-600 dark:text-neutral-300">
            {$t("queue_panel.no_next_title")}
          </h3>
          <p class="mt-1.5 text-[12px] text-neutral-400 dark:text-neutral-500">
            {$t("queue_panel.no_next_desc")}
          </p>
        </div>
      {/if}
    {/if}
  </div>
</aside>