<script lang="ts">
  import Icon from "@iconify/svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { t } from "#lib/i18n";
  import TypedConfirmDialog from "#lib/components/ui/dialog/TypedConfirmDialog.svelte";

  let { open = $bindable(true) }: { open: boolean } = $props();

  // Phrase coupée autour de son mot en gras ({b}).
  const boldParts = (cle: string) => [...$t(cle).split("{b}"), ""];
  const ELEMENTS = ["libraries", "playlists", "liked", "settings", "thumbnails"];

  async function handleConfirm() {
    await invoke('reset_application');

    // Relancer l'app — la DB sera recréée par les migrations au démarrage
    try {
      const { relaunch } = await import('@tauri-apps/plugin-process');
      await relaunch();
    } catch {
      // En mode dev, relaunch peut échouer — on ferme simplement
      const { getCurrentWindow } = await import('@tauri-apps/api/window');
      await getCurrentWindow().close();
    }
  }
</script>

<TypedConfirmDialog bind:open title={$t("settings.reset")} confirmLabel={$t("settings.reset_confirm_all")} onconfirm={handleConfirm}>
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
</TypedConfirmDialog>
