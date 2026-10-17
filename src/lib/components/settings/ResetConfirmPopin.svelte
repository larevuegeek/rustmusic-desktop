<script lang="ts">
  import Icon from "@iconify/svelte";
  import { fade, scale } from "svelte/transition";
  import { invoke } from "@tauri-apps/api/core";
  import { profilSelector } from "#lib/stores/profil/profil.store";
  import { libraryStore } from "#lib/stores/library/library.store";
  import { playlistStore } from "#lib/stores/playlist/playlist.store";
  import { liked } from "#lib/stores/playlist/like.store";
  import { goto } from "$app/navigation";
  import { t } from "#lib/i18n";
  import FormInput from "#lib/components/ui/input/FormInput.svelte";
  import PopinFooter from "#lib/components/ui/popin/PopinFooter.svelte";

  let { open = $bindable(true) }: { open: boolean } = $props();
  let loading = $state(false);
  let saisie = $state("");

  const motConfirmation = $derived($t("settings.reset_confirm_word"));
  // Phrase coupée autour de son mot en gras ({b}).
  const boldParts = (cle: string) => [...$t(cle).split("{b}"), ""];
  const ELEMENTS = ["libraries", "playlists", "liked", "settings", "thumbnails"];
  // Le bouton ne s'arme qu'une fois le mot tapé à l'identique.
  const confirmationBloquee = $derived(loading || saisie !== motConfirmation);

  async function handleConfirm() {
    loading = true;
    try {
      await invoke('reset_application');
      open = false;

      // Relancer l'app — la DB sera recréée par les migrations au démarrage
      try {
        const { relaunch } = await import('@tauri-apps/plugin-process');
        await relaunch();
      } catch {
        // En mode dev, relaunch peut échouer — on ferme simplement
        const { getCurrentWindow } = await import('@tauri-apps/api/window');
        await getCurrentWindow().close();
      }
    } catch (e) {
      console.error('Reset failed:', e);
    } finally {
      loading = false;
    }
  }

  function handleCancel() {
    open = false;
  }

  function confirm() {
    if (!confirmationBloquee) handleConfirm();
  }

  // Échap annule, Entrée confirme si le mot est bon.
  function onKeydown(e: KeyboardEvent) {
    if (!open) return;
    if (e.key === "Escape") handleCancel();
    else if (e.key === "Enter") confirm();
  }
</script>

<svelte:window onkeydown={onKeydown} />

{#if open}
  <div class="fixed inset-0 z-50 flex items-center justify-center" transition:fade={{ duration: 150 }}>
    <button
      type="button"
      class="absolute inset-0 bg-black/60 backdrop-blur-sm cursor-default"
      onclick={handleCancel}
      aria-label={$t("common.close")}
    ></button>

    <div
      role="alertdialog"
      aria-modal="true"
      aria-label={$t("settings.reset")}
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
          <span class="truncate">{$t("settings.reset")}</span>
        </h3>
        <button
          type="button"
          class="shrink-0 -mr-1.5 w-8 h-8 rounded-[9px] flex items-center justify-center
                 cursor-pointer text-(--rg-mu) hover:text-(--rg-tx) hover:bg-(--rg-hover) transition-colors"
          aria-label={$t("common.close")}
          onclick={handleCancel}
        >
          <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round">
            <path d="M18 6 6 18M6 6l12 12" />
          </svg>
        </button>
      </div>

      <div class="flex-1 min-h-0 overflow-y-auto scrollbar-none px-6 pt-5 pb-6 space-y-5 text-sm">
        <p class="leading-relaxed text-(--rg-tx2) text-pretty">
          {boldParts("settings.reset_intro")[0]}<span class="font-bold text-red-600 dark:text-red-400">{$t("settings.reset_irreversible")}</span>{boldParts("settings.reset_intro")[1]}
        </p>

        <ul class="px-4 py-3 rounded-2xl space-y-2 border border-(--rg-bd) bg-(--rg-creux) text-(--rg-tx2)">
          {#each ELEMENTS as el (el)}
            {@const phrase = boldParts(`settings.reset_item_${el}`)}
            <li class="flex items-center gap-2.5">
              <span class="w-1.5 h-1.5 rounded-full bg-red-500/70 shrink-0"></span>
              <span>{phrase[0]}<span class="font-semibold text-(--rg-tx)">{$t(`settings.reset_item_${el}_b`)}</span>{phrase[1]}</span>
            </li>
          {/each}
        </ul>

        <p class="flex items-center gap-2 text-xs text-(--rg-mu)">
          <Icon icon="material-symbols:info-outline-rounded" width="15" class="shrink-0" />
          {$t("settings.reset_admin_kept")}
        </p>

        <div class="flex flex-col gap-2">
          <label for="reset_confirm_input" class="text-xs font-semibold text-(--rg-tx2)">
            {$t("settings.reset_confirm_label").replace("{word}", motConfirmation)}
          </label>
          <FormInput
            id="reset_confirm_input"
            bind:value={saisie}
            placeholder={motConfirmation}
            invalid={!!saisie && saisie !== motConfirmation}
            disabled={loading}
          />
        </div>
      </div>

      <PopinFooter
        cancelLabel={$t("common.cancel")}
        oncancel={handleCancel}
        cancelDisabled={loading}
        submitLabel={$t("settings.reset_confirm_all")}
        submitIcon="material-symbols:delete-outline-rounded"
        onsubmit={confirm}
        submitDisabled={confirmationBloquee}
        busy={loading}
        tone="danger"
      />
    </div>
  </div>
{/if}
