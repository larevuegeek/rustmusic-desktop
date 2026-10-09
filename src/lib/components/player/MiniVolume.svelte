<script lang="ts">
  // Volume du mini-lecteur : un bouton (molette = réglage), une bulle verticale ou horizontale (Apparence).
  import Icon from "@iconify/svelte";
  import { t } from "#lib/i18n";
  import { volume, reglerVolume } from "#lib/stores/player/volume.store";
  import { playbackPipelineStore } from "#lib/stores/player/playbackPipeline.store";
  import { settingsStore } from "#lib/stores/settings/settings.store";

  let {
    buttonClass,
    placement = "side",
    forceVertical = false,
  }: {
    buttonClass: string;
    /** `side` : à gauche du bouton (mini) ; `above` : au-dessus (micro). */
    placement?: "side" | "above";
    /** Le micro est trop étroit pour la bulle horizontale. */
    forceVertical?: boolean;
  } = $props();

  // En DSD natif, le volume se règle sur le DAC.
  const dopActive = $derived($playbackPipelineStore?.backend?.endsWith("DoP") === true);
  const muted = $derived($volume === 0);
  const horizontal = $derived(!forceVertical && $settingsStore.mini_volume_layout === "horizontal");
  const icon = $derived(muted ? "material-symbols:volume-off-rounded" : $volume < 50 ? "material-symbols:volume-down-rounded" : "material-symbols:volume-up-rounded");

  let open = $state(false);
  let previous = 80;
  let root: HTMLElement | null = $state(null);

  function toggleMute() {
    if (muted) reglerVolume(previous > 0 ? previous : 80);
    else {
      previous = $volume;
      reglerVolume(0);
    }
  }

  function onWheel(e: WheelEvent) {
    if (dopActive) return;
    e.stopPropagation();
    reglerVolume($volume + (e.deltaY < 0 ? 5 : -5));
  }

  // Glissière verticale dessinée à la main : le range vertical natif varie d'une WebView à l'autre.
  let rail: HTMLElement | null = $state(null);

  function valueAt(clientY: number) {
    if (!rail) return;
    const r = rail.getBoundingClientRect();
    reglerVolume(((r.bottom - clientY) / r.height) * 100);
  }

  function onPointerDown(e: PointerEvent) {
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    valueAt(e.clientY);
  }

  function onPointerMove(e: PointerEvent) {
    if ((e.currentTarget as HTMLElement).hasPointerCapture(e.pointerId)) valueAt(e.clientY);
  }

  function onKeyDown(e: KeyboardEvent) {
    const step = { ArrowUp: 5, ArrowRight: 5, ArrowDown: -5, ArrowLeft: -5, PageUp: 10, PageDown: -10 }[e.key];
    if (step !== undefined) reglerVolume($volume + step);
    else if (e.key === "Home") reglerVolume(0);
    else if (e.key === "End") reglerVolume(100);
    else return;
    e.preventDefault();
    e.stopPropagation();
  }

  // Fermeture au clic extérieur ou sur Échap.
  $effect(() => {
    if (!open) return;
    const outside = (e: MouseEvent) => {
      if (root && !root.contains(e.target as Node)) open = false;
    };
    const escape = (e: KeyboardEvent) => {
      if (e.key === "Escape") open = false;
    };
    document.addEventListener("pointerdown", outside);
    document.addEventListener("keydown", escape);
    return () => {
      document.removeEventListener("pointerdown", outside);
      document.removeEventListener("keydown", escape);
    };
  });
</script>

<div class="relative shrink-0" bind:this={root} data-no-drag>
  <button
    type="button"
    class="{buttonClass} {open ? 'text-(--lc-acc)' : ''} {muted && !dopActive ? 'text-red-500/85 dark:text-red-400/85' : ''}"
    title={dopActive ? $t("player.volume_dop_locked") : `${$t("common.volume")} : ${$volume} %`}
    aria-label={$t("common.volume")}
    aria-expanded={open}
    onclick={() => (open = !open)}
    onwheel={onWheel}
  >
    <Icon {icon} width="22" />
  </button>

  {#if open && horizontal && !dopActive}
    <div
      class="absolute bottom-full right-0 mb-2 z-50 w-56 flex items-center gap-2 pl-1.5 pr-3 py-1.5 rounded-xl border
             bg-(--lc-menu) border-(--lc-menu-bd) shadow-[0_12px_30px_rgba(0,0,0,0.35)] text-(--lc-tx)"
    >
      <button
        type="button"
        class="w-8 h-8 shrink-0 flex items-center justify-center rounded-full cursor-pointer transition-colors hover:bg-(--lc-survol)
               {muted ? 'text-red-500/85 dark:text-red-400/85' : 'text-(--lc-ic-fort)'}"
        title={muted ? $t("player.unmute") : $t("player.mute")}
        aria-label={muted ? $t("player.unmute") : $t("player.mute")}
        onclick={toggleMute}
      >
        <Icon {icon} width="20" />
      </button>
      <!-- Même rail que le grand lecteur : la pastille reste dans le rail, même à 0. -->
      <div class="group/vol relative flex-1 h-5 flex items-center">
        <div class="relative w-full h-1.25 group-hover/vol:h-1.5 rounded-full bg-(--lc-rail) transition-[height] duration-150">
          <div
            class="absolute inset-y-0 left-0 rounded-full {muted ? 'bg-red-500/60' : 'bg-(--lc-acc)'}"
            style:width="calc((100% - 12px) * {$volume / 100} + 6px)"
          ></div>
          <div
            class="absolute top-1/2 w-3 h-3 -mt-1.5 rounded-full bg-white shadow-[0_1px_4px_rgba(0,0,0,0.45)] pointer-events-none"
            style:left="calc((100% - 12px) * {$volume / 100})"
          ></div>
        </div>
        <input
          type="range"
          min="0"
          max="100"
          step="1"
          value={$volume}
          oninput={(e) => reglerVolume(Number(e.currentTarget.value))}
          onwheel={onWheel}
          class="absolute inset-0 w-full h-full cursor-pointer opacity-0"
          aria-label={$t("common.volume")}
        />
      </div>
      <span class="w-8 shrink-0 text-right font-mono text-[11px] tabular-nums text-(--lc-tx2)">{$volume}</span>
    </div>
  {:else if open}
    <div
      class="absolute z-50 flex flex-col items-center gap-1.5 py-2 rounded-xl border
             bg-(--lc-menu) border-(--lc-menu-bd) shadow-[0_12px_30px_rgba(0,0,0,0.35)] text-(--lc-tx)
             {dopActive ? 'bottom-full right-0 mb-2 w-44 px-3' : placement === 'above' ? 'bottom-full right-0 mb-1.5 w-11' : 'right-full top-1/2 -translate-y-1/2 mr-1.5 w-11'}"
    >
      {#if dopActive}
        <p class="text-[12px] leading-snug text-(--lc-tx2)">{$t("player.volume_dop_locked")}</p>
      {:else}
        <span class="font-mono text-[11px] tabular-nums text-(--lc-tx2)">{$volume}</span>
        <!-- Même rail que le grand lecteur, debout : la pastille reste dans le rail, même à 0. -->
        <div
          bind:this={rail}
          class="group/vol relative w-5 h-18 flex justify-center cursor-pointer touch-none outline-none rounded-full focus-visible:ring-2 focus-visible:ring-(--lc-acc)"
          role="slider"
          tabindex="0"
          aria-label={$t("common.volume")}
          aria-orientation="vertical"
          aria-valuemin={0}
          aria-valuemax={100}
          aria-valuenow={$volume}
          onpointerdown={onPointerDown}
          onpointermove={onPointerMove}
          onkeydown={onKeyDown}
          onwheel={onWheel}
        >
          <div class="relative w-1.25 h-full group-hover/vol:w-1.5 rounded-full bg-(--lc-rail) transition-[width] duration-150">
            <div
              class="absolute inset-x-0 bottom-0 rounded-full {muted ? 'bg-red-500/60' : 'bg-(--lc-acc)'}"
              style:height="calc((100% - 12px) * {$volume / 100} + 6px)"
            ></div>
            <div
              class="absolute left-1/2 w-3 h-3 -ml-1.5 rounded-full bg-white shadow-[0_1px_4px_rgba(0,0,0,0.45)] pointer-events-none"
              style:bottom="calc((100% - 12px) * {$volume / 100})"
            ></div>
          </div>
        </div>
        <button
          type="button"
          class="w-8 h-8 shrink-0 flex items-center justify-center rounded-full cursor-pointer transition-colors hover:bg-(--lc-survol)
                 {muted ? 'text-red-500/85 dark:text-red-400/85' : 'text-(--lc-ic-fort)'}"
          title={muted ? $t("player.unmute") : $t("player.mute")}
          aria-label={muted ? $t("player.unmute") : $t("player.mute")}
          onclick={toggleMute}
        >
          <Icon {icon} width="20" />
        </button>
      {/if}
    </div>
  {/if}
</div>
