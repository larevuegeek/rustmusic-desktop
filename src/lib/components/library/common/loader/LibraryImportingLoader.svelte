<script lang="ts">
  import Icon from "@iconify/svelte";
  import { importProgressStore } from "$lib/stores/library/importProgress.store";
  import { t } from "$lib/i18n";
  import { minutesSecondes } from "$lib/helper/tools/dateTools";

  let percent = $derived($importProgressStore.percent);
  let current = $derived($importProgressStore.current);
  let total = $derived($importProgressStore.total);
  let fileName = $derived($importProgressStore.fileName);
  let elapsedMs = $derived($importProgressStore.elapsedMs);
  let active = $derived($importProgressStore.active);

  let elapsedFormatted = $derived(minutesSecondes(elapsedMs / 1000));

  // Estimation du temps restant
  let etaFormatted = $derived.by(() => {
    if (!active || current === 0 || elapsedMs === 0) return null;
    const msPerFile = elapsedMs / current;
    const remaining = (total - current) * msPerFile;
    const s = Math.floor(remaining / 1000);
    if (s < 5) return $t("system.almost_done");
    return $t("system.time_left").replace("{time}", minutesSecondes(s));
  });

  const circumference = 2 * Math.PI * 42;
</script>

<div class="flex flex-col items-center justify-center h-full py-16 px-6">
  <div class="flex flex-col items-center max-w-sm w-full">

    <!-- Cercle de progression -->
    <div class="relative w-32 h-32 mb-8">
      <!-- Glow effect -->
      {#if active}
        <div class="absolute inset-2 rounded-full bg-emerald-500/15 blur-xl animate-pulse"></div>
      {/if}

      <svg class="w-32 h-32 -rotate-90 relative" viewBox="0 0 100 100">
        <!-- Track -->
        <circle cx="50" cy="50" r="42" fill="none"
                stroke="currentColor" stroke-width="3"
                class="text-neutral-200/60 dark:text-white/6" />
        <!-- Progress -->
        <circle cx="50" cy="50" r="42" fill="none"
                stroke="url(#importGrad)" stroke-width="3.5"
                stroke-linecap="round"
                stroke-dasharray="{circumference}"
                stroke-dashoffset="{circumference * (1 - percent / 100)}"
                class="transition-all duration-500 ease-out"
                style="filter: drop-shadow(0 0 6px rgba(16, 185, 129, 0.4));" />
        <defs>
          <linearGradient id="importGrad" x1="0%" y1="0%" x2="100%" y2="0%">
            <stop offset="0%" stop-color="#22c55e" />
            <stop offset="50%" stop-color="#10b981" />
            <stop offset="100%" stop-color="#06b6d4" />
          </linearGradient>
        </defs>
      </svg>

      <!-- Centre -->
      <div class="absolute inset-0 flex flex-col items-center justify-center">
        <span class="text-2xl font-bold tabular-nums text-neutral-800 dark:text-white tracking-tight">
          {percent}<span class="text-sm font-medium text-neutral-400">%</span>
        </span>
      </div>
    </div>

    <!-- Titre + icône -->
    <div class="flex items-center gap-2 mb-2">
      {#if active}
        <Icon icon="lucide:hard-drive-download" width={18} class="text-emerald-500" />
      {:else if percent === 100}
        <Icon icon="lucide:check-circle-2" width={18} class="text-emerald-500" />
      {:else}
        <Icon icon="lucide:loader-2" width={18} class="text-neutral-400 animate-spin" />
      {/if}
      <p class="text-sm font-semibold text-neutral-800 dark:text-neutral-100">
        {#if active}
          {$t("system.importing")}
        {:else if percent === 100}
          {$t("system.importing_done")}
        {:else}
          {$t("system.preparing")}
        {/if}
      </p>
    </div>

    <!-- Stats -->
    <div class="flex items-center gap-3 text-xs tabular-nums text-neutral-400 dark:text-neutral-500 mb-5">
      <span class="flex items-center gap-1">
        <Icon icon="lucide:music" width={12} />
        {current} / {total}
      </span>
      {#if elapsedMs > 0}
        <span class="w-px h-3 bg-neutral-300/50 dark:bg-neutral-700/50"></span>
        <span class="flex items-center gap-1">
          <Icon icon="lucide:clock" width={12} />
          {elapsedFormatted}
        </span>
      {/if}
      {#if etaFormatted}
        <span class="w-px h-3 bg-neutral-300/50 dark:bg-neutral-700/50"></span>
        <span class="text-emerald-500/70">{etaFormatted}</span>
      {/if}
    </div>

    <!-- Barre de progression -->
    <div class="w-full h-1.5 rounded-full bg-neutral-200/70 dark:bg-white/6 overflow-hidden mb-4">
      <div
        class="h-full rounded-full relative overflow-hidden transition-all duration-500 ease-out
               bg-linear-to-r from-emerald-500 via-emerald-400 to-cyan-400"
        style="width: {percent}%"
      >
        <div class="absolute inset-0 bg-linear-to-r from-transparent via-white/25 to-transparent animate-shimmer"></div>
      </div>
    </div>

    <!-- Fichier en cours -->
    {#if fileName}
      <div class="flex items-center gap-1.5 w-full px-2">
        <Icon icon="lucide:file-audio" width={11} class="text-neutral-400/60 shrink-0" />
        <p class="text-[11px] text-neutral-400 dark:text-neutral-500 truncate">
          {fileName}
        </p>
      </div>
    {/if}

  </div>
</div>

<style>
  @keyframes shimmer {
    0% { transform: translateX(-100%); }
    100% { transform: translateX(200%); }
  }
  .animate-shimmer {
    animation: shimmer 1.5s ease-in-out infinite;
  }
</style>
