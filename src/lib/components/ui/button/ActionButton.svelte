<script lang="ts">
  // Bouton d'action générique (rescan, fetch, reset…).
  // `busy` affiche un spinner et désactive le bouton.
  import Icon from "@iconify/svelte";

  let {
    icon,
    label,
    onclick,
    busy = false,
    danger = false,
  }: {
    icon: string;
    label: string;
    onclick: () => void;
    busy?: boolean;
    danger?: boolean;
  } = $props();
</script>

<button
  class="px-3 py-1.5 rounded-lg text-xs font-medium cursor-pointer transition-colors shrink-0
         disabled:opacity-50
         {danger
           ? 'border border-red-200 dark:border-red-800/50 text-red-500 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/20'
           : 'border border-neutral-200 dark:border-neutral-700 text-neutral-600 dark:text-neutral-400 hover:bg-neutral-100 dark:hover:bg-neutral-800'}"
  disabled={busy}
  {onclick}
>
  <Icon icon={busy ? 'lucide:loader-2' : icon} width="14" class="inline mr-1 {busy ? 'animate-spin' : ''}" />
  {label}
</button>
