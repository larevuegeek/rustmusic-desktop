<script lang="ts">
  // Confirmation d'une action destructrice : le bouton ne s'arme qu'une fois le mot tapé à l'identique.
  import Icon from "@iconify/svelte";
  import type { Snippet } from "svelte";
  import { fade, scale } from "svelte/transition";
  import { t } from "#lib/i18n";
  import FormInput from "#lib/components/ui/input/FormInput.svelte";
  import PopinFooter from "#lib/components/ui/popin/PopinFooter.svelte";

  let { open = $bindable(false), title, confirmLabel, onconfirm, children }: {
    open: boolean;
    title: string;
    confirmLabel: string;
    /** L'action ; la boîte se ferme quand elle aboutit. */
    onconfirm: () => Promise<void> | void;
    /** Ce que l'action emporte. */
    children: Snippet;
  } = $props();

  let busy = $state(false);
  let typed = $state("");
  const word = $derived($t("settings.reset_confirm_word"));
  const locked = $derived(busy || typed !== word);

  // Rouverte : la saisie repart de zéro.
  $effect(() => {
    if (open) typed = "";
  });

  async function confirm() {
    if (locked) return;
    busy = true;
    try {
      await onconfirm();
      open = false;
    } catch (e) {
      // Échec : la boîte reste ouverte (l'appelant a déjà prévenu s'il le faut).
      console.error("[confirmation]", e);
    } finally {
      busy = false;
    }
  }

  const cancel = () => {
    if (!busy) open = false;
  };

  // Échap annule, Entrée confirme si le mot est bon.
  function onKeydown(e: KeyboardEvent) {
    if (!open) return;
    if (e.key === "Escape") cancel();
    else if (e.key === "Enter") confirm();
  }
</script>

<svelte:window onkeydown={onKeydown} />

{#if open}
  <div class="fixed inset-0 z-50 flex items-center justify-center" transition:fade={{ duration: 150 }}>
    <button type="button" class="absolute inset-0 bg-black/60 backdrop-blur-sm cursor-default" onclick={cancel} aria-label={$t("common.close")}></button>

    <div
      role="alertdialog"
      aria-modal="true"
      aria-label={title}
      class="relative w-full max-w-md mx-4 max-h-[88vh] flex flex-col
             rounded-2xl overflow-hidden text-(--rg-tx)
             bg-(--rg-carte) border border-(--rg-bd) dark:border-[#2a312d]
             shadow-[0_24px_60px_-12px_rgba(0,0,0,0.25)] dark:shadow-[0_30px_80px_-20px_rgba(0,0,0,0.9)]"
      transition:scale={{ duration: 180, start: 0.96 }}
    >
      <!-- En-tête teinté rouge : action destructrice -->
      <div class="shrink-0 flex items-center justify-between gap-3 px-6 pt-5 pb-4 border-b border-(--rg-bd)
                  bg-linear-135 from-red-500/8 to-transparent to-60%">
        <h3 class="flex items-center gap-2.5 min-w-0 text-[17px] font-extrabold tracking-[-0.01em] text-(--rg-tx)">
          <span class="shrink-0 w-8 h-8 rounded-[9px] flex items-center justify-center
                       bg-red-500/8 border border-red-500/25 text-red-600 dark:text-red-400">
            <Icon icon="material-symbols:warning-outline-rounded" width="16" />
          </span>
          <span class="truncate">{title}</span>
        </h3>
        <button
          type="button"
          class="shrink-0 -mr-1.5 w-8 h-8 rounded-[9px] flex items-center justify-center
                 cursor-pointer text-(--rg-mu) hover:text-(--rg-tx) hover:bg-(--rg-hover) transition-colors"
          aria-label={$t("common.close")}
          onclick={cancel}
        >
          <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round">
            <path d="M18 6 6 18M6 6l12 12" />
          </svg>
        </button>
      </div>

      <div class="flex-1 min-h-0 overflow-y-auto scrollbar-none px-6 pt-5 pb-6 space-y-5 text-sm">
        {@render children()}

        <div class="flex flex-col gap-2">
          <label for="typed_confirm_input" class="text-xs font-semibold text-(--rg-tx2)">
            {$t("settings.reset_confirm_label").replace("{word}", word)}
          </label>
          <FormInput id="typed_confirm_input" bind:value={typed} placeholder={word} invalid={!!typed && typed !== word} disabled={busy} />
        </div>
      </div>

      <PopinFooter
        cancelLabel={$t("common.cancel")}
        oncancel={cancel}
        cancelDisabled={busy}
        submitLabel={confirmLabel}
        submitIcon="material-symbols:delete-outline-rounded"
        onsubmit={confirm}
        submitDisabled={locked}
        {busy}
        tone="danger"
      />
    </div>
  </div>
{/if}
