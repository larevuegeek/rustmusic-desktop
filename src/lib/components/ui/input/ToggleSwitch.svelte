<script lang="ts">
  // Interrupteur on/off générique.
  // `color` : accent quand actif (green = défaut, amber, purple, emerald).
  let {
    checked,
    onclick,
    label,
    color = 'green',
    disabled = false,
  }: {
    checked: boolean;
    onclick: () => void;
    label: string;
    color?: 'green' | 'amber' | 'purple' | 'emerald';
    disabled?: boolean;
  } = $props();

  const activeColors = {
    green: 'bg-green-500',
    amber: 'bg-amber-500',
    purple: 'bg-purple-500',
    emerald: 'bg-emerald-500',
  };
</script>

<button
  class="relative w-10 h-6 rounded-full cursor-pointer transition-colors duration-200 shrink-0
         {checked ? activeColors[color] : 'bg-neutral-300 dark:bg-neutral-700'}
         {disabled ? 'opacity-50 cursor-wait' : ''}"
  aria-label={label}
  {disabled}
  {onclick}
>
  <div class="absolute top-0.5 left-0.5 w-5 h-5 rounded-full bg-white shadow transition-transform duration-200
              {checked ? 'translate-x-4' : ''}"></div>
</button>
