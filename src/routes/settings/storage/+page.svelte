<script lang="ts">
  import { fetchCovers, fetchPortraits, cleanImages } from "#lib/actions/library/ImageAction";
  import Icon from "@iconify/svelte";
  import { onMount } from "svelte";
  import { forgetLocations } from "#lib/helper/library/trackLocation";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { openPath } from "@tauri-apps/plugin-opener";
  import { save, open } from "@tauri-apps/plugin-dialog";
  import { toasts } from "#lib/stores/ui/toast.store";
  import { settingsStore } from "#lib/stores/settings/settings.store";
  import { libraryContentStore } from "#lib/stores/library/libraryContent.store";
  import ImportPreviewPopin, { type ImportReport } from "#lib/components/settings/ImportPreviewPopin.svelte";
  import { appDataDir } from "@tauri-apps/api/path";
  import { libraryStore } from "#lib/stores/library/library.store";
  import { t, currentLocale } from "#lib/i18n";
  import OptionGroup from "#lib/components/ui/input/OptionGroup.svelte";
  import OptionItem from "#lib/components/ui/input/OptionItem.svelte";
  import GhostButton from "#lib/components/ui/button/GhostButton.svelte";
  import ResetConfirmPopin from "#lib/components/settings/ResetConfirmPopin.svelte";
  import ServersSettings from "#lib/components/settings/ServersSettings.svelte";
  import ToggleSwitch from "#lib/components/ui/input/ToggleSwitch.svelte";
  import { artistCheck, openArtistReport } from "#lib/stores/library/artistCheck.store";
  import SegmentedControl from "#lib/components/ui/input/SegmentedControl.svelte";
  import { playlistStore } from "#lib/stores/playlist/playlist.store";
  import { runAutoExport, setPlaylistAutoExport, type ExportReport } from "#lib/services/playlist/autoExport.service";

  let showResetDialog = $state(false);
  let rescanning = $state(false);
  let fetchingImages = $state(false);
  let fetchingCovers = $state(false);
  let repairingLinks = $state(false);
  let dataDir = $state('');
  let exporting = $state(false);
  let importPath = $state<string | null>(null);
  let importReport = $state<ImportReport | null>(null);
  let importing = $state(false);
  let replaceExisting = $state(false);

  /** « 3 playlists » : clé `_one` ou `_n` selon le nombre. */
  const nb = (n: number, cle: string) =>
    $t(n === 1 ? `${cle}_one` : `${cle}_n`).replace("{n}", n.toLocaleString($currentLocale));

  /**
   * Choisit un fichier et annonce ce qu'il ferait, sans rien écrire.
   *
   * L'aperçu et l'import passent par le même code côté Rust, appelé une fois à
   * blanc puis une fois pour de bon : deux chemins distincts finiraient par
   * diverger, et l'aperçu mentirait.
   */
  async function handleImportPick() {
    const chemin = await open({
      title: $t("settings.import_dialog_title"),
      multiple: false,
      filters: [{ name: $t("settings.export_file_type"), extensions: ["json"] }],
    });
    if (!chemin || typeof chemin !== 'string') return;

    importing = true;
    try {
      importReport = await invoke<ImportReport>('preview_import', { path: chemin });
      importPath = chemin;
      replaceExisting = false;
    } catch (e) {
      console.error('Preview failed:', e);
      toasts.push({ type: "error", title: $t("settings.import_rejected"), message: String(e) });
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
        title: $t("settings.import_done"),
        message: [
          nb(bilan.playlists_created + bilan.playlists_replaced, "settings.backup_playlists"),
          nb(bilan.tracks_matched_by_path + bilan.tracks_matched_by_tags, "settings.import_done_found"),
          ...(manquants > 0 ? [nb(manquants, "settings.import_done_missing")] : []),
        ].join(", "),
      });
    } catch (e) {
      console.error('Import failed:', e);
      toasts.push({ type: "error", title: $t("settings.import_failed"), message: String(e) });
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
      title: $t("settings.export_dialog_title"),
      defaultPath: `rustmusic-${jour}.json`,
      filters: [{ name: $t("settings.export_file_type"), extensions: ["json"] }],
    });

    // Annulation du sélecteur : ce n'est pas une erreur, on ne dit rien.
    if (!chemin) return;

    exporting = true;
    try {
      const bilan = await invoke<ExportSummary>('export_settings_and_playlists', { path: chemin });
      toasts.push({
        type: "success",
        title: $t("settings.export_done"),
        message:
          `${nb(bilan.settings, "settings.export_done_settings")}, ${nb(bilan.playlists, "settings.backup_playlists")} ` +
          `(${nb(bilan.tracks, "settings.export_done_tracks")}), ${nb(bilan.liked, "settings.export_done_liked")}`,
      });
    } catch (e) {
      console.error('Export failed:', e);
      toasts.push({ type: "error", title: $t("settings.export_failed"), message: String(e) });
    } finally {
      exporting = false;
    }
  }

  async function handleRescan() {
    rescanning = true;
    // Un arrêt vaut pour toutes les bibliothèques.
    let arrete = false;
    const ecoute = await listen<{ cancelled: boolean }>('rescan-complete', (e) => { arrete ||= e.payload.cancelled; });
    try {
      const libraries = $libraryStore.libraries;
      for (const lib of libraries) {
        await invoke('rescan_library', { libraryId: lib.id });
        forgetLocations();
        if (arrete) break;
      }
    } catch (e) {
      console.error('Rescan failed:', e);
    } finally {
      ecoute();
      rescanning = false;
    }
  }

  // Relance aussi les artistes restés sans portrait ; ceux déjà trouvés ne sont pas retéléchargés.
  async function handleFetchArtistImages() {
    fetchingImages = true;
    try {
      await fetchPortraits(true);
    } finally {
      fetchingImages = false;
    }
  }

  let cleaningImages = $state(false);
  async function handleCleanImages() {
    cleaningImages = true;
    try {
      await cleanImages();
    } finally {
      cleaningImages = false;
    }
  }

  async function handleFetchAlbumCovers() {
    fetchingCovers = true;
    try {
      await fetchCovers($libraryStore.libraries.map((l) => l.id).filter((id): id is number => id != null));
    } finally {
      fetchingCovers = false;
    }
  }

  /** Voir `artist_link_repair`. Aucun fichier relu, le tag est déjà en base. */
  async function handleRepairArtistLinks() {
    repairingLinks = true;
    try {
      const bilan = await invoke<{
        tracks_seen: number;
        tracks_multi: number;
        links_created: number;
        artists_created: number;
        main_artist_fixed: number;
      }>('repair_artist_links', { libraryId: null });

      await libraryStore.refresh();

      toasts.push({
        type: 'success',
        title: $t('settings.repair_links'),
        message: bilan.links_created === 0 && bilan.main_artist_fixed === 0
          ? $t('settings.repair_links_none')
          : $t('settings.repair_links_done')
              .replace('{links}', String(bilan.links_created))
              .replace('{multi}', String(bilan.tracks_multi))
              .replace('{artists}', String(bilan.artists_created))
              .replace('{fixed}', String(bilan.main_artist_fixed)),
      });
    } catch (e) {
      console.error('Repair artist links failed:', e);
      toasts.push({
        type: 'error',
        title: $t('settings.repair_links'),
        message: String(e),
      });
    } finally {
      repairingLinks = false;
    }
  }

  async function handleOpenDataFolder() {
    // dataDir est vide tant que le onMount n'a pas résolu le chemin.
    if (!dataDir) return;
    await openPath(dataDir);
  }

  onMount(async () => {
      // La barre latérale, qui charge les playlists, est absente des réglages.
      if (!$playlistStore.playlists.length) void playlistStore.refresh();
      dataDir = await appDataDir();
  })

  // ─── Export automatique des playlists ───
  const exported = $derived($playlistStore.playlists.filter((p) => p.auto_export));
  // Celles qui peuvent l'être : de cet ordinateur, hors mix du jour.
  const exportable = $derived($playlistStore.playlists.filter((p) => !p.server_id && !p.is_mix));
  let exportingPlaylists = $state(false);
  let lastExport = $state<ExportReport | null>(null);

  async function chooseExportDir() {
    const dir = await open({ directory: true, title: $t("playlist_export.choose_dir") }).catch(() => null);
    if (typeof dir !== "string") return;
    await settingsStore.set("playlist_export_dir", dir);
    if (exported.length) void exportNow();
  }

  async function exportNow() {
    exportingPlaylists = true;
    lastExport = await runAutoExport();
    exportingPlaylists = false;
  }

  // Format ou chemins changés : les fichiers suivent aussitôt.
  async function setExportOption(key: "playlist_export_format" | "playlist_export_relative", value: string) {
    await settingsStore.set(key, value);
    if (exported.length && $settingsStore.playlist_export_dir) void exportNow();
  }

  const exportSummary = $derived.by(() => {
    if (!lastExport) return "";
    const r = lastExport;
    let text = $t("playlist_export.done").replace("{n}", String(r.playlists)).replace("{w}", String(r.written)).replace("{t}", r.tracks.toLocaleString($currentLocale));
    if (r.absolute) text += " · " + $t("playlist_export.absolute").replace("{n}", String(r.absolute));
    if (r.skipped) text += " · " + $t("playlist_export.skipped").replace("{n}", String(r.skipped));
    return text;
  });
</script>

<!-- ─── Bibliothèques ─── -->
<OptionGroup foldKey="settings.storage_libraries_group" title={$t("settings.storage_libraries_group")} hint={$t("settings.storage_libraries_group_hint")}>
  <OptionItem title={$t("settings.rescan_library")} desc={$t("settings.rescan_library_desc")} keywords="scan">
    <GhostButton icon="material-symbols:sync-rounded" label={$t("settings.rescan_btn")} busy={rescanning} onclick={handleRescan} />
  </OptionItem>
  <OptionItem title={$t("settings.storage_artist_images")} desc={$t("settings.storage_artist_images_desc")} keywords="deezer portrait">
    <GhostButton
      icon="material-symbols:download-rounded"
      label={fetchingImages ? $t("common.fetching") : $t("common.fetch")}
      busy={fetchingImages}
      onclick={handleFetchArtistImages}
    />
  </OptionItem>
  <OptionItem title={$t("settings.album_covers")} desc={$t("settings.album_covers_desc")} keywords="deezer pochette">
    <GhostButton
      icon="material-symbols:download-rounded"
      label={fetchingCovers ? $t("common.fetching") : $t("common.fetch")}
      busy={fetchingCovers}
      onclick={handleFetchAlbumCovers}
    />
  </OptionItem>
  <OptionItem title={$t("settings.clean_images")} desc={$t("settings.clean_images_desc")} keywords="cache images pochettes portraits espace">
    <GhostButton icon="material-symbols:cleaning-services-outline-rounded" label={$t("settings.clean_images_btn")} busy={cleaningImages} onclick={handleCleanImages} />
  </OptionItem>
  <OptionItem title={$t("settings.repair_links")} desc={$t("settings.repair_links_desc")} keywords="feat artistes">
    <GhostButton icon="material-symbols:link-rounded" label={$t("settings.repair_links_btn")} busy={repairingLinks} onclick={handleRepairArtistLinks} />
  </OptionItem>
</OptionGroup>

<!-- ─── Artistes multiples ─── -->
<OptionGroup foldKey="settings.artist_split_group" title={$t("artist_split.title")} hint={$t("artist_split.group_hint")}>
  <OptionItem title={$t("artist_split.check_now")} keywords="artistes multiples virgule musicbrainz feat séparer"
              desc={$artistCheck.running
                ? $t("artist_split.progress").replace("{done}", String($artistCheck.done)).replace("{total}", String($artistCheck.total)) + ($artistCheck.current ? ` · ${$artistCheck.current}` : "")
                : $t("artist_split.check_now_desc")}>
    <GhostButton icon="material-symbols:group-search-rounded" label={$t("artist_split.check_btn")} onclick={() => artistCheck.start()} />
  </OptionItem>
  <OptionItem title={$t("artist_split.auto")} desc={$t("artist_split.auto_desc")} keywords="artistes musicbrainz scan automatique" onclick={() => settingsStore.toggle("artist_check_auto")}>
    <ToggleSwitch checked={$settingsStore.artist_check_auto === "true"} label={$t("artist_split.auto")} onclick={() => settingsStore.toggle("artist_check_auto")} />
  </OptionItem>
  <OptionItem title={$t("artist_split.review")} desc={$t("artist_split.review_desc")} keywords="artistes annuler corrections">
    <GhostButton icon="lucide:undo-2" label={$t("artist_split.review_btn")} onclick={() => openArtistReport()} />
  </OptionItem>
</OptionGroup>

<!-- ─── Export automatique des playlists (clé USB, téléphone) ─── -->
<OptionGroup foldKey="settings.playlist_export_group" title={$t("playlist_export.title")} hint={$t("playlist_export.hint")}>
  <OptionItem title={$t("playlist_export.dir")} keywords="m3u m3u8 export playlist clé usb téléphone android dossier"
              desc={$settingsStore.playlist_export_dir || $t("playlist_export.dir_none")}>
    <GhostButton icon="material-symbols:folder-open-outline-rounded"
                 label={$t($settingsStore.playlist_export_dir ? "playlist_export.change_dir" : "playlist_export.choose_dir_btn")} onclick={chooseExportDir} />
  </OptionItem>
  <OptionItem title={$t("playlist_export.format")} desc={$t("playlist_export.format_desc")} keywords="m3u m3u8 format">
    <SegmentedControl value={$settingsStore.playlist_export_format === "m3u" ? "m3u" : "m3u8"} label={$t("playlist_export.format")}
                      options={[{ value: "m3u8", label: ".m3u8" }, { value: "m3u", label: ".m3u" }]}
                      onchange={(v) => setExportOption("playlist_export_format", v)} />
  </OptionItem>
  <OptionItem title={$t("playlist_export.relative")} desc={$t("playlist_export.relative_desc")} keywords="chemins relatifs android"
              onclick={() => setExportOption("playlist_export_relative", $settingsStore.playlist_export_relative === "false" ? "true" : "false")}>
    <ToggleSwitch checked={$settingsStore.playlist_export_relative !== "false"} label={$t("playlist_export.relative")}
                  onclick={() => setExportOption("playlist_export_relative", $settingsStore.playlist_export_relative === "false" ? "true" : "false")} />
  </OptionItem>
  {#if $settingsStore.playlist_export_dir}
    {#each exportable as pl (pl.id)}
      <OptionItem title={pl.name} keywords="export m3u playlist {pl.name}"
                  desc={$t(pl.auto_export ? "playlist_export.row_on" : "playlist_export.row_off")}
                  onclick={() => setPlaylistAutoExport(pl.id, pl.name, !pl.auto_export)}>
        {#snippet lead()}<Icon icon={pl.is_smart ? "lucide:sparkles" : (pl.icon ?? "lucide:list-music")} width="18" style="color: {pl.color}" />{/snippet}
        <ToggleSwitch checked={!!pl.auto_export} label={pl.name} onclick={() => setPlaylistAutoExport(pl.id, pl.name, !pl.auto_export)} />
      </OptionItem>
    {:else}
      <OptionItem title={$t("playlist_export.no_playlist")} desc={$t("playlist_export.no_playlist_desc")} />
    {/each}
  {/if}
  <OptionItem title={$t("playlist_export.now")} keywords="exporter maintenant m3u"
              desc={exportSummary || (exported.length
                ? $t("playlist_export.marked").replace("{n}", String(exported.length)).replace("{names}", exported.slice(0, 3).map((p) => p.name).join(", ") + (exported.length > 3 ? "…" : ""))
                : $t($settingsStore.playlist_export_dir ? "playlist_export.none_marked" : "playlist_export.choose_first"))}>
    <GhostButton icon="material-symbols:sync-saved-locally-outline-rounded" label={$t("playlist_export.now_btn")} busy={exportingPlaylists}
                 disabled={!exported.length || !$settingsStore.playlist_export_dir} onclick={exportNow} />
  </OptionItem>
</OptionGroup>

<!-- ─── Sauvegarde ─── -->
<ServersSettings />

<OptionGroup foldKey="settings.backup_group" title={$t("settings.backup_group")} hint={$t("settings.backup_group_hint")}>
  <OptionItem title={$t("settings.export_title")} desc={$t("settings.export_desc")} keywords="json sauvegarde">
    <GhostButton
      icon="material-symbols:download-rounded"
      label={exporting ? $t("settings.export_running") : $t("settings.export_btn")}
      busy={exporting}
      onclick={handleExport}
    />
  </OptionItem>
  <OptionItem title={$t("settings.import_title")} desc={$t("settings.import_desc")} keywords="json restaurer">
    <GhostButton
      icon="material-symbols:upload-rounded"
      label={importing && !importReport ? $t("settings.import_reading") : $t("settings.import_btn")}
      busy={importing && !importReport}
      onclick={handleImportPick}
    />
  </OptionItem>
</OptionGroup>

<!-- ─── Données ─── -->
<OptionGroup foldKey="settings.data_group" title={$t("settings.data_group")} hint={$t("settings.data_group_hint")}>
  <OptionItem title={$t("settings.open_data_folder")} desc={$t("settings.open_data_folder_desc")} value={dataDir} keywords="dossier base covers">
    <GhostButton icon="material-symbols:folder-open-outline-rounded" label={$t("settings.open_btn")} onclick={handleOpenDataFolder} />
  </OptionItem>
  <OptionItem title={$t("settings.reset")} desc={$t("settings.reset_desc")} keywords="effacer supprimer">
    <GhostButton icon="material-symbols:delete-outline-rounded" label={$t("settings.reset_btn")} danger onclick={() => (showResetDialog = true)} />
  </OptionItem>
</OptionGroup>

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
