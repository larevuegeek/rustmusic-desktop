<script lang="ts">
  // Champ de saisie d'un tag.
  //
  // Extrait pour que l'éditeur d'un fichier et celui d'une sélection partagent
  // exactement le même rendu : deux copies du style dériveraient à la première
  // retouche.
  //
  // Un bloc à fond plein, **sans trait de contour** : c'est le fond qui
  // délimite le champ. Les traits, eux, s'accumulent — douze cadres empilés
  // donnent une grille de formulaire administratif.
  //
  // Le focus n'ajoute jamais de rectangle : fond teinté, intitulé coloré, halo
  // diffus. `data-focus-ring` déplace vers ce bloc l'indicateur de contraste
  // élevé, qui sinon dessinerait un cadre dur autour de la seule saisie
  // (voir `app.css`).
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
  class="group relative rounded-lg overflow-hidden transition-all duration-150
         bg-neutral-100/70 dark:bg-white/4
         hover:bg-neutral-200/50 dark:hover:bg-white/6
         focus-within:bg-emerald-500/10
         focus-within:shadow-lg focus-within:shadow-emerald-500/25"
>
  {#if dirty}
    <span
      class="absolute left-0 inset-y-0 w-0.75 bg-emerald-500"
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
      class="block text-[10px] font-semibold uppercase tracking-[0.09em] leading-none
             text-neutral-400 dark:text-neutral-500
             group-focus-within:text-emerald-600 dark:group-focus-within:text-emerald-400
             transition-colors"
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
               outline-none
               text-neutral-900 dark:text-neutral-50
               placeholder:text-neutral-300 dark:placeholder:text-neutral-700
               disabled:opacity-50"
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
               text-neutral-900 dark:text-neutral-50
               placeholder:text-neutral-300 dark:placeholder:text-neutral-700
               disabled:opacity-50
               {large ? 'text-[15px] font-medium' : 'text-[13px]'}
               {dirty ? 'pr-8' : ''}"
      />
    {/if}

    {#if warning}
      <span class="mt-1 flex items-center gap-1 text-[10px] text-amber-500">
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
      class="absolute right-2 w-6 h-6 rounded-md cursor-pointer
             flex items-center justify-center
             text-neutral-400 dark:text-neutral-500
             hover:text-emerald-600 dark:hover:text-emerald-400
             hover:bg-emerald-500/15 transition-colors
             {multiline ? 'top-1.5' : 'top-1/2 -translate-y-1/2'}"
    >
      <Icon icon="lucide:rotate-ccw" width="12" />
    </button>
  {/if}
</div>
