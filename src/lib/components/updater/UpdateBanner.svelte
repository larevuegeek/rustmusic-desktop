<script lang="ts">
  import Icon from "@iconify/svelte";
  import { t, currentLocale } from "$lib/i18n";
  import { tailleLisible } from "$lib/helper/tools/sizeTools";
  import { updaterState } from "$lib/stores/updater/updater.store";
  import {
    downloadAndInstall,
    dismissUpdate,
  } from "$lib/services/updater/updater.service";

  // ─── Composant Banner mise à jour ──────────────────────────────────────
  // Position : fixed bottom-right, au-dessus du player.
  // Affiché uniquement quand le store updaterState n'est pas "idle".
  // Stages : available → downloading (avec progress) → installing → ready (restart auto)
  //          ou → error (dismiss possible)

  let percent = $derived.by(() => {
    const s = $updaterState;
    if (s.kind !== "downloading" || !s.total || s.total <= 0) return 0;
    return Math.min(100, Math.round((s.downloaded / s.total) * 100));
  });

  let downloadedMB = $derived.by(() => {
    const s = $updaterState;
    if (s.kind !== "downloading") return "0";
    return tailleLisible(s.downloaded, $currentLocale) || "0";
  });

  let totalMB = $derived.by(() => {
    const s = $updaterState;
    if (s.kind !== "downloading" || !s.total) return null;
    return tailleLisible(s.total, $currentLocale);
  });
</script>

{#if $updaterState.kind !== "idle" && $updaterState.kind !== "checking"}
  <div
    class="fixed bottom-38 right-4 z-50 max-w-sm w-[calc(100%-2rem)]
           rounded-xl shadow-2xl border backdrop-blur-xl
           bg-neutral-900/90 text-neutral-100 border-white/10
           p-4"
    role="status"
    aria-live="polite"
  >
    {#if $updaterState.kind === "available"}
      <div class="flex items-start gap-3">
        <div class="shrink-0 mt-0.5">
          <span class="flex items-center justify-center w-9 h-9 rounded-lg bg-green-500/15 border border-green-500/25">
            <Icon icon="lucide:sparkles" width={18} class="text-green-400" />
          </span>
        </div>
        <div class="flex-1 min-w-0">
          <p class="text-sm font-semibold text-white">
            {$t('updater.available_title').replace('{version}', $updaterState.version)}
          </p>
          <p class="text-xs text-neutral-400 mt-0.5">
            {$t('updater.available_desc')}
          </p>
          <div class="flex items-center gap-2 mt-3">
            <button
              onclick={downloadAndInstall}
              class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-green-600 hover:bg-green-500
                     text-white text-xs font-medium transition-colors cursor-pointer"
            >
              <Icon icon="lucide:download" width={13} />
              {$t('updater.install_now')}
            </button>
            <button
              onclick={dismissUpdate}
              class="px-3 py-1.5 rounded-lg text-xs text-neutral-400 hover:text-white hover:bg-white/5
                     transition-colors cursor-pointer"
            >
              {$t('updater.later')}
            </button>
          </div>
        </div>
      </div>
    {:else if $updaterState.kind === "downloading"}
      <div class="flex items-start gap-3">
        <div class="shrink-0 mt-0.5">
          <Icon icon="lucide:loader-circle" width={18} class="text-green-400 animate-spin" />
        </div>
        <div class="flex-1 min-w-0">
          <p class="text-sm font-semibold text-white">
            {$t('updater.downloading_title').replace('{version}', $updaterState.version)}
          </p>
          <p class="text-xs text-neutral-400 mt-0.5 font-mono">
            {downloadedMB}{totalMB ? ` / ${totalMB} · ${percent}%` : ''}
          </p>
          <div class="mt-2 h-1 rounded-full bg-white/10 overflow-hidden">
            <div
              class="h-full bg-linear-to-r from-green-500 to-emerald-400 transition-all duration-200"
              style="width: {percent}%"
            ></div>
          </div>
        </div>
      </div>
    {:else if $updaterState.kind === "installing"}
      <div class="flex items-center gap-3">
        <Icon icon="lucide:loader-circle" width={18} class="text-green-400 animate-spin" />
        <p class="text-sm font-semibold text-white flex-1">
          {$t('updater.installing')}
        </p>
      </div>
    {:else if $updaterState.kind === "ready"}
      <div class="flex items-center gap-3">
        <Icon icon="lucide:check-circle-2" width={18} class="text-green-400" />
        <p class="text-sm font-semibold text-white flex-1">
          {$t('updater.ready_restart').replace('{version}', $updaterState.version)}
        </p>
      </div>
    {:else if $updaterState.kind === "error"}
      <div class="flex items-start gap-3">
        <div class="shrink-0 mt-0.5">
          <Icon icon="lucide:alert-triangle" width={18} class="text-rose-400" />
        </div>
        <div class="flex-1 min-w-0">
          <p class="text-sm font-semibold text-white">
            {$t('updater.error_title')}
          </p>
          <p class="text-xs text-neutral-400 mt-0.5 break-words">
            {$updaterState.message}
          </p>
          <button
            onclick={dismissUpdate}
            class="mt-2 px-3 py-1.5 rounded-lg text-xs text-neutral-400 hover:text-white hover:bg-white/5
                   transition-colors cursor-pointer"
          >
            {$t('updater.dismiss')}
          </button>
        </div>
      </div>
    {/if}
  </div>
{/if}
