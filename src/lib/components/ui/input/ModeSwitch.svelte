<script lang="ts">
  // Bascule de rayon (Artistes / Compositeurs…) : icône, libellé, nombre ; la pastille active glisse d'un choix à l'autre.
  import Icon from "@iconify/svelte";

  let {
    value,
    options,
    label,
    onchange,
    labelClass = "",
    class: classes = "",
  }: {
    value: string;
    options: { value: string; label: string; icon: string; count?: string }[];
    label: string;
    onchange: (value: string) => void;
    /** Classes du libellé et du nombre (les masquer quand la place manque). */
    labelClass?: string;
    class?: string;
  } = $props();

  let zone = $state<HTMLDivElement | null>(null);
  let pastille = $state<{ x: number; w: number } | null>(null);
  let animee = $state(false);

  // Position mesurée : suit le choix et la largeur des libellés (masqués ou non).
  function measure() {
    const b = zone?.querySelector<HTMLElement>('[aria-checked="true"]');
    if (b) pastille = { x: b.offsetLeft, w: b.offsetWidth };
  }
  $effect(() => {
    void value;
    measure();
  });
  $effect(() => {
    if (!zone) return;
    const ro = new ResizeObserver(measure);
    ro.observe(zone);
    // Pas de glissement à l'apparition, seulement aux changements.
    requestAnimationFrame(() => (animee = true));
    return () => ro.disconnect();
  });
</script>

<div bind:this={zone} class="relative shrink-0 flex h-9.5 p-0.75 rounded-[11px] border bg-(--rg-carte) border-(--rg-bd) {classes}" role="radiogroup" aria-label={label}>
  {#if pastille}
    <span
      class="absolute top-0.75 bottom-0.75 left-0 rounded-[8px] border bg-(--rg-gbg) border-(--rg-gbd) shadow-[0_1px_6px_rgba(0,0,0,0.12)] pointer-events-none
             {animee ? 'transition-[transform,width] duration-300 ease-[cubic-bezier(0.32,0.72,0,1)]' : ''}"
      style="transform: translateX({pastille.x}px); width: {pastille.w}px"
      aria-hidden="true"
    ></span>
  {/if}
  {#each options as option (option.value)}
    {@const actif = option.value === value}
    <button
      type="button"
      role="radio"
      aria-checked={actif}
      title={option.label}
      class="relative z-1 flex items-center gap-2 h-full px-3 rounded-[8px] text-[13px] font-semibold whitespace-nowrap cursor-pointer transition-colors duration-200
             focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--rg-g)
             {actif ? 'text-(--rg-gtx)' : 'text-(--rg-mu) hover:text-(--rg-tx)'}"
      onclick={() => onchange(option.value)}
    >
      <Icon icon={option.icon} width="18" class="shrink-0 transition-transform duration-300 {actif ? 'scale-110' : ''}" />
      <span class={labelClass}>{option.label}</span>
      {#if option.count}
        <span class="min-w-6 px-1.5 py-px rounded-full text-[11px] font-bold tabular-nums transition-colors duration-200
                     {actif ? 'bg-(--rg-g) text-(--rg-on-g)' : 'bg-(--rg-s2) text-(--rg-mu)'} {labelClass}">{option.count}</span>
      {/if}
    </button>
  {/each}
</div>
