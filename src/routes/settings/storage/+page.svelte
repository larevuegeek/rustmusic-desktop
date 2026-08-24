<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { openPath } from "@tauri-apps/plugin-opener";
  import { save, open } from "@tauri-apps/plugin-dialog";
  import { toasts } from "$lib/stores/ui/toast.store";
  import { settingsStore } from "$lib/stores/settings/settings.store";
  import { libraryContentStore } from "$lib/stores/library/libraryContent.store";
  import ImportPreviewPopin, { type ImportReport } from "$lib/components/settings/ImportPreviewPopin.svelte";
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
  let dataDir = $state('');
  let exporting = $state(false);
  let importPath = $state<string | null>(null);
  let importReport = $state<ImportReport | null>(null);
  let importing = $state(false);
  let replaceExisting = $state(false);

  /**
   * Choisit un fichier et annonce ce qu'il ferait, sans rien écrire.
   *
   * L'aperçu et l'import passent par le même code côté Rust, appelé une fois à
   * blanc puis une fois pour de bon : deux chemins distincts finiraient par
   * diverger, et l'aperçu mentirait.
   */
  async function handleImportPick() {
    const chemin = await open({
      title: "Importer un export RustMusic",
      multiple: false,
      filters: [{ name: "Export RustMusic", extensions: ["json"] }],
    });
    if (!chemin || typeof chemin !== 'string') return;

    importing = true;
    try {
      importReport = await invoke<ImportReport>('preview_import', { path: chemin });
      importPath = chemin;
      replaceExisting = false;
    } catch (e) {
      console.error('Preview failed:', e);
      toasts.push({ type: "error", title: "Fichier refusé", message: String(e) });
    } finally {
      importing = false;
    }
  }

  async function handleImportConfirm() {
    if (!importPath) return;
    importing = true;
    try {
      const bilan = await invoke<ImportReport>('import_settings_and_playlists', {
        path: importPath,
        options: {
          settings: true,
          playlists: true,
          liked: true,
          replace_existing: replaceExisting,
        },
      });

      // Les réglages viennent d'être réécrits sous les pieds du magasin : sans
      // cette relecture, l'interface garderait l'ancien thème et l'ancienne
      // langue jusqu'au prochain démarrage.
      await settingsStore.init();
      await libraryContentStore.refresh();

      importReport = null;
      importPath = null;

      const manquants = bilan.tracks_missing;
      toasts.push({
        type: manquants > 0 ? "info" : "success",
        title: "Import terminé",
        message:
          `${bilan.playlists_created + bilan.playlists_replaced} playlist(s), ` +
          `${bilan.tracks_matched_by_path + bilan.tracks_matched_by_tags} morceaux retrouvés` +
          (manquants > 0 ? `, ${manquants} introuvables` : ''),
      });
    } catch (e) {
      console.error('Import failed:', e);
      toasts.push({ type: "error", title: "Import impossible", message: String(e) });
    } finally {
      importing = false;
    }
  }

  type ExportSummary = {
    path: string;
    settings: number;
    profils: number;
    playlists: number;
    tracks: number;
    liked: number;
    bytes: number;
  };

  /**
   * Écrit réglages, profils, playlists et titres aimés dans un fichier.
   *
   * Le nom proposé porte la date : on exporte rarement une fois, et deux
   * sauvegardes qui s'appellent pareil finissent par s'écraser l'une l'autre.
   */
  async function handleExport() {
    const jour = new Date().toISOString().slice(0, 10);
    const chemin = await save({
      title: "Exporter les réglages et les playlists",
      defaultPath: `rustmusic-${jour}.json`,
      filters: [{ name: "Export RustMusic", extensions: ["json"] }],
    });

    // Annulation du sélecteur : ce n'est pas une erreur, on ne dit rien.
    if (!chemin) return;

    exporting = true;
    try {
      const bilan = await invoke<ExportSummary>('export_settings_and_playlists', { path: chemin });
      toasts.push({
        type: "success",
        title: "Export terminé",
        message:
          `${bilan.settings} réglages, ${bilan.playlists} playlist${bilan.playlists > 1 ? 's' : ''} ` +
          `(${bilan.tracks} piste${bilan.tracks > 1 ? 's' : ''}), ${bilan.liked} titre${bilan.liked > 1 ? 's' : ''} aimé${bilan.liked > 1 ? 's' : ''}`,
      });
    } catch (e) {
      console.error('Export failed:', e);
      toasts.push({ type: "error", title: "Export impossible", message: String(e) });
    } finally {
      exporting = false;
    }
  }

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
    // dataDir est vide tant que le onMount n'a pas résolu le chemin.
    if (!dataDir) return;
    await openPath(dataDir);
  }

  onMount(async () => {
      dataDir = await appDataDir();
  })
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

  <!-- Export des réglages et des playlists -->
  <OptionRow
    icon="lucide:hard-drive-download"
    title="Exporter réglages et playlists"
    desc="Un fichier JSON avec les réglages, les profils, les playlists et les titres aimés. La bibliothèque n'y est pas : un scan la reconstruit."
  >
    <ActionButton
      icon="lucide:download"
      label={exporting ? "Export…" : "Exporter"}
      busy={exporting}
      onclick={handleExport}
    />
  </OptionRow>

  <!-- Import -->
  <OptionRow
    icon="lucide:hard-drive-upload"
    title="Importer un export"
    desc="Rejoue un fichier d'export. Les morceaux sont retrouvés par leur chemin, ou par leurs tags si les chemins ont changé. Rien n'est écrit avant confirmation."
  >
    <ActionButton
      icon="lucide:upload"
      label={importing && !importReport ? "Lecture…" : "Importer"}
      busy={importing && !importReport}
      onclick={handleImportPick}
    />
  </OptionRow>

  <!-- Ouvrir le dossier de données -->
  <OptionRow
    icon="lucide:folder-open"
    title={$t('settings.open_data_folder')}
    desc={$t('settings.open_data_folder_desc')}
    value={dataDir}
  >
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

{#if importReport}
  <ImportPreviewPopin
    report={importReport}
    busy={importing}
    bind:replaceExisting
    onconfirm={handleImportConfirm}
    onclose={() => { importReport = null; importPath = null; }}
  />
{/if}
