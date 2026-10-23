<script lang="ts">
  // Choix des dossiers d'un serveur DLNA : on descend dans l'arborescence, on coche ceux à ajouter.
  import Icon from "@iconify/svelte";
  import { t, currentLocale } from "#lib/i18n";
  import { serverStore } from "#lib/stores/mediaServer/mediaServer.store";
  import { errorMessage } from "#lib/helper/tools/errorTools";
  import type { DlnaFolder } from "#lib/types/ui/mediaServer/MediaServer";

  let {
    url,
    selected = $bindable([]),
    disabled = false,
  }: { url: string; selected?: { id: string; name: string }[]; disabled?: boolean } = $props();

  /** Chemin ouvert, racine comprise. */
  let trail = $state<{ id: string; name: string }[]>([{ id: "0", name: "" }]);
  let folders = $state<DlnaFolder[]>([]);
  let loading = $state(false);
  let erreur = $state<string | null>(null);

  const current = $derived(trail[trail.length - 1]);
  $effect(() => {
    void load(current.id);
  });

  async function load(id: string) {
    loading = true;
    erreur = null;
    try {
      const found = await serverStore.browse(url, id);
      if (trail[trail.length - 1].id === id) folders = found;
    } catch (err) {
      erreur = errorMessage(err);
      folders = [];
    } finally {
      loading = false;
    }
  }

  const isSelected = (id: string) => selected.some((f) => f.id === id);
  function toggle(f: DlnaFolder) {
    selected = isSelected(f.id) ? selected.filter((x) => x.id !== f.id) : [...selected, { id: f.id, name: f.name }];
  }
  const open = (f: DlnaFolder) => (trail = [...trail, { id: f.id, name: f.name }]);
  const items = (n: number) => $t("servers.dlna_items").replace("{n}", n.toLocaleString($currentLocale));
</script>

<div class="flex flex-col gap-2">
  <!-- Fil d'Ariane -->
  <nav class="flex flex-wrap items-center gap-0.5 text-[13px]" aria-label={$t("servers.dlna_choose")}>
    {#each trail as step, i (step.id)}
      {#if i > 0}<Icon icon="material-symbols:chevron-right-rounded" width="16" class="text-(--rg-mu)" />{/if}
      <button type="button" disabled={disabled || i === trail.length - 1} onclick={() => (trail = trail.slice(0, i + 1))}
              class="px-1.5 py-0.5 rounded-md font-semibold truncate max-w-56 cursor-pointer disabled:cursor-default
                     {i === trail.length - 1 ? 'text-(--rg-tx)' : 'text-(--rg-gtx) hover:bg-(--rg-s2)'}">
        {i === 0 ? $t("servers.dlna_root") : step.name}
      </button>
    {/each}
  </nav>

  <div class="flex flex-col gap-1.5 max-h-72 overflow-y-auto pr-1">
    {#if loading && folders.length === 0}
      <p class="flex items-center gap-2 px-1 py-2 text-sm text-(--rg-mu)">
        <Icon icon="material-symbols:progress-activity" width="16" class="animate-spin" />{$t("servers.dlna_loading")}
      </p>
    {:else if erreur}
      <p class="px-1 py-2 text-sm text-red-600 dark:text-red-400">{erreur}</p>
    {:else}
      {#each folders as f (f.id)}
        {@const on = isSelected(f.id)}
        <div class="flex items-center gap-1 rounded-xl border transition-colors
                    {on ? 'border-(--rg-gbd) bg-(--rg-gbg)' : 'border-(--rg-bd) hover:border-(--rg-bd2)'}">
          <label class="flex-1 min-w-0 flex items-center gap-3 pl-3.5 py-2.5 cursor-pointer">
            <input type="checkbox" class="checkbox-app" checked={on} onchange={() => toggle(f)} {disabled} />
            <Icon icon="material-symbols:folder-outline-rounded" width="18" class="shrink-0 text-(--rg-mu)" />
            <span class="flex-1 min-w-0 truncate text-sm font-semibold text-(--rg-tx)">{f.name}</span>
            {#if f.children != null}<span class="shrink-0 text-xs text-(--rg-mu)">{items(f.children)}</span>{/if}
          </label>
          {#if f.children !== 0}
            <button type="button" {disabled} onclick={() => open(f)} title={$t("servers.dlna_open")} aria-label={`${$t("servers.dlna_open")} ${f.name}`}
                    class="w-10 h-10 shrink-0 mr-0.5 rounded-lg flex items-center justify-center cursor-pointer text-(--rg-mu) hover:text-(--rg-tx) hover:bg-(--rg-s2)">
              <Icon icon="material-symbols:chevron-right-rounded" width="20" />
            </button>
          {/if}
        </div>
      {:else}
        <p class="px-1 py-2 text-sm text-(--rg-mu)">{$t("servers.dlna_empty_folder")}</p>
      {/each}
    {/if}
  </div>

  {#if selected.length}
    <div class="flex flex-wrap gap-1.5 pt-1">
      {#each selected as f (f.id)}
        <span class="h-7 pl-2.5 pr-1 inline-flex items-center gap-1.5 rounded-full text-[12.5px] font-medium bg-(--rg-gbg) text-(--rg-tx) border border-(--rg-gbd)">
          <Icon icon="material-symbols:cloud-outline-rounded" width="14" class="text-(--rg-mu)" />{f.name}
          <button type="button" {disabled} onclick={() => (selected = selected.filter((x) => x.id !== f.id))} aria-label={$t("common.remove")}
                  class="w-5 h-5 rounded-full flex items-center justify-center cursor-pointer text-(--rg-mu) hover:text-(--rg-tx) hover:bg-(--rg-s2)">
            <Icon icon="material-symbols:close-rounded" width="14" />
          </button>
        </span>
      {/each}
    </div>
  {/if}
</div>
