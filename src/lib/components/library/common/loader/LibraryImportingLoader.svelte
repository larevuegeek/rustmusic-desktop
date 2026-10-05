<script lang="ts">
  import Icon from "@iconify/svelte";
  import { importProgressStore } from "#lib/stores/library/importProgress.store";
  import { t, currentLocale } from "#lib/i18n";
  import { minutesSecondes } from "#lib/helper/tools/dateTools";
  import { page } from "$app/state";
  import LibraryFoldersPopin from "#lib/components/library/common/popin/LibraryFoldersPopin.svelte";

  let percent = $derived($importProgressStore.percent);
  let current = $derived($importProgressStore.current);
  let total = $derived($importProgressStore.total);
  let fileName = $derived($importProgressStore.fileName);
  let elapsedMs = $derived($importProgressStore.elapsedMs);
  let active = $derived($importProgressStore.active);
  let listing = $derived(active && $importProgressStore.phase === 'listing');
  let stopping = $derived($importProgressStore.stopping);
  let found = $derived($importProgressStore.found);
  let directory = $derived($importProgressStore.directory);

  const libraryId = $derived($importProgressStore.libraryId ?? Number(page.params.library_id));
  let dossiers = $state(false);

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
        {#if listing}
          <Icon icon="lucide:folder-search" width={26} class="text-emerald-500 animate-pulse" />
        {:else}
          <span class="text-2xl font-bold tabular-nums text-neutral-800 dark:text-white tracking-tight">
            {percent}<span class="text-sm font-medium text-neutral-400">%</span>
          </span>
        {/if}
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
        {#if stopping}
          {$t("folders_popin.stopping")}
        {:else if listing}
          {$t("system.task_listing")}
        {:else if active}
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
        {#if listing}
          {$t(found === 1 ? "system.files_found_one" : "system.files_found_n").replace("{n}", found.toLocaleString($currentLocale))}
        {:else}
          {current.toLocaleString($currentLocale)} / {total.toLocaleString($currentLocale)}
        {/if}
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

    <!-- Dossier en cours -->
    {#if directory}
      <div class="flex items-center gap-1.5 w-full px-2 mb-1">
        <Icon icon="lucide:folder" width={11} class="text-neutral-400/60 shrink-0" />
        <p class="font-mono text-[11px] text-neutral-400 dark:text-neutral-500 truncate" title={directory}>
          {directory}
        </p>
      </div>
    {/if}

    <!-- Fichier en cours -->
    {#if fileName && !listing}
      <div class="flex items-center gap-1.5 w-full px-2">
        <Icon icon="lucide:file-audio" width={11} class="text-neutral-400/60 shrink-0" />
        <p class="text-[11px] text-neutral-400 dark:text-neutral-500 truncate">
          {fileName}
        </p>
      </div>
    {/if}

    <!-- Actions -->
    <div class="flex items-center gap-2 mt-6">
      {#if active}
        <button
          type="button"
          class="h-9 px-3.5 flex items-center gap-2 rounded-lg text-sm font-semibold cursor-pointer transition-colors
                 border border-neutral-200/80 dark:border-neutral-700/60 text-neutral-600 dark:text-neutral-300
                 enabled:hover:text-amber-500 enabled:hover:border-amber-500/40 disabled:opacity-50 disabled:cursor-default"
          disabled={stopping}
          onclick={() => importProgressStore.stop()}
        >
          <Icon icon="material-symbols:stop-circle-outline-rounded" width="18" />
          {stopping ? $t("folders_popin.stopping") : $t("folders_popin.stop")}
        </button>
      {/if}
      {#if libraryId}
        <button
          type="button"
          class="h-9 px-3.5 flex items-center gap-2 rounded-lg text-sm font-semibold cursor-pointer transition-colors
                 text-neutral-500 dark:text-neutral-400 hover:text-neutral-800 dark:hover:text-neutral-100
                 hover:bg-neutral-100/80 dark:hover:bg-neutral-800/60"
          onclick={() => (dossiers = true)}
        >
          <Icon icon="material-symbols:folder-open-outline-rounded" width="18" />
          {$t("library_head.folders")}
        </button>
      {/if}
    </div>

  </div>
</div>

{#if dossiers && libraryId}
  <LibraryFoldersPopin bind:open={dossiers} {libraryId} />
{/if}

<style>
  @keyframes shimmer {
    0% { transform: translateX(-100%); }
    100% { transform: translateX(200%); }
  }
  .animate-shimmer {
    animation: shimmer 1.5s ease-in-out infinite;
  }
</style>
