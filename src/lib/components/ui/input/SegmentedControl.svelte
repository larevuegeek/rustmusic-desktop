<script lang="ts">
  // Choix exclusif en ligne. `plein` (réglages) : option active en pastille contrastée ; `discret` (barres d'outils) : fond doux, icônes possibles.
  import Icon from "@iconify/svelte";

  let {
    value,
    options,
    label,
    onchange,
    variant = "plein",
    class: classes = "",
  }: {
    value: string | null;
    /** `iconOnly` : bouton carré, `label` en info-bulle. `after` : icône après le libellé (sens de tri). */
    options: { value: string; label: string; icon?: string; iconOnly?: boolean; after?: string | null }[];
    label: string;
    onchange: (value: string) => void;
    variant?: "plein" | "discret";
    class?: string;
  } = $props();

  const discret = $derived(variant === "discret");
</script>

<div
  class="shrink-0 flex p-0.75 rounded-[10px] border {discret ? 'bg-(--rg-carte) border-(--rg-bd)' : 'bg-(--rg-s2) border-(--rg-off)'} {classes}"
  role="radiogroup"
  aria-label={label}
>
  {#each options as option (option.value)}
    {@const actif = option.value === value}
    <button
      type="button"
      role="radio"
      aria-checked={actif}
      title={option.iconOnly ? option.label : undefined}
      aria-label={option.iconOnly ? option.label : undefined}
      class="flex items-center justify-center gap-1 rounded-[7px] text-[13px] font-semibold cursor-pointer transition-colors whitespace-nowrap
             focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--rg-g)
             {discret ? (option.iconOnly ? 'w-8 h-7' : 'h-7 px-3') : 'h-8 px-3.5'}
             {actif ? (discret ? 'bg-(--rg-s2) text-(--rg-tx)' : 'bg-(--rg-tx) text-(--rg-bg)') : 'text-(--rg-mu) hover:text-(--rg-tx)'}"
      onclick={() => onchange(option.value)}
    >
      {#if option.icon}<Icon icon={option.icon} width="18" />{/if}
      {#if !option.iconOnly}{option.label}{/if}
      {#if option.after}<Icon icon={option.after} width="14" />{/if}
    </button>
  {/each}
</div>
