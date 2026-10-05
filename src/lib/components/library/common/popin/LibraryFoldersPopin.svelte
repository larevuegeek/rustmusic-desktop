<script lang="ts">
  import { tailleLisible } from "#lib/helper/tools/sizeTools";
  import Icon from "@iconify/svelte";
  import { oublierLocalisations } from "#lib/helper/library/trackLocation";
  import { invoke } from "@tauri-apps/api/core";
  import { handleAddDirectory } from "#lib/actions/library/LibraryAction";
  import { libraryContentStore } from "#lib/stores/library/libraryContent.store";
  import { importProgressStore } from "#lib/stores/library/importProgress.store";
  import { toasts } from "#lib/stores/ui/toast.store";
  import { portal } from "#lib/helper/portal";
  import { t, currentLocale } from "#lib/i18n";
  import { depuisCourt } from "#lib/helper/tools/dateTools";

  type LibraryDir = {
    id: string;
    library_id: number;
    path: string;
    name: string;
    is_recursive: boolean;
    is_active: boolean;
    total_files: number;
    total_size: number;
    last_scan_at: string | null;
    scan_status: string;
    created_at: string;
  };

  let { open = $bindable(true), libraryId }: { open: boolean; libraryId: number } = $props();

  let dirs: LibraryDir[] = $state([]);
  let loading = $state(true);
  let rescanningId: string | null = $state(null);

  $effect(() => {
    if (open) loadDirs();
  });

  const imp = $derived($importProgressStore);
  const importIci = $derived(imp.active && imp.libraryId === libraryId);

  // Recharge quand un dossier inconnu commence à s'importer, puis à la fin de chaque dossier.
  let dossierSuivi = '';
  $effect(() => {
    const dossier = importIci ? imp.directory : '';
    if (dossier === dossierSuivi) return;
    const fini = dossierSuivi !== '';
    dossierSuivi = dossier;
    if (open && (fini || !dirs.some(d => d.path === dossier))) loadDirs(false);
  });

  async function loadDirs(attente = true) {
    if (attente) loading = true;
    try {
      dirs = await invoke('get_library_dirs', { libraryId });
    } catch (e) {
      console.error('Failed to load dirs:', e);
    } finally {
      loading = false;
    }
  }

  function signalerOccupe(e: unknown): boolean {
    if (e !== 'deja_en_cours') return false;
    toasts.push({ type: "error", title: $t("notify.error"), message: $t("notify.import_busy") });
    return true;
  }

  async function handleRescanDir(dir: LibraryDir) {
    rescanningId = dir.id;
    try {
      await invoke('rescan_library_dir', { libraryId, dirId: dir.id });
      oublierLocalisations();
      await loadDirs();
      libraryContentStore.load(libraryId);
    } catch (e) {
      if (!signalerOccupe(e)) console.error('Rescan failed:', e);
    } finally {
      rescanningId = null;
    }
  }

  async function handleRemoveDir(dir: LibraryDir) {
    try {
      await invoke('remove_library_dir', { dirId: dir.id });
      dirs = dirs.filter(d => d.id !== dir.id);
    } catch (e) {
      console.error('Remove failed:', e);
    }
  }

  async function handleAddFolder() {
    await handleAddDirectory(libraryId);
    await loadDirs();
  }

  const formatSize = (bytes: number) => tailleLisible(bytes, $currentLocale);

  function nb(n: number, un: string, plusieurs: string): string {
    return n === 1 ? $t(un) : $t(plusieurs).replace('{n}', n.toLocaleString($currentLocale));
  }

  const totalFichiers = $derived(dirs.reduce((s, d) => s + (d.total_files ?? 0), 0));
  const totalTaille = $derived(dirs.reduce((s, d) => s + (d.total_size ?? 0), 0));
  const resume = $derived([
    nb(dirs.length, 'folders_popin.folders_one', 'folders_popin.folders_n'),
    totalFichiers > 0 ? nb(totalFichiers, 'folders_popin.files_one', 'folders_popin.files_n') : '',
    formatSize(totalTaille),
  ].filter(Boolean).join(' · '));

  const maintenant = Date.now();
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && (open = false)} />

<!-- Porté vers `body` : ouverte depuis la barre latérale, elle y restait enfermée
     (son conteneur porte une transformation, qui piège les `fixed`). -->
<div use:portal>
<div class="sidebar fixed inset-0 z-9990 flex items-center justify-center p-4">
  <button
    type="button"
    class="absolute inset-0 bg-black/60 backdrop-blur-sm cursor-default"
    onclick={() => open = false}
    aria-label={$t('common.close')}
  ></button>

  <div
    role="dialog"
    aria-modal="true"
    aria-labelledby="dossiers-titre"
    class="relative w-full max-w-xl max-h-[80vh] flex flex-col overflow-hidden rounded-2xl
           bg-(--sb-bg) border border-(--sb-bd) shadow-2xl shadow-black/40"
  >
    <!-- En-tête -->
    <div class="flex items-center gap-3 px-5 pt-5 pb-4">
      <span class="w-10 h-10 rounded-xl shrink-0 flex items-center justify-center bg-(--sb-gbg) text-(--sb-g)">
        <Icon icon="material-symbols:folder-managed-outline-rounded" width="22" class="sb-icone" />
      </span>
      <div class="flex-1 min-w-0">
        <h2 id="dossiers-titre" class="text-lg font-bold leading-tight text-(--sb-tx)">{$t('folders_popin.title')}</h2>
        <p class="text-xs text-(--sb-mu) truncate">{loading ? $t('common.loading') : resume}</p>
      </div>
      <button
        type="button"
        class="w-8 h-8 shrink-0 rounded-lg flex items-center justify-center cursor-pointer transition-colors
               text-(--sb-mu) hover:bg-(--sb-s2) hover:text-(--sb-tx)"
        onclick={() => open = false}
        title={$t('common.close')}
        aria-label={$t('common.close')}
      >
        <Icon icon="material-symbols:close-rounded" width="20" />
      </button>
    </div>

    <!-- Dossiers -->
    <div class="flex-1 overflow-y-auto px-4 pb-4 smart-scroll">
      {#if loading}
        <div class="flex items-center justify-center py-12">
          <Icon icon="lucide:loader-2" width="20" class="animate-spin text-(--sb-mu)" />
        </div>

      {:else if dirs.length === 0}
        <div class="flex flex-col items-center justify-center py-10 text-center rounded-xl border border-dashed border-(--sb-bd2)">
          <Icon icon="material-symbols:folder-open-outline-rounded" width="32" class="text-(--sb-mu) mb-2" />
          <p class="text-sm font-semibold text-(--sb-tx2)">{$t('folders_popin.empty')}</p>
          <p class="text-xs text-(--sb-mu) mt-0.5">{$t('folders_popin.empty_desc')}</p>
        </div>

      {:else}
        <div class="flex flex-col gap-2">
          {#each dirs as dir (dir.id)}
            {@const suivi = importIci && imp.directory === dir.path}
            {@const enCours = suivi || rescanningId === dir.id}
            <div class="flex items-center gap-3 p-3 rounded-xl bg-(--sb-s1) border border-(--sb-bd)">
              <span class="w-10 h-10 rounded-lg shrink-0 flex items-center justify-center bg-(--sb-gbg) text-(--sb-g)">
                <Icon icon="material-symbols:folder-outline-rounded" width="20" class="sb-icone" />
              </span>

              <div class="flex-1 min-w-0">
                <p class="text-sm font-bold truncate text-(--sb-tx)">{dir.name}</p>
                <p class="font-mono text-[11px] truncate text-(--sb-mu2)" title={dir.path}>{dir.path}</p>

                <div class="flex flex-wrap items-center gap-1.5 mt-2 text-[11px] font-medium">
                  {#if dir.total_files > 0}
                    <span class="px-2 py-0.5 rounded-md bg-(--sb-s2) text-(--sb-tx2)">
                      {nb(dir.total_files, 'folders_popin.files_one', 'folders_popin.files_n')}
                    </span>
                  {/if}
                  {#if dir.total_size > 0}
                    <span class="px-2 py-0.5 rounded-md bg-(--sb-s2) text-(--sb-tx2)">{formatSize(dir.total_size)}</span>
                  {/if}
                  <span class="flex items-center gap-1.5 px-2 py-0.5 rounded-md bg-(--sb-s2) text-(--sb-tx2)">
                    <span class="sb-point w-1.5 h-1.5 rounded-full
                                 {enCours ? 'text-amber-400 animate-pulse' : dir.last_scan_at ? 'text-(--sb-point)' : 'text-(--sb-mu)'}"></span>
                    {#if suivi && imp.stopping}
                      {$t('folders_popin.stopping')}
                    {:else if suivi && imp.phase === 'listing'}
                      {$t('folders_popin.listing').replace('{n}', imp.found.toLocaleString($currentLocale))}
                    {:else if suivi}
                      {imp.current.toLocaleString($currentLocale)} / {imp.total.toLocaleString($currentLocale)}
                    {:else if enCours}
                      {$t('folders_popin.scanning')}
                    {:else if dir.scan_status === 'cancelled'}
                      {$t('folders_popin.cancelled')}
                    {:else if dir.last_scan_at}
                      {$t('folders_popin.scanned').replace('{ago}', depuisCourt(dir.last_scan_at, maintenant, $t))}
                    {:else}
                      {$t('folders_popin.never_scanned')}
                    {/if}
                  </span>
                </div>

                {#if suivi && imp.phase === 'importing'}
                  <div class="mt-2 h-1 rounded-full overflow-hidden bg-(--sb-s2)">
                    <div class="h-full rounded-full bg-(--sb-g) transition-[width] duration-300" style="width: {imp.percent}%"></div>
                  </div>
                  <p class="mt-1 font-mono text-[10px] truncate text-(--sb-mu2)" title={imp.fileName}>{imp.fileName}</p>
                {/if}
              </div>

              <div class="flex items-center gap-0.5 shrink-0 self-start">
                {#if suivi}
                  <button
                    type="button"
                    class="w-8 h-8 rounded-lg flex items-center justify-center cursor-pointer transition-colors
                           text-amber-400 enabled:hover:bg-(--sb-s2) disabled:cursor-default disabled:opacity-50"
                    title={$t('folders_popin.stop')}
                    aria-label={$t('folders_popin.stop')}
                    disabled={imp.stopping}
                    onclick={() => importProgressStore.stop()}
                  >
                    <Icon icon="material-symbols:stop-circle-outline-rounded" width="18" />
                  </button>
                {:else}
                  <button
                    type="button"
                    class="w-8 h-8 rounded-lg flex items-center justify-center cursor-pointer transition-colors
                           text-(--sb-mu) enabled:hover:bg-(--sb-s2) enabled:hover:text-(--sb-tx) disabled:cursor-default disabled:opacity-50"
                    title={$t('folders_popin.rescan')}
                    aria-label={$t('folders_popin.rescan')}
                    disabled={enCours || imp.active}
                    onclick={() => handleRescanDir(dir)}
                  >
                    <Icon icon="material-symbols:refresh-rounded" width="18" class={enCours ? 'animate-spin text-amber-400' : ''} />
                  </button>
                {/if}
                <button
                  type="button"
                  class="w-8 h-8 rounded-lg flex items-center justify-center cursor-pointer transition-colors
                         text-(--sb-mu) hover:bg-red-500/10 hover:text-red-400"
                  title={$t('folders_popin.remove')}
                  aria-label={$t('folders_popin.remove')}
                  onclick={() => handleRemoveDir(dir)}
                >
                  <Icon icon="material-symbols:delete-outline-rounded" width="18" />
                </button>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </div>

    <!-- Pied -->
    <div class="flex items-center justify-between gap-3 px-5 py-4 border-t border-(--sb-sep)">
      <button
        type="button"
        class="flex items-center gap-2 h-9 px-3.5 rounded-lg cursor-pointer text-sm font-semibold transition-[filter]
               bg-(--sb-g) text-(--rg-on-g) hover:brightness-110"
        onclick={handleAddFolder}
      >
        <Icon icon="material-symbols:add-rounded" width="18" />
        {$t('folders_popin.add')}
      </button>
      <button
        type="button"
        class="h-9 px-4 rounded-lg cursor-pointer text-sm font-semibold transition-colors
               text-(--sb-tx2) hover:bg-(--sb-s2) hover:text-(--sb-tx)"
        onclick={() => open = false}
      >
        {$t('common.close')}
      </button>
    </div>
  </div>
</div>
</div>
