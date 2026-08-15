<script lang="ts">
  // Suivi et compte rendu d'un traitement par lot.
  //
  // # Un panneau flottant, pas une popin
  // Un lot peut durer vingt minutes sur une bibliothèque réseau. Une popin
  // modale interdirait d'écouter, de naviguer, de préparer la suite pendant
  // ce temps — pour un traitement qui ne demande aucune intervention. Le
  // panneau se pose au-dessus du lecteur et laisse l'application utilisable.
  //
  // # Ce qu'il ne sait pas
  // Ni ce qui est traité, ni comment le relancer : le libellé et la fonction
  // de relance viennent de l'appelant. C'est ce qui lui permettra de servir
  // au renommage et au déplacement sans être touché.
  import Icon from "@iconify/svelte";
  import { fly } from "svelte/transition";
  import { batchStore } from "$lib/stores/ui/batch.store";
  import { cancelBatch, fileName, skipped } from "$lib/services/batch/batch.service";
  import { t } from "$lib/i18n";

  let state = $derived($batchStore);
  let report = $derived(state.report);

  let percent = $derived(
    state.progress && state.progress.total > 0
      ? Math.round((state.progress.done / state.progress.total) * 100)
      : 0,
  );

  async function handleCancel() {
    if (!state.jobId) return;
    batchStore.markCancelling();
    await cancelBatch(state.jobId);
  }

  async function handleRetry() {
    if (!report || !state.retry) return;
    const paths = report.failures.map((f) => f.path);
    const retry = state.retry;
    batchStore.dismiss();
    await retry(paths);
  }
</script>

{#if state.jobId}
  <div
    class="fixed right-4 bottom-28 z-40 w-80 rounded-xl overflow-hidden
           bg-white dark:bg-neutral-900
           ring-1 ring-black/5 dark:ring-white/10
           shadow-2xl shadow-black/20 dark:shadow-black/60"
    transition:fly={{ y: 12, duration: 180 }}
  >
    <!-- En-tête -->
    <div class="flex items-center gap-2.5 px-3.5 py-2.5
                border-b border-neutral-200/70 dark:border-white/8">
      {#if state.running}
        <Icon icon="lucide:loader-circle" width="14"
              class="shrink-0 animate-spin text-emerald-500" />
      {:else if report?.failures.length}
        <Icon icon="lucide:triangle-alert" width="14" class="shrink-0 text-amber-500" />
      {:else}
        <Icon icon="lucide:check" width="14" class="shrink-0 text-emerald-500" />
      {/if}

      <span class="flex-1 min-w-0 truncate text-[13px] font-medium
                   text-neutral-800 dark:text-neutral-100">
        {state.label}
      </span>

      <!-- Fermer n'apparaît qu'une fois le lot terminé : escamoter un
           traitement en cours ferait perdre son compte rendu. -->
      {#if !state.running}
        <button
          type="button"
          onclick={() => batchStore.dismiss()}
          aria-label={$t('batch.close')}
          class="shrink-0 w-6 h-6 rounded-md flex items-center justify-center
                 cursor-pointer transition-colors
                 text-neutral-400 dark:text-neutral-500
                 hover:text-neutral-800 dark:hover:text-neutral-200
                 hover:bg-neutral-100 dark:hover:bg-white/8"
        >
          <Icon icon="lucide:x" width="13" />
        </button>
      {/if}
    </div>

    <div class="px-3.5 py-3">
      {#if state.running}
        <!-- ─── En cours ─── -->
        <div class="flex items-baseline justify-between mb-1.5">
          <span class="text-[11px] tabular-nums text-neutral-600 dark:text-neutral-300">
            {state.progress?.done ?? 0} / {state.progress?.total ?? 0}
          </span>
          {#if (state.progress?.failed ?? 0) > 0}
            <span class="text-[11px] text-amber-500">
              {state.progress?.failed}
              {(state.progress?.failed ?? 0) > 1 ? $t('batch.failures') : $t('batch.failure')}
            </span>
          {/if}
        </div>

        <div class="h-1.5 rounded-full overflow-hidden bg-neutral-200 dark:bg-white/10">
          <div
            class="h-full rounded-full bg-emerald-500 transition-[width] duration-200"
            style="width: {percent}%"
          ></div>
        </div>

        {#if state.progress?.current}
          <p class="mt-2 text-[10px] truncate text-neutral-400 dark:text-neutral-500">
            {fileName(state.progress.current)}
          </p>
        {/if}

        <div class="mt-2.5 flex justify-end">
          <button
            type="button"
            onclick={handleCancel}
            disabled={state.cancelling}
            class="px-2.5 h-7 rounded-lg text-[12px] cursor-pointer transition-colors
                   text-neutral-600 dark:text-neutral-300
                   hover:bg-neutral-100 dark:hover:bg-white/8
                   disabled:opacity-50 disabled:cursor-default"
          >
            {state.cancelling ? $t('batch.cancelling') : $t('batch.cancel')}
          </button>
        </div>
      {:else if report}
        <!-- ─── Compte rendu ─── -->
        <div class="flex flex-wrap items-center gap-x-2 gap-y-1 text-[11px]">
          <span class="text-emerald-600 dark:text-emerald-400">
            {report.succeeded} {$t('batch.succeeded')}
          </span>
          {#if report.failures.length}
            <span class="text-neutral-300 dark:text-neutral-600">·</span>
            <span class="text-red-500">
              {report.failures.length}
              {report.failures.length > 1 ? $t('batch.failures') : $t('batch.failure')}
            </span>
          {/if}
          <!-- Un lot annulé doit dire ce qu'il n'a PAS fait : sans ce compte,
               on ignore dans quel état est la bibliothèque. -->
          {#if skipped(report) > 0}
            <span class="text-neutral-300 dark:text-neutral-600">·</span>
            <span class="text-neutral-500 dark:text-neutral-400">
              {skipped(report)} {$t('batch.skipped')}
            </span>
          {/if}
        </div>

        {#if report.failures.length}
          <div class="mt-2 max-h-40 overflow-y-auto scrollbar-none space-y-1">
            {#each report.failures as failure (failure.path)}
              <div
                class="px-2 py-1.5 rounded-lg bg-red-500/8"
                title={failure.path}
              >
                <p class="text-[11px] truncate text-neutral-700 dark:text-neutral-200">
                  {fileName(failure.path)}
                </p>
                <p class="text-[10px] text-red-500 leading-snug">{failure.error}</p>
              </div>
            {/each}
          </div>

          {#if state.retry}
            <div class="mt-2.5 flex justify-end">
              <button
                type="button"
                onclick={handleRetry}
                class="flex items-center gap-1.5 px-2.5 h-7 rounded-lg text-[12px] font-medium
                       cursor-pointer transition-colors
                       bg-emerald-500 text-white hover:bg-emerald-600"
              >
                <Icon icon="lucide:refresh-cw" width="12" />
                {$t('batch.retry_failures')}
              </button>
            </div>
          {/if}
        {/if}
      {/if}
    </div>
  </div>
{/if}
