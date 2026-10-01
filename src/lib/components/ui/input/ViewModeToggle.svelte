<script lang="ts">
  import { t } from "$lib/i18n";
  import Icon from "@iconify/svelte";
  import { viewMode } from "$lib/stores/ui/viewMode.store";
  import { alphabetNavVisible } from "$lib/stores/ui/alphabetNav.store";

  let { showAlphabet = false }: { showAlphabet?: boolean } = $props();
</script>

<div class="flex items-center gap-1.5">
  <!-- View mode toggle -->
  <div class="flex items-center bg-neutral-100 dark:bg-white/5 rounded-lg p-px">
    <button
      class="p-1.5 rounded-md cursor-pointer transition-all duration-150
             {$viewMode === 'grid'
               ? 'bg-white dark:bg-neutral-700 shadow-sm text-neutral-800 dark:text-neutral-200'
               : 'text-neutral-400 hover:text-neutral-600 dark:hover:text-neutral-300'}"
      onclick={() => viewMode.set('grid')}
      aria-label={$t("albums_view.grid")}
    >
      <Icon icon="lucide:layout-grid" width="14" />
    </button>
    <button
      class="p-1.5 rounded-md cursor-pointer transition-all duration-150
             {$viewMode === 'list'
               ? 'bg-white dark:bg-neutral-700 shadow-sm text-neutral-800 dark:text-neutral-200'
               : 'text-neutral-400 hover:text-neutral-600 dark:hover:text-neutral-300'}"
      onclick={() => viewMode.set('list')}
      aria-label={$t("albums_view.list")}
    >
      <Icon icon="lucide:list" width="14" />
    </button>
  </div>

  <!-- Alphabet nav toggle -->
  {#if showAlphabet}
    <div class="flex items-center bg-neutral-100 dark:bg-white/5 rounded-lg p-px">
      <button
        class="p-1.5 rounded-md cursor-pointer transition-all duration-150
               {$alphabetNavVisible
                 ? 'bg-white dark:bg-neutral-700 shadow-sm text-emerald-600 dark:text-emerald-400'
                 : 'text-neutral-400 hover:text-neutral-600 dark:hover:text-neutral-300'}"
        onclick={() => alphabetNavVisible.update(v => !v)}
        aria-label={$t("common.alpha_nav")}
        title={$t("common.alpha_nav")}
      >
        <Icon icon="lucide:a-arrow-down" width="14" />
      </button>
    </div>
  {/if}
</div>
