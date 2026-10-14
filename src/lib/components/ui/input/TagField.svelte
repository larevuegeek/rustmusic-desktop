<script lang="ts">
  // Champ de saisie d'un tag.
  //
  // Extrait pour que l'éditeur d'un fichier et celui d'une sélection partagent
  // exactement le même rendu : deux copies du style dériveraient à la première
  // retouche.
  //
  // Un bloc de saisie à filet discret, comme les champs des popins : le libellé
  // vit dedans, ce qui évite une ligne de plus par champ.
  //
  // Le focus n'ajoute jamais de rectangle : filet et intitulé à l'accent.
  // `data-focus-ring` déplace vers ce bloc l'indicateur de contraste élevé, qui
  // sinon dessinerait un cadre dur autour de la seule saisie (voir `app.css`).
  import Icon from "@iconify/svelte";
  import { fade } from "svelte/transition";
  import type { Snippet } from "svelte";

  let {
    label,
    value = $bindable(""),
    placeholder = "—",
    dirty = false,
    large = false,
    disabled = false,
    multiline = false,
    /** Averti d'un effet qu'on ne verrait pas autrement — « effacera N valeurs ». */
    warning = "",
    revertLabel = "",
    onrevert = undefined,
    oninput = undefined,
    /**
     * Contenu de saisie sur mesure, à la place du champ standard.
     *
     * Sert au couple « n / total », qui tient deux saisies dans un seul bloc
     * parce que l'ID3 n'en stocke qu'une (« 7/12 »). Sans cette échappatoire
     * il faudrait recopier tout l'habillage ailleurs, et les deux copies
     * dériveraient à la première retouche.
     */
    children = undefined,
  }: {
    label: string;
    value?: string;
    placeholder?: string;
    dirty?: boolean;
    large?: boolean;
    disabled?: boolean;
    multiline?: boolean;
    warning?: string;
    revertLabel?: string;
    onrevert?: (() => void) | undefined;
    /** Reçoit l'événement : certains appelants ont besoin de la valeur saisie. */
    oninput?: ((event: Event) => void) | undefined;
    children?: Snippet | undefined;
  } = $props();
</script>

<div
  data-focus-ring="row"
  class="group relative rounded-xl overflow-hidden transition-colors duration-150
         bg-(--rg-champ) border border-(--rg-bd)
         hover:border-(--rg-bd2) focus-within:border-(--rg-g) focus-within:hover:border-(--rg-g)
         focus-within:bg-[color-mix(in_srgb,white_5%,var(--rg-champ))]"
>
  {#if dirty}
    <span
      class="absolute left-0 inset-y-0 w-0.75 bg-(--rg-g)"
      transition:fade={{ duration: 120 }}
    ></span>
  {/if}

  <!-- Un `<label>` autour d'une saisie sur mesure ferait porter le clic à sa
       première case, ce qui n'est pas le geste attendu quand il y en a deux. -->
  <svelte:element
    this={children ? 'div' : 'label'}
    class="block px-3.5 py-2 {children ? '' : 'cursor-text'}"
  >
    <span
      class="block text-[10px] font-bold uppercase tracking-[0.1em] leading-none
             text-(--rg-mu) group-focus-within:text-(--rg-g) transition-colors"
    >
      {label}
    </span>

    {#if children}
      {@render children()}
    {:else if multiline}
      <textarea
        rows="2"
        data-focus-ring="none"
        bind:value
        {disabled}
        {placeholder}
        {oninput}
        class="w-full mt-1.5 bg-transparent text-[13px] leading-snug resize-none scrollbar-none
               outline-none text-(--rg-tx) placeholder:text-(--rg-ph) disabled:opacity-50"
      ></textarea>
    {:else}
      <input
        type="text"
        data-focus-ring="none"
        bind:value
        {disabled}
        {placeholder}
        {oninput}
        class="w-full mt-1.5 bg-transparent leading-tight outline-none
               text-(--rg-tx) placeholder:text-(--rg-ph) disabled:opacity-50
               {large ? 'text-[15px] font-semibold' : 'text-[13px]'}
               {dirty ? 'pr-8' : ''}"
      />
    {/if}

    {#if warning}
      <span class="mt-1 flex items-center gap-1 text-[10px] font-medium text-(--rg-am)">
        <Icon icon="lucide:triangle-alert" width="10" class="shrink-0" />
        {warning}
      </span>
    {/if}
  </svelte:element>

  {#if dirty && onrevert}
    <button
      type="button"
      onclick={onrevert}
      title={revertLabel}
      aria-label={revertLabel}
      class="absolute right-2 w-6 h-6 rounded-lg cursor-pointer
             flex items-center justify-center transition-colors
             text-(--rg-mu2) hover:text-(--rg-g) hover:bg-(--rg-gbg)
             {multiline ? 'top-1.5' : 'top-1/2 -translate-y-1/2'}"
    >
      <Icon icon="lucide:rotate-ccw" width="12" />
    </button>
  {/if}
</div>
