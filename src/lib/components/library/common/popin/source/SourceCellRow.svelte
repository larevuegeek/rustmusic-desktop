<script lang="ts">
  // Une valeur dans le détail d'un morceau : ce qui diffère se coche, le reste se constate.
  // `absent` reste distinct de `same` : Deezer ne confirme pas ce qu'il ignore.
  import Icon from "@iconify/svelte";
  import { t } from "$lib/i18n";
  import type { Cell } from "./sourceSelection.svelte.js";

  const { cell, on, ontoggle }: { cell: Cell; on: boolean; ontoggle: () => void } = $props();
</script>

<div
  class="flex items-center gap-2.5 px-2 py-1 rounded-md
         {on ? 'bg-emerald-500/10' : ''}
         {cell.state === 'differs' ? '' : 'opacity-55'}"
>
  {#if cell.state === 'differs'}
    <input
      type="checkbox"
      checked={on}
      onchange={() => ontoggle()}
      aria-label={$t(`tags.${cell.field}`)}
      class="checkbox-app"
    />
  {:else}
    <span class="shrink-0 w-4 flex items-center justify-center">
      <Icon
        icon={cell.state === 'same' ? 'lucide:check' : 'lucide:minus'}
        width="10"
        class="text-neutral-300 dark:text-neutral-600"
      />
    </span>
  {/if}

  <span class="shrink-0 w-24 text-[11px] truncate
               text-neutral-500 dark:text-neutral-400">
    {$t(`tags.${cell.field}`)}
  </span>

  <span class="flex-1 min-w-0 text-[11.5px] truncate
               {cell.from
                 ? 'text-neutral-600 dark:text-neutral-300'
                 : 'italic text-neutral-400 dark:text-neutral-600'}"
        title={cell.from}>
    {cell.from || $t('workshop.was_empty')}
  </span>

  <Icon
    icon="lucide:arrow-right"
    width="10"
    class="shrink-0 {cell.state === 'differs'
      ? 'text-neutral-300 dark:text-neutral-600'
      : 'text-transparent'}"
  />

  <span class="flex-1 min-w-0 text-[11.5px] truncate
               {cell.state === 'differs'
                 ? on
                   ? 'font-medium text-emerald-600 dark:text-emerald-400'
                   : 'text-neutral-500 dark:text-neutral-400'
                 : cell.state === 'same'
                   ? 'text-neutral-400 dark:text-neutral-500'
                   : 'italic text-neutral-400 dark:text-neutral-600'}"
        title={cell.to}>
    {cell.state === 'absent' ? $t('source.not_provided') : cell.to}
  </span>
</div>
