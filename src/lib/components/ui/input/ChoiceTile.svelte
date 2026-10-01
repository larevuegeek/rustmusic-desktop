<script lang="ts">
  // Option d'un choix exclusif décrit en toutes lettres : titre, coche, puis explication.
  // À placer dans un conteneur `role="radiogroup"`.
  import RadioCheck from "./RadioCheck.svelte";

  let {
    title,
    desc,
    selected,
    onclick,
  }: { title: string; desc?: string; selected: boolean; onclick: () => void } = $props();
</script>

<button
  type="button"
  role="radio"
  aria-checked={selected}
  class="min-w-0 flex flex-col gap-1.5 px-4 py-3.5 rounded-xl border text-left cursor-pointer transition-colors
         focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--rg-g)
         {selected ? 'border-(--rg-g) bg-(--rg-creux-on)' : 'border-(--rg-bd) bg-(--rg-creux) hover:border-(--rg-bd2)'}"
  {onclick}
>
  <span class="flex items-center justify-between gap-2">
    <span class="text-[15px] font-semibold leading-[1.2] text-(--rg-tx)">{title}</span>
    <RadioCheck checked={selected} />
  </span>
  {#if desc}<span class="text-[13px] leading-[1.4] text-(--rg-mu) text-pretty">{desc}</span>{/if}
</button>
