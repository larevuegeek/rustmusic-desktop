<script lang="ts">
  // Champ de saisie habillé comme les réglages ; `onchange` à la sortie du champ ou sur Entrée.
  let {
    value,
    label,
    onchange,
    type = "text",
    mono = false,
    class: classe = "w-56",
    min,
    max,
    maxlength,
  }: {
    value: string | number;
    label: string;
    onchange: (value: string) => void;
    type?: "text" | "number";
    mono?: boolean;
    class?: string;
    min?: number;
    max?: number;
    maxlength?: number;
  } = $props();
</script>

<input
  {type}
  {value}
  {min}
  {max}
  {maxlength}
  aria-label={label}
  class="shrink-0 h-9.5 px-3 rounded-[10px] outline-none transition-colors
         bg-(--rg-s2) border border-(--rg-off) hover:border-(--rg-bd2) focus:border-(--rg-g)
         text-sm font-semibold text-(--rg-tx) {mono ? 'font-mono tabular-nums text-right' : ''} {classe}"
  onblur={(e) => onchange(e.currentTarget.value)}
  onkeydown={(e) => { if (e.key === "Enter") e.currentTarget.blur(); }}
/>
