<script lang="ts">
  import Icon from "@iconify/svelte";
  import { fade } from "svelte/transition";
  import CoverImg from "$lib/components/ui/image/CoverImg.svelte";
  import { t } from "$lib/i18n";
  import { player } from "$lib/stores/player/player.store";
  import { playerService } from "$lib/services/player/player.service";
  import { queueState } from "$lib/stores/queue/queueState.store";
  import { displayTitle } from "$lib/helper/tools/stringTools";
  import { durationToMinutes } from "$lib/helper/tools/dateTools";
  import { isDsdFormat } from "$lib/helper/tools/audioFormatTools";
  import { getLyrics } from "$lib/services/lyrics/lyrics.service";
  import { parseLrc, findActiveLineIndex, type LrcLine } from "$lib/helper/lyrics/lrcParser";
  import { exitMiniPlayer, setMiniExpanded, reportCollapsedHeight } from "$lib/stores/ui/miniPlayer.store";
  import type { QueueTrack } from "$lib/types/db/queue/QueueTrack";

  let audioFile = $derived($player?.audioFile);
  let audioTags = $derived(audioFile?.tags);
  let trackTitle = $derived(
    displayTitle(audioTags?.title, $player?.pathFile, $t("common.unknown_title")),
  );
  let artist = $derived(audioTags?.artist ?? "");
  let coverSrc = $derived(
    audioTags?.attached_images?.[0]?.image_src ?? "/images/no-cd.png",
  );
  let pathFile = $derived($player?.pathFile ?? null);

  let isPlaying = $derived($player?.status === "playing");
  let duration = $derived($player?.duration ?? 0);
  let jsPosition = $derived($player?.jsPosition ?? 0);
  let percent = $derived(
    duration ? Math.min(100, Math.max(0, (jsPosition / duration) * 100)) : 0,
  );
  let hasTrack = $derived(!!$player?.pathFile);

  let qualityBadge = $derived.by(() => {
    const f = audioFile;
    if (!f) return null;
    if (isDsdFormat(f.audio_format)) return { label: "DSD", tone: "amber" };
    if (f.bits_per_sample && f.bits_per_sample >= 24) return { label: "Hi-Res", tone: "emerald" };
    if (f.audio_format) return { label: f.audio_format, tone: "neutral" };
    return null;
  });
  let rateLabel = $derived.by(() => {
    const sr = audioFile?.sample_rate;
    if (!sr) return "";
    if (isDsdFormat(audioFile?.audio_format)) {
      return `${(sr / 1_000_000).toFixed(2).replace(/\.?0+$/, "")} MHz`;
    }
    return `${Math.round(sr / 1000)} kHz`;
  });

  // ─── Panneau déroulant : 'none' | 'queue' | 'lyrics' ───
  type Panel = "none" | "queue" | "lyrics";
  let panel = $state<Panel>("none");

  async function selectPanel(p: Panel) {
    if (panel === p) {
      panel = "none";
      setMiniExpanded(false);
    } else {
      panel = p;
      setMiniExpanded(true);
    }
  }

  // ─── File d'attente ───
  let currentIndex = $derived($queueState.currentIndex);
  let upNext = $derived(
    currentIndex > -1 ? $queueState.tracks.slice(currentIndex + 1) : $queueState.tracks,
  );
  async function playFromQueue(track: QueueTrack, localIdx: number) {
    const real = (currentIndex > -1 ? currentIndex + 1 : 0) + localIdx;
    await queueState.setCurrentIndex(real);
    await playerService.playFile(track);
  }

  // ─── Paroles synchronisées ───
  let lyricsLines = $state<LrcLine[]>([]);
  let lyricsPlain = $state<string | null>(null);
  let lyricsStatus = $state<"idle" | "loading" | "ready" | "empty">("idle");
  let lyricsForPath = "";
  let lyricsBox = $state<HTMLDivElement | null>(null);
  let lineEls: Record<number, HTMLElement> = {};

  let currentMs = $derived(jsPosition * 1000);
  let activeLine = $derived(
    lyricsLines.length > 0 ? findActiveLineIndex(lyricsLines, currentMs) : -1,
  );

  async function loadLyrics(path: string) {
    lyricsStatus = "loading";
    lyricsLines = [];
    lyricsPlain = null;
    lyricsForPath = path;
    try {
      const res = await getLyrics(path);
      if (lyricsForPath !== path) return; // course : le morceau a changé
      if (!res || (!res.synced && !res.plain)) {
        lyricsStatus = "empty";
        return;
      }
      if (res.synced) lyricsLines = parseLrc(res.synced);
      lyricsPlain = res.plain;
      lyricsStatus = lyricsLines.length > 0 || res.plain ? "ready" : "empty";
    } catch {
      lyricsStatus = "empty";
    }
  }

  // Charger les paroles quand on ouvre l'onglet OU quand le morceau change.
  $effect(() => {
    if (panel === "lyrics" && pathFile && pathFile !== lyricsForPath) {
      loadLyrics(pathFile);
    }
  });

  // Auto-scroll de la ligne active au centre.
  $effect(() => {
    const idx = activeLine;
    if (panel !== "lyrics" || idx < 0 || !lyricsBox) return;
    const el = lineEls[idx];
    if (el) el.scrollIntoView({ block: "center", behavior: "smooth" });
  });

  // Mesure la hauteur réelle du contenu replié → la fenêtre s'y ajuste au pixel.
  let headerEl = $state<HTMLDivElement | null>(null);
  $effect(() => {
    const el = headerEl;
    if (!el) return;
    reportCollapsedHeight(el.offsetHeight);
    const ro = new ResizeObserver(() => reportCollapsedHeight(el.offsetHeight));
    ro.observe(el);
    return () => ro.disconnect();
  });

  function seekAt(e: MouseEvent) {
    if (!duration) return;
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const ratio = Math.min(1, Math.max(0, (e.clientX - rect.left) / rect.width));
    playerService.seekTo(ratio * duration);
  }
  function seekKey(e: KeyboardEvent) {
    if (!duration) return;
    if (e.key === "ArrowRight") playerService.seekTo(Math.min(duration, jsPosition + 5));
    else if (e.key === "ArrowLeft") playerService.seekTo(Math.max(0, jsPosition - 5));
  }
</script>

<div class="fixed inset-0 flex flex-col bg-white dark:bg-neutral-950 text-neutral-900 dark:text-white overflow-hidden select-none">
  <!-- Halo coloré depuis la cover -->
  <div
    class="absolute top-0 left-0 w-48 h-48 rounded-full blur-[70px] opacity-20 pointer-events-none"
    style="background-image: url({coverSrc}); background-size: cover; background-position: center;"
  ></div>

  <!-- Contenu replié (mesuré pour dimensionner la fenêtre au pixel près) -->
  <div bind:this={headerEl} class="relative shrink-0">
  <!-- Top bar -->
  <div data-tauri-drag-region class="relative flex items-center justify-between px-3 pt-2 pb-0.5 shrink-0">
    <span data-tauri-drag-region class="flex items-center gap-1.5 text-[9px] font-semibold uppercase tracking-widest text-neutral-500 dark:text-neutral-400 pointer-events-none">
      <svg viewBox="0 0 256 256" class="w-3 h-3"><path fill="#22c55e" d="M232 128v56a24 24 0 0 1-24 24h-16a24 24 0 0 1-24-24v-40a24 24 0 0 1 24-24h23.65A87.71 87.71 0 0 0 128.68 40H128a88 88 0 0 0-87.64 80H64a24 24 0 0 1 24 24v40a24 24 0 0 1-24 24H48a24 24 0 0 1-24-24v-56a104.11 104.11 0 0 1 177.89-73.34A103.4 103.4 0 0 1 232 128"/></svg>
      RustMusic
    </span>
    <button
      class="flex items-center justify-center w-6 h-6 rounded-md cursor-pointer text-neutral-500 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-white transition-colors"
      onclick={() => exitMiniPlayer()}
      aria-label={$t("mini.restore")} title={$t("mini.restore")}
    >
      <Icon icon="lucide:maximize-2" width="13" />
    </button>
  </div>

  <!-- Player row -->
  <div class="relative flex items-center gap-3.5 px-3.5 pt-2 pb-1 shrink-0">
    <img src={coverSrc} alt="" class="w-15 h-15 rounded-xl object-cover shrink-0 shadow-xl shadow-black/50 ring-1 ring-white/10" />
    <div class="flex-1 min-w-0">
      <p class="text-[14px] font-bold truncate leading-tight" title={trackTitle}>
        {hasTrack ? trackTitle : $t("player.inactive")}
      </p>
      <p class="text-[11px] text-neutral-500 dark:text-neutral-400 truncate leading-tight mt-0.5">{artist}</p>
      {#if hasTrack && qualityBadge}
        <div class="flex items-center gap-1.5 mt-1.5">
          <span class="px-1.5 py-0.5 rounded text-[8px] font-bold uppercase tracking-wide
                       {qualityBadge.tone === 'amber' ? 'bg-amber-500/15 text-amber-300' :
                        qualityBadge.tone === 'emerald' ? 'bg-emerald-500/15 text-emerald-300' :
                        'bg-neutral-200/60 dark:bg-white/10 text-neutral-600 dark:text-neutral-300'}">
            {qualityBadge.label}
          </span>
          {#if rateLabel}<span class="text-[9px] text-neutral-500 dark:text-neutral-400 tabular-nums">{rateLabel}</span>{/if}
        </div>
      {/if}
    </div>
    <div class="flex items-center gap-2.5 shrink-0">
      <button class="text-neutral-600 dark:text-neutral-300 hover:text-neutral-900 dark:hover:text-white transition-colors cursor-pointer"
              onclick={async () => await playerService.prevTrack()} aria-label="Précédent">
        <Icon icon="lucide:skip-back" width="16" />
      </button>
      <button class="flex items-center justify-center w-10 h-10 rounded-full bg-emerald-500 text-white
                     hover:bg-emerald-400 transition-colors cursor-pointer shrink-0 shadow-lg shadow-emerald-500/25"
              onclick={() => playerService.handleTogglePlay()} aria-label={isPlaying ? "Pause" : "Lecture"}>
        <Icon icon={isPlaying ? "lucide:pause" : "lucide:play"} width="18" class={isPlaying ? "" : "ml-0.5"} />
      </button>
      <button class="text-neutral-600 dark:text-neutral-300 hover:text-neutral-900 dark:hover:text-white transition-colors cursor-pointer"
              onclick={async () => await playerService.nextTrack()} aria-label="Suivant">
        <Icon icon="lucide:skip-forward" width="16" />
      </button>
    </div>
  </div>

  <!-- Progression -->
  <div class="relative px-3.5 pb-1.5 shrink-0">
    <div class="flex items-center gap-2">
      <span class="text-[8px] text-neutral-500 dark:text-neutral-400 tabular-nums w-7 text-right">{durationToMinutes(jsPosition)}</span>
      <div class="relative flex-1 h-1.5 rounded-full bg-neutral-200/60 dark:bg-white/10 overflow-hidden cursor-pointer group"
           role="slider" tabindex="0" aria-label="Progression"
           aria-valuenow={Math.round(percent)} aria-valuemin={0} aria-valuemax={100}
           onclick={seekAt} onkeydown={seekKey}>
        <div class="absolute inset-y-0 left-0 rounded-full bg-emerald-500 group-hover:bg-emerald-400 transition-colors" style="width: {percent}%"></div>
      </div>
      <span class="text-[8px] text-neutral-500 dark:text-neutral-400 tabular-nums w-7">{durationToMinutes(duration)}</span>
    </div>
  </div>

  <!-- Onglets File d'attente / Paroles -->
  <div class="relative shrink-0 flex items-stretch border-t border-neutral-200/70 dark:border-white/5">
    {#each [{ id: 'queue', icon: 'ph:queue', label: $t('mini.up_next') }, { id: 'lyrics', icon: 'lucide:mic-vocal', label: $t('mini.lyrics') }] as tab}
      {@const isOpen = panel === tab.id}
      <button
        class="flex-1 flex items-center justify-center gap-1.5 py-2 text-[10px] font-medium cursor-pointer transition-colors relative
               {isOpen ? 'text-emerald-400' : 'text-neutral-500 dark:text-neutral-400 hover:text-neutral-200'}"
        onclick={() => selectPanel(tab.id as Panel)}
      >
        <Icon icon={tab.icon} width="12" />
        {tab.label}
        {#if tab.id === 'queue' && upNext.length > 0}
          <span class="px-1 rounded bg-neutral-200/60 dark:bg-white/10 text-[8px] tabular-nums">{upNext.length}</span>
        {/if}
        <Icon icon="lucide:chevron-{isOpen ? 'up' : 'down'}" width="11" class="opacity-60" />
        {#if isOpen}
          <span class="absolute bottom-0 left-1/2 -translate-x-1/2 w-8 h-0.5 rounded-full bg-emerald-500"></span>
        {/if}
      </button>
    {/each}
  </div>
  </div>
  <!-- /Contenu replié -->

  <!-- Panneau déroulant -->
  {#if panel !== "none"}
    <div class="relative flex-1 min-h-0 border-t border-neutral-200/70 dark:border-white/5 overflow-hidden" in:fade={{ duration: 180, delay: 60 }}>
      {#if panel === "queue"}
        <div class="h-full overflow-y-auto scrollbar-app">
          {#if upNext.length === 0}
            <p class="px-3 py-6 text-[11px] text-neutral-500 dark:text-neutral-400 text-center">{$t("mini.queue_empty")}</p>
          {:else}
            {#each upNext as track, i (track.queueId)}
              <button class="w-full flex items-center gap-2.5 px-3 py-1.5 text-left cursor-pointer hover:bg-neutral-100 dark:bg-white/5 transition-colors"
                      onclick={() => playFromQueue(track, i)}>
                <div class="w-8 h-8 rounded overflow-hidden shrink-0 bg-neutral-100 dark:bg-white/5">
                  <CoverImg path={track.cover} alt="" size="1x" class="w-full h-full object-cover" />
                </div>
                <div class="min-w-0 flex-1">
                  <p class="text-[11px] font-medium truncate">{track.title}</p>
                  <p class="text-[9px] text-neutral-500 dark:text-neutral-400 truncate">{track.artist ?? ''}</p>
                </div>
                {#if track.duration}
                  <span class="text-[9px] text-neutral-600 tabular-nums shrink-0">{durationToMinutes(track.duration)}</span>
                {/if}
              </button>
            {/each}
          {/if}
        </div>
      {:else if panel === "lyrics"}
        <div bind:this={lyricsBox} class="h-full overflow-y-auto scrollbar-app px-4 py-3">
          {#if lyricsStatus === "loading"}
            <p class="text-[11px] text-neutral-500 dark:text-neutral-400 text-center py-6">{$t("mini.lyrics_loading")}</p>
          {:else if lyricsLines.length > 0}
            <div class="space-y-1.5">
              {#each lyricsLines as line, i (i)}
                <p
                  bind:this={lineEls[i]}
                  class="text-[12px] leading-snug transition-colors duration-200
                         {i === activeLine ? 'text-emerald-400 font-semibold' : 'text-neutral-500 dark:text-neutral-400'}"
                >
                  {line.text || '♪'}
                </p>
              {/each}
            </div>
          {:else if lyricsPlain}
            <p class="text-[12px] text-neutral-600 dark:text-neutral-300 whitespace-pre-wrap leading-relaxed">{lyricsPlain}</p>
          {:else}
            <p class="text-[11px] text-neutral-500 dark:text-neutral-400 text-center py-6">{$t("mini.lyrics_empty")}</p>
          {/if}
        </div>
      {/if}
    </div>
  {/if}
</div>
