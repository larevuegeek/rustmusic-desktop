<script lang="ts">
  // Bandeau ambre de confirmation en bas de popin : message, annuler, confirmer.
  import Icon from "@iconify/svelte";
  import { fade } from "svelte/transition";
  import type { Snippet } from "svelte";

  const {
    open,
    busy = false,
    cancelLabel,
    confirmLabel,
    oncancel,
    onconfirm,
    children,
  }: {
    open: boolean;
    /** Action en cours : boutons désactivés, roue sur la confirmation. */
    busy?: boolean;
    cancelLabel: string;
    confirmLabel: string;
    oncancel: () => void;
    onconfirm: () => void;
    children: Snippet;
  } = $props();
</script>

{#if open}
  <div class="shrink-0 flex items-center gap-3 px-4 py-2.5
              bg-amber-500/10 border-t border-amber-500/25"
       transition:fade={{ duration: 120 }}>
    <Icon icon="lucide:triangle-alert" width="14" class="shrink-0 text-amber-500" />
    <span class="flex-1 min-w-0 text-[11.5px] text-amber-700 dark:text-amber-300">
      {@render children()}
    </span>
    <button
      type="button"
      onclick={oncancel}
      disabled={busy}
      class="px-3 h-7 rounded-lg text-[12px] cursor-pointer transition-colors
             text-amber-700 dark:text-amber-300 hover:bg-amber-500/15
             disabled:opacity-50"
    >
      {cancelLabel}
    </button>
    <button
      type="button"
      onclick={onconfirm}
      disabled={busy}
      class="flex items-center gap-1.5 px-3 h-7 rounded-lg text-[12px] font-medium
             cursor-pointer transition-colors
             bg-amber-500 text-white hover:bg-amber-600 disabled:opacity-50"
    >
      {#if busy}
        <Icon icon="lucide:loader-circle" width="12" class="animate-spin" />
      {/if}
      {confirmLabel}
    </button>
  </div>
{/if}
