<script lang="ts">
  // Micro-lecteur : la pochette seule ; au survol, les commandes par-dessus. La molette règle le volume.
  import Icon from "@iconify/svelte";
  import { fade } from "svelte/transition";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import AnneauAttente from "#lib/components/ui/loader/AnneauAttente.svelte";
  import MiniVolume from "./MiniVolume.svelte";
  import PlayerProgressBar from "./PlayerProgressBar.svelte";
  import { t } from "#lib/i18n";
  import { player } from "#lib/stores/player/player.store";
  import { playerService } from "#lib/services/player/player.service";
  import { volume, reglerVolume } from "#lib/stores/player/volume.store";
  import { playbackPipelineStore } from "#lib/stores/player/playbackPipeline.store";
  import { settingsStore } from "#lib/stores/settings/settings.store";
  import { couleurMorceau } from "#lib/stores/player/trackColor.store";
  import { minutesSecondes } from "#lib/helper/tools/dateTools";
  import { exitMicroPlayer, exitMiniPlayer, miniPinned, toggleMiniPin, resizeMicroPlayer, endMicroResize, currentMicroSize } from "#lib/stores/ui/miniPlayer.store";

  const NO_COVER = "/images/no-cd.png";

  const hasTrack = $derived(!!$player?.pathFile);
  const coverSrc = $derived($player?.audioFile?.tags?.attached_images?.[0]?.image_src ?? null);
  const isPlaying = $derived($player?.status === "playing");
  const waiting = $derived(isPlaying && !!$player?.isPreparing);
  const duration = $derived($player?.duration ?? 0);
  const position = $derived($player?.jsPosition ?? 0);
  const percent = $derived(duration ? Math.min(100, Math.max(0, (position / duration) * 100)) : 0);
  // En DSD natif, le volume se règle sur le DAC.
  const dopActive = $derived($playbackPipelineStore?.backend?.endsWith("DoP") === true);
  const muted = $derived($volume === 0);
  const volumeIcon = $derived(muted ? "material-symbols:volume-off-rounded" : $volume < 50 ? "material-symbols:volume-down-rounded" : "material-symbols:volume-up-rounded");

  // Molette : volume, avec le niveau affiché un instant.
  let volumeHint = $state(false);
  let hintTimer: ReturnType<typeof setTimeout> | null = null;
  function onWheel(e: WheelEvent) {
    if (dopActive) return;
    reglerVolume($volume + (e.deltaY < 0 ? 5 : -5));
    volumeHint = true;
    if (hintTimer) clearTimeout(hintTimer);
    hintTimer = setTimeout(() => (volumeHint = false), 900);
  }

  // Toute la pochette déplace la fenêtre, sans le double-clic « agrandir » des zones Tauri.
  function onPointerDown(e: PointerEvent) {
    if (e.button !== 0 || (e.target as HTMLElement).closest("button, input, [role=slider], [data-no-drag]")) return;
    void getCurrentWindow().startDragging();
  }

  // Même règle que la barre de titre : réduire dans la zone de notification, ou fermer.
  async function close() {
    const w = getCurrentWindow();
    if ($settingsStore.minimize_to_tray === "true") return w.hide();
    await w.hide();
    await exitMiniPlayer();
    await w.close();
  }

  // Poignée du coin : le carré grandit de la plus grande des deux distances, jusqu'à ×3.
  let resizeStart: { x: number; y: number; size: number } | null = null;
  let resizeFrame = 0;
  function onGripDown(e: PointerEvent) {
    e.stopPropagation();
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    resizeStart = { x: e.screenX, y: e.screenY, size: currentMicroSize() };
  }
  function onGripMove(e: PointerEvent) {
    if (!resizeStart) return;
    const start = resizeStart;
    const size = start.size + Math.max(e.screenX - start.x, e.screenY - start.y);
    cancelAnimationFrame(resizeFrame);
    resizeFrame = requestAnimationFrame(() => void resizeMicroPlayer(size));
  }
  function onGripUp() {
    if (!resizeStart) return;
    resizeStart = null;
    void endMicroResize();
  }

  const topButton = "w-7 h-7 flex items-center justify-center rounded-lg cursor-pointer transition-colors text-white/85 hover:text-white hover:bg-white/15";
  const sideButton = "w-10 h-10 flex items-center justify-center rounded-full cursor-pointer transition-colors text-white hover:bg-white/15";
</script>

<div
  class="lecteur group fixed inset-0 overflow-hidden select-none bg-black cursor-grab active:cursor-grabbing"
  style:--lh={$couleurMorceau.h}
  style:--ls={$couleurMorceau.s}
  role="presentation"
  onpointerdown={onPointerDown}
  onwheel={onWheel}
>
  <img src={hasTrack && coverSrc ? coverSrc : NO_COVER} alt="" class="absolute inset-0 w-full h-full object-cover pointer-events-none"
       onerror={(e) => ((e.currentTarget as HTMLImageElement).src = NO_COVER)} />

  <!-- Voile et commandes au survol -->
  <div class="absolute inset-0 flex flex-col opacity-0 group-hover:opacity-100 focus-within:opacity-100 transition-opacity duration-200
              bg-[linear-gradient(to_bottom,rgba(0,0,0,0.7),rgba(0,0,0,0.5)_35%,rgba(0,0,0,0.5)_65%,rgba(0,0,0,0.75))]">
    <div class="flex items-center gap-0.5 p-1.5">
      {#if hasTrack}
        <span class="flex-1 pl-1.5 font-mono text-[10px] tabular-nums text-white/80">{minutesSecondes(position)} / {minutesSecondes(duration)}</span>
      {:else}
        <span class="flex-1"></span>
      {/if}
      <button type="button" class="{topButton} {$miniPinned ? 'text-(--lc-acc)!' : ''}" title={$t("mini.pin")} aria-label={$t("mini.pin")} aria-pressed={$miniPinned} onclick={toggleMiniPin}>
        <Icon icon={$miniPinned ? "material-symbols:keep" : "material-symbols-light:keep-outline"} width="16" />
      </button>
      <button type="button" class={topButton} title={$t("mini.back_to_mini")} aria-label={$t("mini.back_to_mini")} onclick={() => exitMicroPlayer()}>
        <Icon icon="material-symbols-light:open-in-full-rounded" width="16" />
      </button>
      <button type="button" class="{topButton} hover:bg-[#c42b1c]!" title={$t("common.close")} aria-label={$t("common.close")} onclick={close}>
        <Icon icon="material-symbols-light:close-rounded" width="18" />
      </button>
    </div>

    <div class="flex-1 flex items-center justify-center gap-1.5">
      <button type="button" class={sideButton} title={$t("player.prev")} aria-label={$t("player.prev")} onclick={() => playerService.prevTrack()}>
        <Icon icon="material-symbols:skip-previous-rounded" width="28" />
      </button>
      <button type="button"
              class="relative w-14 h-14 flex items-center justify-center rounded-full cursor-pointer transition-transform hover:scale-105 active:scale-95 bg-white/92 text-black shadow-[0_6px_18px_rgba(0,0,0,0.4)]"
              title={waiting ? $t("player.preparing") : isPlaying ? $t("player.pause") : $t("player.play")}
              aria-label={isPlaying ? $t("player.pause") : $t("player.play")}
              aria-busy={waiting}
              onclick={() => playerService.handleTogglePlay()}>
        <Icon icon={isPlaying ? "material-symbols:pause-rounded" : "material-symbols:play-arrow-rounded"} width="32" />
        <AnneauAttente actif={waiting} />
      </button>
      <button type="button" class={sideButton} title={$t("player.next")} aria-label={$t("player.next")} onclick={() => playerService.nextTrack()}>
        <Icon icon="material-symbols:skip-next-rounded" width="28" />
      </button>
    </div>

    <!-- Timeline du morceau, et le volume à droite -->
    <div class="flex items-center gap-1.5 pl-3 pr-1.5 pb-3.5" data-no-drag>
      <div class="flex-1 min-w-0">
        <PlayerProgressBar position={percent} onseek={(p) => duration && playerService.seekTo((p / 100) * duration)} />
      </div>
      <MiniVolume buttonClass={topButton} placement="above" forceVertical />
    </div>
  </div>

  <!-- Poignée de redimensionnement -->
  <div
    class="absolute bottom-0 right-0 w-4 h-4 flex items-end justify-end p-0.5 cursor-nwse-resize touch-none text-white/70
           opacity-0 group-hover:opacity-100 transition-opacity"
    data-no-drag
    role="presentation"
    title={$t("mini.resize")}
    onpointerdown={onGripDown}
    onpointermove={onGripMove}
    onpointerup={onGripUp}
    onpointercancel={onGripUp}
  >
    <svg viewBox="0 0 10 10" class="w-2.5 h-2.5" aria-hidden="true"><path d="M9 3L3 9M9 6L6 9" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" /></svg>
  </div>

  <!-- Niveau du volume, le temps d'un tour de molette -->
  {#if volumeHint}
    <div class="absolute top-2.5 left-1/2 -translate-x-1/2 flex items-center gap-1.5 px-2.5 h-7 rounded-full bg-black/70 text-white pointer-events-none"
         transition:fade={{ duration: 120 }}>
      <Icon icon={volumeIcon} width="15" />
      <span class="font-mono text-[11px] tabular-nums">{$volume}</span>
    </div>
  {/if}

  <!-- Progression au repos ; au survol, la timeline prend le relais -->
  {#if hasTrack}
    <div class="absolute inset-x-0 bottom-0 h-0.75 bg-white/20 pointer-events-none group-hover:opacity-0 transition-opacity">
      <div class="h-full bg-(--lc-play)" style:width="{percent}%"></div>
    </div>
  {/if}
</div>
