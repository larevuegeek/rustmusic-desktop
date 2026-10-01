<script lang="ts">
  // Choix exclusif en grandes cases : icône ou jauge (niveau 1 à 4) au-dessus de l'intitulé,
  // teintées par `tone` (code couleur d'une échelle de qualité).
  import Icon from "@iconify/svelte";

  type Ton = "green" | "amber" | "sky" | "rose";
  type Option = { value: string; label: string; icon?: string; level?: number; tone?: Ton };

  const BARRES: Record<Ton, string> = {
    green: "bg-green-500", amber: "bg-amber-500", sky: "bg-sky-500", rose: "bg-rose-500",
  };
  const ACTIFS: Record<Ton, string> = {
    green: "bg-green-500/12 ring-1 ring-inset ring-green-500/45",
    amber: "bg-amber-500/12 ring-1 ring-inset ring-amber-500/45",
    sky: "bg-sky-500/12 ring-1 ring-inset ring-sky-500/45",
    rose: "bg-rose-500/12 ring-1 ring-inset ring-rose-500/45",
  };
  const TEXTES: Record<Ton, string> = {
    green: "text-green-600 dark:text-green-400",
    amber: "text-amber-600 dark:text-amber-400",
    sky: "text-sky-600 dark:text-sky-400",
    rose: "text-rose-600 dark:text-rose-400",
  };

  // Sans ton : vert une fois choisi, gris sinon.
  const teinte = (o: Option, actif: boolean) =>
    o.tone ? `${BARRES[o.tone]} ${actif ? "" : "opacity-80"}` : actif ? "bg-(--rg-g)" : "bg-(--rg-mu2)";

  let {
    value,
    options,
    label,
    onchange,
    disabled = false,
  }: { value: string | null; options: Option[]; label: string; onchange: (value: string) => void; disabled?: boolean } = $props();
</script>

<div
  class="grid gap-1 p-1 rounded-xl bg-(--rg-creux) border border-(--rg-line)"
  style:grid-template-columns="repeat({options.length}, minmax(0, 1fr))"
  role="radiogroup"
  aria-label={label}
>
  {#each options as option (option.value)}
    {@const actif = option.value === value}
    <button
      type="button"
      role="radio"
      aria-checked={actif}
      {disabled}
      class="h-14 min-w-0 px-1 flex flex-col items-center justify-center gap-1.5 rounded-[9px]
             text-[13px] font-semibold leading-[1.2] transition-colors disabled:cursor-wait
             focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--rg-g)
             {actif
               ? option.tone ? `${ACTIFS[option.tone]} ${TEXTES[option.tone]}` : 'bg-(--rg-s2) text-(--rg-tx) shadow-[inset_0_0_0_1px_var(--rg-bd2)]'
               : 'text-(--rg-mu) hover:text-(--rg-tx) cursor-pointer'}"
      onclick={() => onchange(option.value)}
    >
      {#if option.icon}
        <Icon icon={option.icon} width="20" class={option.tone ? TEXTES[option.tone] : ""} />
      {:else if option.level}
        <span class="flex items-end gap-0.75 h-4.5" aria-hidden="true">
          {#each [6, 10, 14, 18] as hauteur, i (i)}
            <i
              class="block w-1.5 rounded-[2px]
                     {i >= option.level ? 'bg-(--rg-off)' : teinte(option, actif)}"
              style:height="{hauteur}px"
            ></i>
          {/each}
        </span>
      {/if}
      <span class="max-w-full truncate">{option.label}</span>
    </button>
  {/each}
</div>
