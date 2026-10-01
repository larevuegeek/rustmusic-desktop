<script lang="ts">
  // Interrupteur en pastille, pour cocher un emplacement parmi plusieurs sur une même ligne.
  import Icon from "@iconify/svelte";

  let {
    label,
    checked,
    onclick,
    disabled = false,
    title,
  }: { label: string; checked: boolean; onclick: () => void; disabled?: boolean; title?: string } = $props();
</script>

<button
  type="button"
  role="switch"
  aria-checked={checked}
  {disabled}
  {title}
  class="h-8 flex items-center gap-1.5 pl-2.5 pr-3 rounded-full border text-[13px] font-semibold whitespace-nowrap transition-colors
         focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--rg-g)
         disabled:cursor-not-allowed disabled:opacity-60
         {checked
           ? 'bg-(--rg-gbg) border-(--rg-gbd) text-(--rg-gtx)'
           : 'border-(--rg-off) text-(--rg-mu) hover:text-(--rg-tx)'}
         {disabled ? '' : 'cursor-pointer'}"
  {onclick}
>
  <Icon icon="material-symbols:check-rounded" width="16" class={checked ? '' : 'opacity-35'} />
  {label}
</button>
