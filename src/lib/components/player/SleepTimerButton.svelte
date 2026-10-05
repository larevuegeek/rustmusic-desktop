<script lang="ts">
  import Icon from "@iconify/svelte";
  import { t } from "#lib/i18n";
  import {
    sleepTimer,
    formatSleepRemaining,
  } from "#lib/stores/player/sleepTimer.store";

  let open = $state(false);
  let anchorEl: HTMLElement | null = $state(null);
  let customMin = $state(20);

  let active = $derived($sleepTimer.mode !== "off");
  let endOfTrackActive = $derived($sleepTimer.mode === "end-of-track");

  const durations = [15, 30, 45, 60];

  function pick(minutes: number) {
    sleepTimer.startDuration(minutes);
    open = false;
  }
  function pickEndOfTrack() {
    sleepTimer.startEndOfTrack();
    open = false;
  }
  function pickCustom() {
    const m = Math.max(1, Math.min(600, Math.round(customMin)));
    sleepTimer.startDuration(m);
    open = false;
  }
  function cancel() {
    sleepTimer.cancel();
    open = false;
  }

  // Fermer au clic extérieur.
  $effect(() => {
    if (!open) return;
    function onDoc(e: MouseEvent) {
      if (anchorEl && !anchorEl.contains(e.target as Node)) open = false;
    }
    const timer = setTimeout(() => document.addEventListener("click", onDoc), 50);
    return () => {
      clearTimeout(timer);
      document.removeEventListener("click", onDoc);
    };
  });
</script>

<div class="relative" bind:this={anchorEl}>
  <button
    class="flex items-center justify-center rounded-full cursor-pointer transition-all duration-150 h-8
           {active
             ? 'text-emerald-500 hover:text-emerald-400 gap-1 px-2'
             : 'w-8 text-neutral-500 dark:text-neutral-400 hover:text-neutral-700 dark:hover:text-neutral-200'}"
    onclick={() => (open = !open)}
    aria-label={$t("sleep.title")}
    title={active
      ? ($sleepTimer.mode === "duration"
          ? $t("sleep.remaining").replace("{time}", formatSleepRemaining($sleepTimer.remaining))
          : $t("sleep.until_end"))
      : $t("sleep.title")}
  >
    <Icon icon="tabler:alarm-snooze" width="20" height="20" />
    {#if active && $sleepTimer.mode === "duration"}
      <span class="text-[11px] font-semibold tabular-nums">{formatSleepRemaining($sleepTimer.remaining)}</span>
    {/if}
    {#if active && $sleepTimer.mode === "end-of-track"}
      <Icon icon="lucide:flag" width="11" height="11" />
    {/if}
  </button>

  {#if open}
    <div
      class="absolute top-11 right-0 z-50 w-64 overflow-hidden
             rounded-2xl border border-neutral-200/80 dark:border-white/10
             bg-white/98 dark:bg-neutral-900/98 backdrop-blur-2xl
             shadow-2xl shadow-black/40"
    >
      <!-- Header -->
      <div class="flex items-center gap-2.5 px-4 pt-3.5 pb-3
                  border-b border-neutral-100 dark:border-white/5
                  bg-linear-to-b from-emerald-500/5 to-transparent">
        <div class="flex items-center justify-center w-8 h-8 rounded-lg bg-emerald-500/12 text-emerald-500 dark:text-emerald-400">
          <Icon icon="tabler:alarm-snooze" width="17" />
        </div>
        <div class="min-w-0">
          <p class="text-[13px] font-semibold text-neutral-800 dark:text-neutral-100 leading-tight">{$t("sleep.title")}</p>
          {#if active}
            <p class="text-[10px] text-emerald-600 dark:text-emerald-400 leading-tight mt-0.5">
              {$sleepTimer.mode === "duration"
                ? $t("sleep.remaining").replace("{time}", formatSleepRemaining($sleepTimer.remaining))
                : $t("sleep.until_end")}
            </p>
          {:else}
            <p class="text-[10px] text-neutral-400 dark:text-neutral-500 leading-tight mt-0.5">{$t("sleep.subtitle")}</p>
          {/if}
        </div>
      </div>

      <div class="p-2.5">
        <!-- Grille de durées -->
        <div class="grid grid-cols-2 gap-1.5">
          {#each durations as d}
            {@const sel = $sleepTimer.mode === "duration" && Math.round(($sleepTimer.remaining ?? 0) / 60) === d}
            <button
              class="flex items-center justify-center gap-1.5 px-2 py-2.5 rounded-xl text-sm font-medium cursor-pointer
                     border transition-all
                     {sel
                       ? 'bg-emerald-500/12 border-emerald-500/30 text-emerald-600 dark:text-emerald-400'
                       : 'bg-neutral-50 dark:bg-white/3 border-neutral-200/60 dark:border-white/6 text-neutral-700 dark:text-neutral-300 hover:border-emerald-500/25 hover:bg-emerald-500/5'}"
              onclick={() => pick(d)}
            >
              <Icon icon="lucide:clock" width="13" class={sel ? "text-emerald-500" : "text-neutral-400 dark:text-neutral-500"} />
              {d}
              <span class="text-[10px] opacity-60">min</span>
            </button>
          {/each}
        </div>

        <!-- Fin du morceau -->
        <button
          class="mt-1.5 w-full flex items-center gap-2.5 px-3 py-2.5 rounded-xl text-sm font-medium cursor-pointer
                 border transition-all
                 {endOfTrackActive
                   ? 'bg-emerald-500/12 border-emerald-500/30 text-emerald-600 dark:text-emerald-400'
                   : 'bg-neutral-50 dark:bg-white/3 border-neutral-200/60 dark:border-white/6 text-neutral-700 dark:text-neutral-300 hover:border-emerald-500/25 hover:bg-emerald-500/5'}"
          onclick={pickEndOfTrack}
        >
          <Icon icon="lucide:flag" width="14" class={endOfTrackActive ? "text-emerald-500" : "text-neutral-400 dark:text-neutral-500"} />
          {$t("sleep.end_of_track")}
          {#if endOfTrackActive}<Icon icon="lucide:check" width="15" class="ml-auto text-emerald-500" />{/if}
        </button>

        <!-- Personnalisé -->
        <div class="mt-1.5 flex items-center gap-2 px-3 py-2 rounded-xl bg-neutral-50 dark:bg-white/3 border border-neutral-200/60 dark:border-white/6">
          <Icon icon="lucide:sliders-horizontal" width="13" class="text-neutral-400 dark:text-neutral-500 shrink-0" />
          <input
            type="number"
            min="1"
            max="600"
            bind:value={customMin}
            class="w-12 bg-transparent text-sm tabular-nums text-neutral-800 dark:text-neutral-200 focus:outline-none"
            aria-label={$t("sleep.custom")}
          />
          <span class="text-[11px] text-neutral-400">min</span>
          <button
            class="ml-auto px-3 py-1 rounded-lg text-[11px] font-semibold cursor-pointer
                   bg-emerald-500 text-white hover:bg-emerald-600 transition-colors"
            onclick={pickCustom}
          >
            {$t("sleep.set")}
          </button>
        </div>

        {#if active}
          <button
            class="mt-1.5 w-full flex items-center justify-center gap-1.5 px-3 py-2 rounded-xl text-[12px] font-medium cursor-pointer
                   text-red-500 hover:bg-red-500/10 transition-colors"
            onclick={cancel}
          >
            <Icon icon="lucide:x" width="13" />
            {$t("sleep.cancel")}
          </button>
        {/if}
      </div>
    </div>
  {/if}
</div>
