<script lang="ts">
  // Option d'un choix exclusif illustré : aperçu, intitulé et coche ronde.
  // À placer dans un conteneur `role="radiogroup"`.
  import type { Snippet } from "svelte";
  import RadioCheck from "./RadioCheck.svelte";

  let {
    label,
    selected,
    onclick,
    title,
    children,
  }: { label: string; selected: boolean; onclick: () => void; title?: string; children: Snippet } = $props();
</script>

<button
  type="button"
  role="radio"
  aria-checked={selected}
  {title}
  class="min-w-0 flex flex-col gap-2.5 pt-2.5 px-2.5 pb-3 rounded-xl border text-left cursor-pointer transition-colors
         focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--rg-g)
         {selected ? 'border-(--rg-g) bg-(--rg-creux-on)' : 'border-(--rg-bd) bg-(--rg-creux) hover:border-(--rg-bd2)'}"
  {onclick}
>
  {@render children()}
  <span class="flex items-center justify-between gap-2 px-0.5 text-sm font-semibold leading-[1.2] text-(--rg-tx)">
    <span class="truncate">{label}</span>
    <RadioCheck checked={selected} />
  </span>
</button>
