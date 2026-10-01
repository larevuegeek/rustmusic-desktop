<script lang="ts">
  // Bouton de barre d'outils : icône, libellé facultatif (sinon carré), état enfoncé en vert.
  import Icon from "@iconify/svelte";

  let {
    icon,
    label,
    title,
    pressed,
    disabled = false,
    labelClass = "",
    onclick,
  }: {
    icon: string;
    label?: string;
    title?: string;
    /** `undefined` : bouton simple ; booléen : bouton bascule. */
    pressed?: boolean;
    disabled?: boolean;
    /** Ex. `@max-xl:hidden` : le libellé disparaît quand la place manque. */
    labelClass?: string;
    onclick: () => void;
  } = $props();
</script>

<button type="button" aria-pressed={pressed} title={title ?? label} aria-label={label ? undefined : title} {disabled} {onclick}
        class="shrink-0 flex items-center justify-center gap-1.5 rounded-[10px] border text-[13px] font-semibold whitespace-nowrap cursor-pointer transition-colors
               disabled:opacity-50 disabled:cursor-default
               {label ? 'h-8.5 pl-2.5 pr-3' : 'w-9 h-9'}
               {pressed ? 'bg-(--rg-gbg) border-(--rg-gbd) text-(--rg-gtx)' : 'bg-(--rg-carte) border-(--rg-bd) text-(--rg-tx2) hover:border-(--rg-bd2) hover:text-(--rg-tx)'}">
  <Icon {icon} width={label ? 18 : 20} />
  {#if label}<span class={labelClass}>{label}</span>{/if}
</button>
