<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { openPath } from "@tauri-apps/plugin-opener";
  import { appDataDir } from "@tauri-apps/api/path";
  import { libraryStore } from "$lib/stores/library/library.store";
  import { t } from "$lib/i18n";
  import OptionRow from "$lib/components/ui/input/OptionRow.svelte";
  import ActionButton from "$lib/components/ui/button/ActionButton.svelte";
  import ResetConfirmPopin from "$lib/components/settings/ResetConfirmPopin.svelte";

  let showResetDialog = $state(false);
  let rescanning = $state(false);
  let fetchingImages = $state(false);
  let fetchingCovers = $state(false);

  async function handleRescan() {
    rescanning = true;
    try {
      const libraries = $libraryStore.libraries;
      for (const lib of libraries) {
        await invoke('rescan_library', { libraryId: lib.id });
      }
    } catch (e) {
      console.error('Rescan failed:', e);
    } finally {
      rescanning = false;
    }
  }

  async function handleFetchArtistImages() {
    fetchingImages = true;
    try {
      await invoke('fetch_all_artist_images', { force: true });
    } catch (e) {
      console.error('Failed to fetch artist images:', e);
    } finally {
      fetchingImages = false;
    }
  }

  async function handleFetchAlbumCovers() {
    fetchingCovers = true;
    try {
      const libraries = $libraryStore.libraries;
      for (const lib of libraries) {
        await invoke('fetch_all_album_covers', { libraryId: lib.id });
      }
    } catch (e) {
      console.error('Failed to fetch album covers:', e);
    } finally {
      fetchingCovers = false;
    }
  }

  async function handleOpenDataFolder() {
    const dir = await appDataDir();
    await openPath(dir);
  }
</script>

<section class="space-y-1">
  <!-- Rescan bibliothèque -->
  <OptionRow icon="lucide:refresh-cw" title={$t('settings.rescan_library')} desc={$t('settings.rescan_library_desc')}>
    <ActionButton
      icon="lucide:refresh-cw"
      label={$t('settings.rescan_btn')}
      busy={rescanning}
      onclick={handleRescan}
    />
  </OptionRow>

  <!-- Images d'artistes -->
  <OptionRow icon="lucide:image-down" title={$t('common.artist_images')} desc={$t('common.artist_images_desc')}>
    <ActionButton
      icon="lucide:download"
      label={fetchingImages ? $t('common.fetching') : $t('common.fetch')}
      busy={fetchingImages}
      onclick={handleFetchArtistImages}
    />
  </OptionRow>

  <!-- Pochettes d'albums -->
  <OptionRow icon="lucide:disc-album" title={$t('settings.album_covers')} desc={$t('settings.album_covers_desc')}>
    <ActionButton
      icon="lucide:download"
      label={fetchingCovers ? $t('common.fetching') : $t('common.fetch')}
      busy={fetchingCovers}
      onclick={handleFetchAlbumCovers}
    />
  </OptionRow>

  <!-- Ouvrir le dossier de données -->
  <OptionRow icon="lucide:folder-open" title={$t('settings.open_data_folder')} desc={$t('settings.open_data_folder_desc')}>
    <ActionButton
      icon="lucide:external-link"
      label={$t('settings.open_btn')}
      onclick={handleOpenDataFolder}
    />
  </OptionRow>

  <!-- Réinitialisation -->
  <OptionRow icon="lucide:rotate-ccw" iconClass="text-red-400/60" title={$t('settings.reset')} desc={$t('settings.reset_desc')}>
    <ActionButton
      icon="lucide:trash-2"
      label={$t('settings.reset_btn')}
      danger
      onclick={() => (showResetDialog = true)}
    />
  </OptionRow>
</section>

{#if showResetDialog}
  <ResetConfirmPopin bind:open={showResetDialog} />
{/if}
