<script lang="ts">
  // Bilan du pied de page : champs modifiés, image touchée, « tout rétablir » et la règle en info-bulle.
  import Icon from "@iconify/svelte";
  import { t } from "#lib/i18n";

  let {
    dirty,
    changedCount,
    extraLabel,
    hint,
    disabled,
    onreset,
  }: {
    dirty: boolean;
    changedCount: number;
    /** Modification hors champs (médias, pochette) ; vide si aucune. */
    extraLabel: string;
    hint: string;
    disabled: boolean;
    onreset: () => void;
  } = $props();
</script>

{#if dirty}
  <span class="flex items-center gap-1.5 text-[11px] text-neutral-600 dark:text-neutral-300">
    <span class="w-1.5 h-1.5 rounded-full bg-emerald-500"></span>
    {#if changedCount}
      {changedCount}
      {changedCount > 1 ? $t('tags.changed_other') : $t('tags.changed_one')}
    {/if}
    {#if changedCount && extraLabel}·{/if}
    {#if extraLabel}{extraLabel}{/if}
  </span>
  <button
    type="button"
    onclick={onreset}
    {disabled}
    class="text-[11px] cursor-pointer underline decoration-dotted underline-offset-2
           text-neutral-400 dark:text-neutral-500
           hover:text-neutral-700 dark:hover:text-neutral-200
           disabled:opacity-40 transition-colors"
  >
    {$t('tags.reset_all')}
  </button>
{:else}
  <span class="text-[11px] text-neutral-400 dark:text-neutral-500">
    {$t('tags.no_change')}
  </span>
{/if}

<!-- Une règle qui se lit une fois : en info-bulle, elle ne prend pas la place. -->
<span
  title={hint}
  class="text-neutral-300 dark:text-neutral-600 cursor-help
         hover:text-neutral-500 dark:hover:text-neutral-400 transition-colors"
>
  <Icon icon="lucide:info" width="12" />
</span>
