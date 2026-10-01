<script lang="ts">
import Icon from "@iconify/svelte";
import { goto } from "$app/navigation";
import { libraryStore } from "$lib/stores/library/library.store";
import { page } from "$app/state";
import LibraryImportingLoader from "$lib/components/library/common/loader/LibraryImportingLoader.svelte";
import { handleAddFiles, handleAddDirectory } from "$lib/actions/library/LibraryAction";
import { onMount } from "svelte";
import { t } from "$lib/i18n";

const libraryId = $derived(Number(page.params.library_id));

// Simple étape : on file vers le dernier onglet visité.
onMount(() => {
  let lastTab: string | null = null;
  try { lastTab = localStorage.getItem(`lib-tab-${libraryId}`); } catch {}
  goto(`/library/${libraryId}/${lastTab || 'tracks'}`, { replaceState: true });
});
const currentLibrary = $derived(
  $libraryStore.libraries.find(l => l.id === libraryId)
);
</script>

{#if $libraryStore.isImporting}

  <LibraryImportingLoader />

{:else if currentLibrary?.total_tracks === 0}
  <div class="flex flex-col items-center justify-center py-20 px-6 text-center">
    <div class="relative mb-5">
      <div class="absolute inset-0 rounded-2xl bg-green-500/25 blur-2xl scale-[2] animate-pulse"></div>
      <div class="relative w-16 h-16 rounded-2xl
                  bg-neutral-100 dark:bg-neutral-800
                  border border-neutral-200/60 dark:border-neutral-700/40
                  flex items-center justify-center">
        <Icon icon="lucide:music" width="24" class="text-green-500/60" />
      </div>
    </div>

    <h3 class="text-base font-semibold text-neutral-700 dark:text-neutral-200 mb-1.5">
      {$t("library.empty_title")}
    </h3>

    <p class="text-sm text-neutral-400 dark:text-neutral-500 max-w-xs leading-relaxed mb-6">
      {$t("library.empty_desc")}
    </p>

    <div class="flex items-center gap-2">
      <button
        class="flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-semibold cursor-pointer
               bg-green-500 text-black shadow-sm shadow-green-900/20
               hover:bg-green-500 hover:shadow-md
               active:scale-[0.97] transition-all duration-150"
        onclick={() => libraryId && handleAddFiles(libraryId)}
      >
        <Icon icon="lucide:file-audio" width="14" />
        {$t("library.import_files")}
      </button>
      <button
        class="flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium cursor-pointer
               border border-neutral-200/80 dark:border-neutral-700/60
               text-neutral-600 dark:text-neutral-300
               hover:bg-neutral-100/80 dark:hover:bg-neutral-800/60
               active:scale-[0.97] transition-all duration-150"
        onclick={() => libraryId && handleAddDirectory(libraryId)}
      >
        <Icon icon="lucide:folder-plus" width="14" />
        {$t("library.import_folder")}
      </button>
    </div>
  </div>
{/if}
