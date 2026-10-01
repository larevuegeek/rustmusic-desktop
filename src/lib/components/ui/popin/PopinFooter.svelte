<script lang="ts">
  // Pied de popin `flush` : état à gauche, Annuler et action principale à droite.
  import Icon from "@iconify/svelte";
  import type { Snippet } from "svelte";

  let {
    start,
    end,
    cancelLabel,
    oncancel,
    cancelDisabled = false,
    submitLabel = "",
    submitIcon = "lucide:check",
    onsubmit,
    submitDisabled = false,
    busy = false,
  }: {
    /** Contenu de gauche (état, compteurs), dans la zone extensible. */
    start?: Snippet;
    /** Contenu juste avant les boutons (raccourci clavier…). */
    end?: Snippet;
    cancelLabel: string;
    oncancel: () => void;
    cancelDisabled?: boolean;
    /** Vide : pas de bouton principal. */
    submitLabel?: string;
    submitIcon?: string;
    onsubmit?: () => void;
    submitDisabled?: boolean;
    /** Roue à la place de l'icône du bouton principal. */
    busy?: boolean;
  } = $props();
</script>

<footer
  class="shrink-0 flex items-center gap-3 px-4 py-2.5
         border-t border-neutral-200/70 dark:border-white/8
         bg-neutral-50/80 dark:bg-black/25"
>
  <div class="flex-1 min-w-0 flex items-center gap-2">
    {@render start?.()}
  </div>

  {@render end?.()}

  <button
    type="button"
    onclick={oncancel}
    disabled={cancelDisabled}
    class="flex items-center gap-1.5 px-3 h-8 rounded-lg text-[13px]
           cursor-pointer transition-colors
           text-neutral-600 dark:text-neutral-300
           hover:bg-neutral-200/70 dark:hover:bg-white/8 disabled:opacity-50"
  >
    <Icon icon="lucide:x" width="13" />
    {cancelLabel}
  </button>
  {#if submitLabel}
    <button
      type="button"
      onclick={onsubmit}
      disabled={submitDisabled}
      class="flex items-center gap-1.5 px-3.5 h-8 rounded-lg text-[13px] font-medium
             cursor-pointer transition-all
             bg-emerald-500 text-white
             hover:bg-emerald-600 hover:shadow-lg hover:shadow-emerald-500/40
             disabled:opacity-35 disabled:cursor-not-allowed disabled:shadow-none"
    >
      {#if busy}
        <Icon icon="lucide:loader-circle" width="13" class="animate-spin" />
      {:else}
        <Icon icon={submitIcon} width="13" />
      {/if}
      {submitLabel}
    </button>
  {/if}
</footer>
