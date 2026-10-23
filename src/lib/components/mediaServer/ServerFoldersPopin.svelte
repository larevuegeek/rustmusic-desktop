<script lang="ts">
  // Dossiers d'un serveur déjà ajouté : on en coche d'autres, chacun devient une bibliothèque.
  import Icon from "@iconify/svelte";
  import { t } from "#lib/i18n";
  import { popinStore } from "#lib/stores/ui/popin.store";
  import { serverStore } from "#lib/stores/mediaServer/mediaServer.store";
  import { toasts } from "#lib/stores/ui/toast.store";
  import { errorMessage } from "#lib/helper/tools/errorTools";
  import PopinFooter from "#lib/components/ui/popin/PopinFooter.svelte";
  import DlnaFolderPicker from "#lib/components/mediaServer/DlnaFolderPicker.svelte";
  import type { MusicFolder, ServerEntry } from "#lib/types/ui/mediaServer/MediaServer";

  let { server }: { server: ServerEntry } = $props();

  const dlna = $derived(server.kind === "dlna");
  const deja = $derived(new Set(server.libraries.map((l) => l.remote_folder_id)));

  let folders = $state<MusicFolder[]>([]);
  let choisis = $state<string[]>([]);
  let dossiers = $state<{ id: string; name: string }[]>([]);
  let chargement = $state(true);
  let busy = $state(false);
  let erreur = $state<string | null>(null);

  $effect(() => {
    if (dlna) {
      chargement = false;
      return;
    }
    serverStore.folders(server.id)
      .then((f) => (folders = f))
      .catch((e) => (erreur = errorMessage(e)))
      .finally(() => (chargement = false));
  });

  const nouveaux = $derived(dlna ? dossiers.map((f) => f.id).filter((id) => !deja.has(id)) : choisis);
  const toggle = (id: string) => (choisis = choisis.includes(id) ? choisis.filter((x) => x !== id) : [...choisis, id]);

  async function add() {
    if (!nouveaux.length) return;
    busy = true;
    erreur = null;
    try {
      await serverStore.addLibraries(server.id, nouveaux);
      toasts.push({ type: "success", title: $t("servers.folders_added").replace("{n}", String(nouveaux.length)) });
      popinStore.close();
    } catch (e) {
      erreur = errorMessage(e);
    } finally {
      busy = false;
    }
  }
</script>

<div class="space-y-4">
  <p class="text-sm leading-relaxed text-(--rg-tx2)">{$t("servers.folders_intro")}</p>

  {#if dlna}
    <DlnaFolderPicker url={server.url} bind:selected={dossiers} disabled={busy} />
  {:else if chargement}
    <p class="flex items-center gap-2 text-sm text-(--rg-mu)">
      <Icon icon="material-symbols:progress-activity" width="16" class="animate-spin" />{$t("servers.dlna_loading")}
    </p>
  {:else}
    <div class="flex flex-col gap-1.5">
      {#each folders as f (f.id)}
        {@const ajoute = deja.has(f.id)}
        <label class="flex items-center gap-3 px-3.5 py-2.5 rounded-xl border transition-colors
                      {ajoute || choisis.includes(f.id) ? 'border-(--rg-gbd) bg-(--rg-gbg)' : 'border-(--rg-bd) hover:border-(--rg-bd2)'}
                      {ajoute ? 'cursor-default' : 'cursor-pointer'}">
          <input type="checkbox" class="checkbox-app" checked={ajoute || choisis.includes(f.id)} disabled={ajoute || busy} onchange={() => toggle(f.id)} />
          <Icon icon="material-symbols:cloud-outline-rounded" width="18" class="shrink-0 text-(--rg-mu)" />
          <span class="flex-1 min-w-0 truncate text-sm font-semibold text-(--rg-tx)">{f.name}</span>
          {#if ajoute}<span class="shrink-0 text-xs text-(--rg-mu)">{$t("servers.folder_added")}</span>{/if}
        </label>
      {:else}
        <p class="text-sm text-(--rg-mu)">{$t("servers.no_folders")}</p>
      {/each}
    </div>
  {/if}

  {#if erreur}
    <p class="flex items-center gap-2 px-4 py-3 rounded-xl border border-red-500/25 bg-red-500/8 text-sm text-red-600 dark:text-red-400">
      <Icon icon="material-symbols:warning-outline-rounded" width="17" class="shrink-0" />{erreur}
    </p>
  {/if}
</div>

<PopinFooter
  bleed
  cancelLabel={$t("common.cancel")}
  oncancel={() => popinStore.close()}
  cancelDisabled={busy}
  submitLabel={$t("servers.add_libraries").replace("{n}", String(nouveaux.length))}
  submitIcon="material-symbols:add-rounded"
  onsubmit={add}
  submitDisabled={nouveaux.length === 0}
  {busy}
/>
