<script lang="ts">
import { goto } from "$app/navigation";
import { oublierLocalisations } from "$lib/helper/library/trackLocation";
import { libraryStore } from "$lib/stores/library/library.store";
import { popinStore } from "$lib/stores/ui/popin.store";
import type { Library } from "$lib/types/db/library/Library";
import Dialog from "$lib/components/ui/dialog/Dialog.svelte";
import Icon from "@iconify/svelte";
import { libraryContentStore } from "$lib/stores/library/libraryContent.store";
import { handleAddDirectory, handleAddFiles } from "$lib/actions/library/LibraryAction";
import { invoke } from "@tauri-apps/api/core";
import { toasts } from "$lib/stores/ui/toast.store";
import { t } from "$lib/i18n";
import LibraryFoldersPopin from "$lib/components/library/common/popin/LibraryFoldersPopin.svelte";

let { library }: { library: Library } = $props();

let rescanning = $state(false);
let showFolders = $state(false);
let fetchingImages = $state(false);

async function handleFetchArtistImages() {
    fetchingImages = true;
    try {
        const found = await invoke<number>('fetch_all_artist_images');
        // Rafraîchir les données pour afficher les nouvelles images
        libraryContentStore.refresh();
        toasts.push({
            type: "success",
            title: "Images artistes",
            message: `${found} image${found !== 1 ? 's' : ''} récupérée${found !== 1 ? 's' : ''}`
        });
    } catch (e) {
        console.error('Failed to fetch artist images:', e);
    } finally {
        fetchingImages = false;
    }
}

async function handleRescan() {
    if (!library?.id) return;
    rescanning = true;
    try {
        await invoke('rescan_library', { libraryId: library.id });
        // Un scan renumérote : les fiches mises en cache peuvent ne plus exister.
        oublierLocalisations();
        libraryContentStore.refresh();
    } catch (e) {
        console.error('Rescan failed:', e);
    } finally {
        rescanning = false;
    }
}

async function openDialogDeleteLibrary() {
    if (!library) return;
    
    popinStore.open(
        "Confirmer la suppression",
        Dialog,
        {
            message: `Êtes-vous sûr de vouloir supprimer la Biblothèque ${library.name} ?`,
            confirmText: "Supprimer",
            cancelText: "Annuler",
            variant: "danger",
            onConfirm: async () => {
            await deleteLibrary();
            },
            onCancel: () => {}
        }
    );
}

async function deleteLibrary() {
    if (!library) return;

    try {
        await libraryStore.removeLibrary(library);
        goto("/library");

    } catch (e) {
       console.log("Impossible de supprimer la bibliothèque");
    }
}

</script>

<!-- ================= ACTIONS ================= -->
<div class="flex items-center gap-1 shrink-0">

  <!-- Ajouter dossier (principal) -->
  <button
    type="button"
    class="group flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-semibold
           bg-green-500 text-black hover:bg-green-400
           active:scale-[0.97] transition-all duration-200 cursor-pointer"
    onclick={() => handleAddDirectory(library.id as number)}
  >
    <Icon icon="mynaui:folder-plus" width={13} />
    <span class="hidden lg:inline">Ajouter un dossier</span>
  </button>

  <!-- Ajouter fichiers -->
  <button
    type="button"
    class="group p-2 rounded-lg cursor-pointer
           text-neutral-500 dark:text-neutral-400
           hover:text-neutral-700 dark:hover:text-neutral-200
           hover:bg-neutral-100 dark:hover:bg-white/[0.07]
           active:scale-[0.97] transition-all duration-150"
    title={$t('library.add_files')}
    onclick={() => handleAddFiles(library.id as number)}
  >
    <Icon icon="lucide:upload" width={15} />
  </button>

  <!-- Rescan -->
  <button
    type="button"
    class="group p-2 rounded-lg cursor-pointer
           text-neutral-500 dark:text-neutral-400
           hover:text-neutral-700 dark:hover:text-neutral-200
           hover:bg-neutral-100 dark:hover:bg-white/[0.07]
           active:scale-[0.97] disabled:opacity-50 transition-all duration-150"
    title={$t('library.sync')}
    onclick={handleRescan}
    disabled={rescanning}
  >
    <Icon icon="lucide:refresh-cw" width={14} class={rescanning ? 'animate-spin' : ''} />
  </button>

  <div class="w-px h-4 bg-neutral-200 dark:bg-white/10 mx-0.5"></div>

  <!-- Dossiers -->
  <button
    type="button"
    class="group p-2 rounded-lg cursor-pointer
           text-neutral-500 dark:text-neutral-400
           hover:text-neutral-700 dark:hover:text-neutral-200
           hover:bg-neutral-100 dark:hover:bg-white/[0.07]
           active:scale-[0.97] transition-all duration-150"
    title="Dossiers synchronisés"
    onclick={() => showFolders = true}
  >
    <Icon icon="lucide:folder-cog" width={14} />
  </button>

  <!-- Images artistes -->
  <button
    type="button"
    class="group p-2 rounded-lg cursor-pointer
           text-neutral-500 dark:text-neutral-400
           hover:text-neutral-700 dark:hover:text-neutral-200
           hover:bg-neutral-100 dark:hover:bg-white/[0.07]
           active:scale-[0.97] disabled:opacity-50 transition-all duration-150"
    title="Images artistes"
    disabled={fetchingImages}
    onclick={handleFetchArtistImages}
  >
    <Icon icon={fetchingImages ? "lucide:loader-2" : "lucide:image-down"} width={14}
          class={fetchingImages ? "animate-spin" : ""} />
  </button>

  <!-- Supprimer -->
  <button
    type="button"
    class="group p-2 rounded-lg cursor-pointer
           text-neutral-500 dark:text-neutral-400
           hover:text-red-500 dark:hover:text-red-400
           hover:bg-red-500/8 dark:hover:bg-red-500/10
           active:scale-[0.97] transition-all duration-150"
    onclick={openDialogDeleteLibrary}
  >
    <Icon icon="lucide:trash-2" width={14} />
  </button>
</div>

{#if showFolders}
  <LibraryFoldersPopin bind:open={showFolders} libraryId={library.id as number} />
{/if}