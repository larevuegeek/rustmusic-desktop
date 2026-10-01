<script lang="ts">
  // Interrupteur on/off générique.
  // `color` : accent quand actif (green = défaut, amber, purple, emerald).
  let {
    checked,
    onclick,
    label,
    color = 'green',
    disabled = false,
    size = 'md',
  }: {
    checked: boolean;
    onclick: () => void;
    label: string;
    color?: 'green' | 'amber' | 'purple' | 'emerald';
    disabled?: boolean;
    /** `sm` : menus et listes serrées. */
    size?: 'md' | 'sm';
  } = $props();

  const activeColors = {
    green: 'bg-(--rg-g)',
    amber: 'bg-amber-500',
    purple: 'bg-purple-500',
    emerald: 'bg-emerald-500',
  };
</script>

<button
  type="button"
  role="switch"
  aria-checked={checked}
  class="relative {size === 'sm' ? 'w-8.5 h-5' : 'w-11 h-6.5'} rounded-full cursor-pointer transition-colors duration-150 shrink-0
         focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--rg-g)
         {checked ? activeColors[color] : 'bg-(--rg-off)'}
         disabled:opacity-35 disabled:cursor-not-allowed"
  aria-label={label}
  {disabled}
  {onclick}
>
  <span class="absolute top-0.75 left-0.75 rounded-full bg-white shadow-[0_1px_3px_rgba(0,0,0,0.4)] transition-transform duration-150
               {size === 'sm' ? 'w-3.5 h-3.5' : 'w-5 h-5'} {checked ? (size === 'sm' ? 'translate-x-3.5' : 'translate-x-4.5') : ''}"></span>
</button>
