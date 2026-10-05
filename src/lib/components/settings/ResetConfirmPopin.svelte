<script lang="ts">
  import { Dialog } from "@karbonjs/ui-svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { profilSelector } from "#lib/stores/profil/profil.store";
  import { libraryStore } from "#lib/stores/library/library.store";
  import { playlistStore } from "#lib/stores/playlist/playlist.store";
  import { liked } from "#lib/stores/playlist/like.store";
  import { goto } from "$app/navigation";
  import { t } from "#lib/i18n";

  let { open = $bindable(true) }: { open: boolean } = $props();
  let loading = $state(false);

  const motConfirmation = $derived($t("settings.reset_confirm_word"));
  // Phrase coupée autour de son mot en gras ({b}).
  const morceaux = (cle: string) => [...$t(cle).split("{b}"), ""];
  const ELEMENTS = ["libraries", "playlists", "liked", "settings", "thumbnails"];

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
</script>

<Dialog
  bind:open
  title={$t("settings.reset")}
  variant="danger"
  backdrop="blur"
  classes={{
    overlay: '!bg-black/70',
    content: '!bg-neutral-950 !border-neutral-800/60'
  }}
  confirmLabel={$t("settings.reset_confirm_all")}
  cancelLabel={$t("common.cancel")}
  confirmInput={motConfirmation}
  confirmInputLabel={$t("settings.reset_confirm_label").replace("{word}", motConfirmation)}
  confirmInputPlaceholder={motConfirmation}
  {loading}
  onconfirm={handleConfirm}
  oncancel={handleCancel}
>
  <div class="space-y-3 text-sm">
    <p class="text-neutral-300 leading-relaxed">
      {morceaux("settings.reset_intro")[0]}<span class="font-bold text-red-400">{$t("settings.reset_irreversible")}</span>{morceaux("settings.reset_intro")[1]}
    </p>

    <ul class="space-y-1.5 text-neutral-400">
      {#each ELEMENTS as el (el)}
        {@const phrase = morceaux(`settings.reset_item_${el}`)}
        <li class="flex items-center gap-2">
          <span class="w-1.5 h-1.5 rounded-full bg-red-500/60 shrink-0"></span>
          {phrase[0]}<span class="text-neutral-200 font-medium">{$t(`settings.reset_item_${el}_b`)}</span>{phrase[1]}
        </li>
      {/each}
    </ul>

    <p class="text-xs text-neutral-500 pt-1">
      {$t("settings.reset_admin_kept")}
    </p>
  </div>
</Dialog>
