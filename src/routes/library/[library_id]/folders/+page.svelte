<script lang="ts">
  import SearchField from "#lib/components/ui/input/SearchField.svelte";
  import MenuSelect from "#lib/components/ui/menu/MenuSelect.svelte";
  import ViewModeSwitch from "#lib/components/ui/input/ViewModeSwitch.svelte";
  import Icon from "@iconify/svelte";
  import { onDestroy } from "svelte";
  import { page } from "$app/state";
  import { goto } from "$app/navigation";
  import { invoke } from "@tauri-apps/api/core";
  import { t, currentLocale } from "#lib/i18n";
  import { viewMode } from "#lib/stores/ui/viewMode.store";
  import { libraryHeader } from "#lib/stores/library/libraryHeader";
  import { libraryStore } from "#lib/stores/library/library.store";
  import { tagWorkshop } from "#lib/stores/tags/tagWorkshop.store";
  import { toasts } from "#lib/stores/ui/toast.store";
  import { canWriteTags } from "#lib/services/tags/tagEditor.service";
  import { handlePlayTrack } from "#lib/actions/player/PlayerAction";
  import { toQueue } from "#lib/mapper/queue/mapQueueTrack";
  import { readableSize } from "#lib/helper/tools/sizeTools";
  import { ilYA } from "#lib/helper/tools/dateTools";
  import { sortKey } from "#lib/helper/library/sortKey";
  import LibraryImportingLoader from "#lib/components/library/common/loader/LibraryImportingLoader.svelte";
  import { importAffiche, importsTermines } from "#lib/stores/library/importProgress.store";
  import TrackContextMenu from "#lib/components/ui/contextmenu/TrackContextMenu.svelte";
  import type { LibraryDir } from "#lib/types/db/library/LibraryDir";
  import SelectionToggle from "#lib/components/ui/selection/SelectionToggle.svelte";
  import SelectionCheck from "#lib/components/ui/selection/SelectionCheck.svelte";
  import { selectionStore } from "#lib/stores/ui/selection.store";
  import { groupKey, toggleGroupSelection } from "#lib/helper/tools/selectionGroups";
  import CoverMosaic from "#lib/components/ui/image/CoverMosaic.svelte";
  import FolderArt from "#lib/components/ui/image/FolderArt.svelte";
  import FileArt from "#lib/components/ui/image/FileArt.svelte";
  import ToolbarButton from "#lib/components/ui/button/ToolbarButton.svelte";
  import CoverImg from "#lib/components/ui/image/CoverImg.svelte";
  import { minutesSeconds } from "#lib/helper/tools/dateTools";
  import type { PalierPiste } from "#lib/helper/tools/audioFormatTools";
  import { readLocal, writeLocal } from "#lib/helper/tools/storage";
  import QualityBadge from "#lib/components/ui/text/QualityBadge.svelte";
  import { formatCourt, trackTier } from "#lib/helper/tools/audioFormatTools";

  type DirEntry = {
    name: string; path: string; is_dir: boolean; size: number; extension: string | null;
    title: string | null; artist: string | null; duration: number | null; thumbnail_path: string | null;
    track_id: string | null;
    preview: FolderPreview | null;
  };
  type FolderPreview = {
    covers: string[]; track_count: number; album_count: number;
    best_format: string | null; best_bits: number | null; best_rate: number | null;
  };
  // Option de la barre : pochettes des sous-dossiers, ou dossier illustré. Retenue d'une session à l'autre.
  let avecPochettes = $state(readLocal("folderCovers", "1") !== "0");
  $effect(() => writeLocal("folderCovers", avecPochettes ? "1" : "0"));
  const pochettesDe = (e: DirEntry) => (avecPochettes ? e.preview?.covers ?? [] : []);
  // Sans profondeur connue au niveau du fichier : le DSD en or, le reste selon le format.
  const paliersFichier = (format: string): PalierPiste => (format === "dsf" || format === "dff" ? "hires" : trackTier(format, null, null));
  /** « DSD128 », « FLAC 24/96 », « AAC » : la meilleure piste du dossier. */
  const qualite = (p: FolderPreview | null) =>
    p?.best_format ? { palier: trackTier(p.best_format, p.best_bits, p.best_rate), texte: formatCourt(p.best_format, p.best_bits, p.best_rate) } : null;

  const libraryId = $derived(Number(page.params.library_id));
  const currentLibrary = $derived($libraryStore.libraries.find((l) => l.id === libraryId));

  // ─── Navigation : racines importées, puis l'arborescence ───
  let rootDirs = $state<LibraryDir[]>([]);
  let entries = $state<DirEntry[]>([]);
  let loading = $state(true);
  let currentPath = $state<string | null>(null);
  // Le fil d'Ariane fait foi : un niveau par entrée, avec le chemin exact rendu par le moteur.
  let breadcrumb = $state<{ name: string; path: string }[]>([]);
  let contextMenu = $state<{ x: number; y: number; entry: DirEntry } | null>(null);
  const isRoot = $derived(currentPath === null);

  $effect(() => {
    const id = libraryId;
    void $importsTermines;
    loading = true;
    invoke<LibraryDir[]>("get_library_dirs", { libraryId: id })
      .then((d) => { if (id === libraryId) rootDirs = d ?? []; })
      .catch((e) => console.error("Failed to load dirs:", e))
      .finally(() => (loading = false));
  });

  async function navigateTo(path: string, name: string) {
    loading = true;
    currentPath = path;
    recherche = "";
    const i = breadcrumb.findIndex((b) => b.path === path);
    breadcrumb = i >= 0 ? breadcrumb.slice(0, i + 1) : breadcrumb.length === 0 ? [{ name, path }] : [...breadcrumb, { name, path }];
    try {
      const r = await invoke<DirEntry[]>("list_directory", { libraryId, path });
      // Un autre dossier a pu être ouvert entre-temps (NAS lent).
      if (currentPath === path) entries = r;
    } catch (e) {
      console.error("Failed to list directory:", e);
      if (currentPath === path) entries = [];
    } finally {
      if (currentPath === path) loading = false;
    }
  }

  // Au-delà de quatre niveaux, le milieu se replie en « … » : le premier et les deux derniers restent.
  type Miette = { crumb: { name: string; path: string } | null; dernier: boolean; cache: { name: string; path: string }[] };
  const miettes = $derived.by((): Miette[] => {
    const n = breadcrumb.length;
    const m = (c: { name: string; path: string }, i: number): Miette => ({ crumb: c, dernier: i === n - 1, cache: [] });
    if (n <= 4) return breadcrumb.map(m);
    return [m(breadcrumb[0], 0), { crumb: null, dernier: false, cache: breadcrumb.slice(1, n - 2) }, m(breadcrumb[n - 2], n - 2), m(breadcrumb[n - 1], n - 1)];
  });

  function goToRoot() {
    currentPath = null;
    breadcrumb = [];
    entries = [];
    recherche = "";
  }

  function goToParent() {
    if (breadcrumb.length <= 1) return goToRoot();
    const parent = breadcrumb[breadcrumb.length - 2];
    navigateTo(parent.path, parent.name);
  }

  const fichiers = $derived(entries.filter((e) => !e.is_dir));

  // ─── Sélection : un fichier est une piste, un dossier apporte toutes les siennes ───
  const selection = $derived($selectionStore);
  const fileId = (e: DirEntry) => e.track_id ?? e.path;
  const asTrack = (e: DirEntry) => ({ id: fileId(e), path: e.path, title: e.title ?? e.name, artist: e.artist, duration: e.duration, thumbnail_path: e.thumbnail_path });
  const isEntryChecked = (e: DirEntry) => (e.is_dir ? selection.groupes.has(groupKey("path", libraryId, e.path)) : selection.ids.has(fileId(e)));
  const isRootChecked = (d: LibraryDir) => selection.groupes.has(groupKey("folder", libraryId, d.id));

  // Maj + clic : la plage suit l'ordre affiché des fichiers.
  $effect(() => {
    selectionStore.setOrder(visibles.filter((e) => !e.is_dir).map((e) => ({ id: fileId(e), track: asTrack(e) })));
  });

  function openEntry(entry: DirEntry, e: MouseEvent) {
    if (!selection.active) return entry.is_dir ? navigateTo(entry.path, entry.name) : playFile(entry);
    if (entry.is_dir) toggleGroupSelection("path", libraryId, entry.path);
    else if (e.shiftKey) selectionStore.selectRange(fileId(entry));
    else selectionStore.toggle(fileId(entry), asTrack(entry));
  }
  function openRoot(dir: LibraryDir) {
    if (selection.active) toggleGroupSelection("folder", libraryId, dir.id);
    else navigateTo(dir.path, dir.name);
  }
  const dirCount = $derived(entries.length - fichiers.length);

  function playFile(entry: DirEntry) {
    handlePlayTrack(entry.path, toQueue(fichiers));
  }
  function playFolder() {
    if (fichiers.length) handlePlayTrack(fichiers[0].path, toQueue(fichiers));
  }

  // Les fichiers du niveau affiché partent dans l'atelier de tags (pas les sous-dossiers).
  let openingWorkshop = $state(false);
  async function openWorkshop() {
    const files = fichiers.map((e) => e.path);
    if (files.length === 0 || openingWorkshop) return;
    openingWorkshop = true;
    try {
      const checks = await Promise.all(files.map((p) => canWriteTags(p)));
      const writable = files.filter((_, i) => checks[i]);
      if (writable.length === 0) {
        toasts.push({ type: "info", title: $t("workshop.title"), message: $t("tags.none_writable") });
        return;
      }
      const folder = breadcrumb[breadcrumb.length - 1]?.name ?? $t("nav.folders");
      await tagWorkshop.load(writable, folder, `/library/${libraryId}/folders`);
      await goto(`/library/${libraryId}/tags`);
    } finally {
      openingWorkshop = false;
    }
  }

  // ─── Filtre et tri ───
  let recherche = $state("");
  type Tri = "name" | "files" | "size" | "scan";
  const TRIS: { cle: Tri; libelle: string; icone: string; racine: boolean }[] = [
    { cle: "name", libelle: "folders_view.sort_name", icone: "material-symbols:sort-by-alpha-rounded", racine: false },
    { cle: "files", libelle: "folders_view.sort_files", icone: "material-symbols:description-outline-rounded", racine: true },
    { cle: "size", libelle: "folders_view.sort_size", icone: "material-symbols:hard-drive-outline-rounded", racine: false },
    { cle: "scan", libelle: "folders_view.sort_scan", icone: "material-symbols:sync-rounded", racine: true },
  ];
  let tri = $state<Tri>("name");
  // Le nombre de fichiers et la date de scan n'existent qu'à la racine.
  const trisDispo = $derived(TRIS.filter((x) => isRoot || !x.racine));
  const triEffectif = $derived(trisDispo.some((x) => x.cle === tri) ? tri : "name");

  const comparer = new Intl.Collator(undefined, { numeric: true, sensitivity: "base" });
  const racines = $derived.by(() => {
    const q = recherche.trim().toLowerCase();
    const liste = rootDirs.filter((d) => !q || `${d.name} ${d.path}`.toLowerCase().includes(q));
    const f: Record<Tri, (a: LibraryDir, b: LibraryDir) => number> = {
      name: (a, b) => comparer.compare(sortKey(a.name), sortKey(b.name)),
      files: (a, b) => b.total_files - a.total_files,
      size: (a, b) => b.total_size - a.total_size,
      scan: (a, b) => String(b.last_scan_at ?? "").localeCompare(String(a.last_scan_at ?? "")),
    };
    return [...liste].sort(f[triEffectif]);
  });
  // Dans un dossier : les sous-dossiers d'abord, puis les fichiers.
  const visibles = $derived.by(() => {
    const q = recherche.trim().toLowerCase();
    const liste = entries.filter((e) => !q || e.name.toLowerCase().includes(q));
    return [...liste].sort((a, b) =>
      Number(b.is_dir) - Number(a.is_dir) || (triEffectif === "size" ? b.size - a.size : 0) || comparer.compare(a.name, b.name));
  });

  // ─── En-tête : dossiers suivis et fichiers ───
  $effect(() => {
    const n = rootDirs.length;
    const fichiersTotal = rootDirs.reduce((s, d) => s + (d.total_files ?? 0), 0);
    libraryHeader.update(() => ({
      chiffres: [
        { n, un: "library_head.folders_one", plusieurs: "library_head.folders_n" },
        { n: fichiersTotal, un: "library_head.files_one", plusieurs: "library_head.files_n" },
        { n: currentLibrary?.total_tracks ?? 0, un: "library_head.tracks_one", plusieurs: "library_head.tracks_n" },
      ],
    }));
  });
  onDestroy(() => libraryHeader.update((h) => ({ ...h, chiffres: null })));

  const formatNumber = (n: number) => n.toLocaleString($currentLocale);
  const nb = (n: number, un: string, plusieurs: string) => `${formatNumber(n)} ${$t(n === 1 ? un : plusieurs)}`;
  const ETATS: Record<string, { cle: string; classes: string }> = {
    scanning: { cle: "library_head.status_scanning", classes: "bg-(--rg-ambg) text-(--rg-am)" },
    error: { cle: "library_head.status_error", classes: "bg-red-500/10 text-red-600 dark:text-red-400" },
  };
  const statusOf = (d: LibraryDir) =>
    // Introuvable au scan (partage non monté…) : ce n'est pas une erreur de lecture.
    d.scan_status === "error" && d.scan_error === "unavailable"
      ? { ...ETATS.error, cle: "library_head.status_unavailable" }
      : ETATS[d.scan_status] ?? { cle: "library_head.status_ok", classes: "bg-(--rg-g)/10 text-(--rg-gtx)" };

  const bouton = "h-8.5 px-3 flex items-center gap-1.5 rounded-[10px] border text-[13px] font-semibold whitespace-nowrap cursor-pointer transition-colors bg-(--rg-carte) border-(--rg-bd) text-(--rg-tx2) hover:border-(--rg-bd2) hover:text-(--rg-tx) disabled:opacity-40";
  const tuileDossier = "shrink-0 rounded-xl flex items-center justify-center border";
</script>

<!-- Icône d'un dossier importé : le dossier illustré dans sa pastille ambre. -->
{#snippet icone(taille: string, coche: boolean | null = null)}
  <span class="relative {tuileDossier} {taille} bg-(--rg-ambg) border-(--rg-ambd)">
    <FolderArt music class="w-[78%] h-[78%] drop-shadow-[0_2px_3px_rgb(0_0_0/0.18)]" />
    {#if coche !== null}<SelectionCheck {coche} class="absolute -top-1.5 -left-1.5" />{/if}
  </span>
{/snippet}

<!-- Sous-dossier : ses pochettes, empilées s'il contient plusieurs albums ; son nom, ses titres, sa meilleure qualité. -->
{#snippet dossierTuile(entry: DirEntry)}
  {@const p = entry.preview}
  {@const covers = pochettesDe(entry)}
  {@const q = qualite(p)}
  {@const coche = isEntryChecked(entry)}
  <button type="button" class="group flex flex-col gap-2.5 w-full min-w-0 text-left cursor-pointer" onclick={(e) => openEntry(entry, e)} title={entry.name}>
    <span class="relative block aspect-square w-full mt-2">
      <!-- Feuillets derrière : le dossier contient plusieurs albums. -->
      {#if covers.length > 0 && (p?.album_count ?? 0) > 1}
        <span class="absolute inset-x-4 -top-2 h-6 rounded-t-[10px] bg-(--rg-s2) border border-(--rg-bd) opacity-70 transition-transform duration-300 group-hover:-translate-y-0.5"></span>
        <span class="absolute inset-x-2 -top-1 h-6 rounded-t-[11px] bg-(--rg-s2) border border-(--rg-bd) transition-transform duration-300 group-hover:-translate-y-px"></span>
      {/if}
      <span class="relative block h-full rounded-xl overflow-hidden border transition-[border-color,box-shadow] duration-200
                   {coche ? 'border-(--rg-g) ring-2 ring-(--rg-g)/40' : 'border-(--rg-bd) group-hover:border-(--rg-bd2)'}
                   shadow-[0_6px_18px_-8px_rgb(0_0_0/0.45)]">
        {#if covers.length > 0}
          <CoverMosaic {covers} class="absolute inset-0" imgClass="transition-transform duration-500 group-hover:scale-105" />
        {:else}
          <!-- Option coupée, ou rien de connu dedans : le dossier illustré, vinyle compris s'il contient de la musique. -->
          <span class="absolute inset-0 flex items-center justify-center bg-radial from-(--rg-ambg) from-10% to-(--rg-carte) to-75%">
            <FolderArt music={!!p} class="w-[58%] drop-shadow-[0_10px_14px_rgb(0_0_0/0.25)] transition-transform duration-300 group-hover:-translate-y-1 group-hover:scale-105" />
          </span>
        {/if}
        <!-- Pastille : sur les pochettes, « c'est un dossier » ; sur l'illustration, le nombre d'albums. -->
        {#if covers.length > 0 || (p?.album_count ?? 0) > 1}
          <span class="absolute left-2 bottom-2 h-6 px-1.5 flex items-center gap-1 rounded-md bg-black/60 text-white text-[11px] font-semibold">
            <Icon icon={covers.length > 0 ? "material-symbols:folder-rounded" : "material-symbols:album"} width={14} class="text-amber-300" />
            {#if p && p.album_count > 1}{formatNumber(p.album_count)}{/if}
          </span>
        {/if}
      </span>
      {#if selection.active}<SelectionCheck {coche} class="absolute top-1.5 left-1.5" />{/if}
    </span>
    <span class="min-w-0 px-0.5">
      <span class="line-clamp-2 wrap-break-word text-sm font-semibold leading-5 text-(--rg-tx)">{entry.name}</span>
      {#if p || q}
        <span class="mt-1 flex items-center gap-1.5 min-w-0 text-[12px] text-(--rg-mu)">
          {#if p}<span class="truncate">{nb(p.track_count, "library_head.tracks_one", "library_head.tracks_n")}</span>{/if}
          {#if q}<QualityBadge palier={q.palier} texte={q.texte} class="px-1.5 py-px shrink-0" />{/if}
        </span>
      {/if}
    </span>
  </button>
{/snippet}

<!-- Fichier : sa pochette (option) ou le fichier illustré ; son titre connu de la bibliothèque, sinon son nom. -->
{#snippet fichierTuile(entry: DirEntry)}
  {@const cover = avecPochettes ? entry.thumbnail_path : null}
  {@const format = (entry.extension ?? "").toLowerCase()}
  {@const coche = isEntryChecked(entry)}
  {@const infos = [entry.artist, entry.duration ? minutesSeconds(entry.duration) : null].filter(Boolean)}
  <button type="button" class="group flex flex-col gap-2.5 w-full min-w-0 text-left cursor-pointer" title={entry.name}
          onclick={(e) => openEntry(entry, e)}
          oncontextmenu={(e) => { e.preventDefault(); contextMenu = { x: e.clientX, y: e.clientY, entry }; }}>
    <span class="relative block aspect-square w-full">
      <span class="relative block h-full rounded-xl overflow-hidden border transition-[border-color,box-shadow] duration-200
                   {coche ? 'border-(--rg-g) ring-2 ring-(--rg-g)/40' : 'border-(--rg-bd) group-hover:border-(--rg-bd2)'}
                   shadow-[0_6px_18px_-8px_rgb(0_0_0/0.45)]">
        {#if cover}
          <CoverImg path={cover} size="2x" alt="" class="absolute inset-0 w-full h-full object-cover transition-transform duration-500 group-hover:scale-105" />
          <span class="absolute left-2 bottom-2 h-6 px-1.5 flex items-center rounded-md bg-black/60 text-white text-[10px] font-bold uppercase tracking-[0.06em]">{format}</span>
        {:else}
          <span class="absolute inset-0 flex items-center justify-center bg-radial from-(--rg-gbg) from-10% to-(--rg-carte) to-75%">
            <FileArt {format} palier={paliersFichier(format)} class="w-[54%] drop-shadow-[0_10px_14px_rgb(0_0_0/0.25)] transition-transform duration-300 group-hover:-translate-y-1 group-hover:scale-105" />
          </span>
        {/if}
        <!-- Lecture au survol : un clic sur la tuile lance le morceau. -->
        {#if !selection.active}
          <span class="absolute right-2 bottom-2 w-9 h-9 flex items-center justify-center rounded-full bg-(--rg-g) text-(--rg-on-g) shadow-lg
                       opacity-0 translate-y-1 group-hover:opacity-100 group-hover:translate-y-0 transition duration-200">
            <Icon icon="material-symbols:play-arrow-rounded" width="22" />
          </span>
        {/if}
      </span>
      {#if selection.active}<SelectionCheck {coche} class="absolute top-1.5 left-1.5" />{/if}
    </span>
    <span class="min-w-0 px-0.5">
      <span class="line-clamp-2 wrap-break-word text-sm font-semibold leading-5 text-(--rg-tx)">{entry.title ?? entry.name.replace(/\.[^.]+$/, "")}</span>
      <span class="mt-1 block truncate text-[12px] text-(--rg-mu)">
        {infos.length ? infos.join(" · ") : entry.size > 0 ? readableSize(entry.size, $currentLocale) : ""}
      </span>
    </span>
  </button>
{/snippet}

{#if $importAffiche}
  <LibraryImportingLoader />
{:else}
  <div class="flex flex-col h-full">
    <!-- ─── Outils : recherche, actions du dossier, tri, vue ─── -->
    <div class="@container shrink-0 pl-4 md:pl-8 pr-4 md:pr-14 pt-3 pb-2">
      <!-- Une seule ligne : la recherche absorbe la place qui varie, les libellés s'effacent par priorité
           (Pochettes, puis Lire et Tags, puis le tri). On ne passe à la ligne qu'en très petite largeur. -->
      <div class="flex flex-wrap @3xl:flex-nowrap items-center gap-2">
        <SearchField bind:value={recherche} class="flex-1 min-w-40 max-w-72" clearLabel={$t("folders_view.clear")}
                     placeholder={isRoot ? $t("folders_view.filter_dirs").replace("{n}", formatNumber(rootDirs.length)) : $t("folders_view.filter_entries")} />

        {#if !isRoot && fichiers.length > 0}
          <button type="button" class="{bouton} shrink-0" onclick={playFolder} title={$t("folders_view.play_folder")}>
            <Icon icon="material-symbols:play-arrow-rounded" width="18" class="text-(--rg-g)" /><span class="@max-5xl:hidden">{$t("folders_view.play_folder")}</span>
          </button>
          <button type="button" class="{bouton} shrink-0" onclick={openWorkshop} disabled={openingWorkshop} title={$t("workshop.fix_tags")}>
            <Icon icon={openingWorkshop ? "material-symbols:progress-activity" : "material-symbols:table-edit-outline-rounded"} width="18" class={openingWorkshop ? "animate-spin" : ""} />
            <span class="@max-5xl:hidden">{$t("workshop.fix_tags")}</span>
          </button>
        {/if}

        <span class="flex-1 min-w-0"></span>

        <span class="shrink-0 flex items-center gap-2">
          <MenuSelect value={triEffectif} options={trisDispo.map((x) => ({ value: x.cle, label: $t(x.libelle), icon: x.icone }))}
                      onchange={(v) => (tri = v as typeof tri)} labelClass="@max-4xl:hidden" menuClass="w-55" />
          {#if !isRoot && entries.length > 0}
            <!-- Dossiers et fichiers : leurs pochettes, ou leur illustration. -->
            <ToolbarButton icon={avecPochettes ? "material-symbols:photo-library-rounded" : "material-symbols:photo-library-outline-rounded"}
                           pressed={avecPochettes} label={$t("folders_view.covers")} title={$t("folders_view.covers_hint")} labelClass="@max-6xl:hidden"
                           onclick={() => (avecPochettes = !avecPochettes)} />
          {/if}
          <ViewModeSwitch />
          <SelectionToggle />
        </span>
      </div>

      <!-- Fil d'Ariane : racine › dossiers ouverts ; à droite, le contenu du niveau. -->
      {#if !isRoot}
        <div class="mt-3 flex items-center gap-3 min-w-0">
          <!-- Capsule : parent | racine › … › niveaux › niveau courant -->
          <nav class="flex items-center min-w-0 h-10 pl-1 pr-1.5 rounded-xl border border-(--rg-bd) bg-(--rg-carte) text-[13px]" aria-label={$t("nav.folders")}>
            <button type="button" class="w-8 h-8 shrink-0 flex items-center justify-center rounded-lg cursor-pointer text-(--rg-mu) hover:bg-(--rg-s2) hover:text-(--rg-tx) transition-colors"
                    title={$t("common.parent_folder")} aria-label={$t("common.parent_folder")} onclick={goToParent}>
              <Icon icon="material-symbols:arrow-upward-rounded" width="18" />
            </button>
            <span class="w-px h-5 mx-1 shrink-0 bg-(--rg-bd)"></span>
            <button type="button" class="shrink-0 h-8 flex items-center gap-1.5 px-2 rounded-lg cursor-pointer text-(--rg-mu) hover:text-(--rg-tx) hover:bg-(--rg-s2) transition-colors" onclick={goToRoot}>
              <Icon icon="material-symbols:hard-drive-outline-rounded" width="16" /><span class="@max-xl:hidden">{$t("folders_view.root")}</span>
            </button>
            {#each miettes as m (m.crumb?.path ?? "…")}
              <Icon icon="material-symbols:chevron-right-rounded" width="16" class="shrink-0 text-(--rg-mu2)" />
              {#if !m.crumb}
                <!-- Niveaux repliés : un clic remonte au dernier d'entre eux. -->
                <button type="button" class="shrink-0 h-8 px-2 rounded-lg cursor-pointer text-(--rg-mu) hover:text-(--rg-tx) hover:bg-(--rg-s2)"
                        title={m.cache.map((c) => c.name).join(" › ")} onclick={() => navigateTo(m.cache[m.cache.length - 1].path, m.cache[m.cache.length - 1].name)}>…</button>
              {:else if !m.dernier}
                <button type="button" class="min-w-0 max-w-44 h-8 truncate px-2 rounded-lg cursor-pointer text-(--rg-mu) hover:text-(--rg-tx) hover:bg-(--rg-s2) transition-colors"
                        title={m.crumb.path} onclick={() => navigateTo(m.crumb!.path, m.crumb!.name)}>{m.crumb.name}</button>
              {:else}
                <span class="min-w-0 h-8 flex items-center gap-1.5 pl-1.5 pr-2.5 rounded-lg bg-(--rg-s2) font-semibold text-(--rg-tx)" title={m.crumb.path} aria-current="page">
                  <FolderArt music={fichiers.length > 0} class="w-5 h-5 shrink-0" />
                  <span class="truncate">{m.crumb.name}</span>
                </span>
              {/if}
            {/each}
          </nav>
          <!-- Contenu du niveau -->
          <span class="ml-auto shrink-0 flex items-center gap-1.5 text-xs text-(--rg-mu)">
            {#if dirCount > 0}
              <span class="h-7 flex items-center gap-1.5 px-2.5 rounded-lg border border-(--rg-bd) bg-(--rg-carte)">
                <Icon icon="material-symbols:folder-rounded" width="14" class="text-(--rg-am)" />{nb(dirCount, "library_head.folders_one", "library_head.folders_n")}
              </span>
            {/if}
            {#if fichiers.length > 0}
              <span class="h-7 flex items-center gap-1.5 px-2.5 rounded-lg border border-(--rg-bd) bg-(--rg-carte)">
                <Icon icon="material-symbols:music-note-rounded" width="14" class="text-(--rg-g)" />{nb(fichiers.length, "library_head.files_one", "library_head.files_n")}
              </span>
            {/if}
          </span>
        </div>
      {/if}
    </div>

    <!-- ─── Contenu ─── -->
    <div class="flex-1 relative min-h-0">
      <div class="absolute inset-0 overflow-y-auto scrollbar-app pl-4 md:pl-8 pr-4 md:pr-14 pb-16">
        {#if loading}
          <div class="flex items-center justify-center py-20 text-(--rg-mu)">
            <Icon icon="material-symbols:progress-activity" width="26" class="animate-spin" />
          </div>

        {:else if isRoot && rootDirs.length === 0}
          <div class="flex flex-col items-center justify-center py-20 text-center">
            <div class="w-16 h-16 mb-5 rounded-2xl border flex items-center justify-center bg-(--rg-gbg) border-(--rg-gbd) text-(--rg-g)">
              <Icon icon="material-symbols:folder-open-outline-rounded" width="30" />
            </div>
            <h3 class="text-base font-semibold text-(--rg-tx) mb-1.5">{$t("common.no_imported_folder")}</h3>
            <p class="text-sm text-(--rg-mu) max-w-xs">{$t("common.no_imported_folder_desc")}</p>
          </div>

        {:else if isRoot}
          {#if racines.length === 0}
            <p class="py-16 text-center text-sm text-(--rg-mu)">{$t("folders_view.no_match")}</p>
          {:else if $viewMode === "list"}
            <!-- Racines en liste : nom et chemin, fichiers, poids, dernier scan. -->
            <div class="flex flex-col pt-2">
              {#each racines as dir (dir.id)}
                {@const etat = statusOf(dir)}
                <button type="button"
                        class="group relative grid grid-cols-[48px_minmax(0,1fr)_100px_90px_150px_40px] max-[900px]:grid-cols-[48px_minmax(0,1fr)_90px_40px] items-center gap-4 py-2 pl-2 pr-3 rounded-xl text-left cursor-pointer transition-colors
                               {isRootChecked(dir) ? 'bg-(--rg-creux-on)' : 'hover:bg-(--rg-carte)'}
                               before:absolute before:left-18 before:right-3 before:top-0 before:h-px [button+&]:before:bg-(--rg-line) hover:before:opacity-0 [&:hover+button]:before:opacity-0"
                        onclick={() => openRoot(dir)}>
                  {@render icone("w-12 h-12", selection.active ? isRootChecked(dir) : null)}
                  <span class="min-w-0">
                    <span class="block truncate text-base font-semibold text-(--rg-tx)">{dir.name}</span>
                    <span class="block mt-0.5 truncate font-mono text-xs text-(--rg-mu)" title={dir.path}>{dir.path}</span>
                  </span>
                  <span class="text-right text-[13px] tabular-nums text-(--rg-mu)"><b class="font-medium text-(--rg-tx2)">{formatNumber(dir.total_files)}</b> {$t(dir.total_files === 1 ? "library_head.files_one" : "library_head.files_n")}</span>
                  <span class="text-right text-[13px] tabular-nums text-(--rg-mu) max-[900px]:hidden">{readableSize(dir.total_size, $currentLocale)}</span>
                  <span class="flex justify-end max-[900px]:hidden">
                    <span class="flex items-center gap-1.5 px-2 py-0.75 rounded-[10px] text-[11px] font-semibold whitespace-nowrap {etat.classes}">
                      <span class="w-1.5 h-1.5 rounded-full bg-current"></span>{dir.last_scan_at ? ilYA(dir.last_scan_at, $currentLocale, "short") : $t(etat.cle)}
                    </span>
                  </span>
                  <Icon icon="material-symbols:chevron-right-rounded" width="19" class="justify-self-end text-(--rg-mu2) group-hover:text-(--rg-tx2)" />
                </button>
              {/each}
            </div>
          {:else}
            <!-- Racines en cartes : de quoi reconnaître un dossier sans l'ouvrir. -->
            <div class="grid grid-cols-[repeat(auto-fill,minmax(240px,1fr))] gap-4 pt-3">
              {#each racines as dir (dir.id)}
                {@const etat = statusOf(dir)}
                <button type="button"
                        class="group flex flex-col gap-3.5 p-4 rounded-2xl border text-left cursor-pointer transition-colors
                               {isRootChecked(dir) ? 'bg-(--rg-creux-on) border-(--rg-g)' : 'bg-(--rg-carte) border-(--rg-bd) hover:border-(--rg-bd2)'}"
                        onclick={() => openRoot(dir)}>
                  <span class="flex items-start justify-between gap-3">
                    {@render icone("w-12 h-12", selection.active ? isRootChecked(dir) : null)}
                    <span class="flex items-center gap-1.5 px-2 py-0.75 rounded-[10px] text-[11px] font-semibold whitespace-nowrap {etat.classes}">
                      <span class="w-1.5 h-1.5 rounded-full bg-current"></span>{$t(etat.cle)}
                    </span>
                  </span>
                  <span class="min-w-0">
                    <span class="block truncate text-[15px] font-bold text-(--rg-tx)" title={dir.name}>{dir.name}</span>
                    <span class="block mt-0.5 truncate font-mono text-[11px] text-(--rg-mu)" title={dir.path}>{dir.path}</span>
                  </span>
                  <span class="flex items-center gap-1.5 text-[13px] text-(--rg-mu) whitespace-nowrap">
                    <span><b class="font-semibold text-(--rg-tx2)">{formatNumber(dir.total_files)}</b> {$t(dir.total_files === 1 ? "library_head.files_one" : "library_head.files_n")}</span>
                    {#if dir.total_size}<span>· {readableSize(dir.total_size, $currentLocale)}</span>{/if}
                    {#if dir.last_scan_at}<span class="ml-auto font-mono text-[11px] text-(--rg-mu2)">{ilYA(dir.last_scan_at, $currentLocale, "short")}</span>{/if}
                  </span>
                </button>
              {/each}
            </div>
          {/if}

        {:else if entries.length === 0}
          <div class="flex flex-col items-center justify-center py-20 text-center text-(--rg-mu)">
            <Icon icon="material-symbols:folder-off-outline-rounded" width="32" class="mb-3" />
            <p class="text-sm">{$t("common.empty_folder")}</p>
          </div>

        {:else if visibles.length === 0}
          <p class="py-16 text-center text-sm text-(--rg-mu)">{$t("folders_view.no_match")}</p>

        {:else if $viewMode === "grid"}
          <!-- Sous-dossiers en pochettes (ce qu'ils contiennent), puis les fichiers en tuiles compactes. -->
          {#if visibles.some((e) => e.is_dir)}
            <div class="grid grid-cols-[repeat(auto-fill,minmax(168px,1fr))] gap-x-4 gap-y-5 pt-4">
              {#each visibles.filter((e) => e.is_dir) as entry (entry.path)}
                {@render dossierTuile(entry)}
              {/each}
            </div>
          {/if}
          {#if visibles.some((e) => !e.is_dir)}
            <div class="grid grid-cols-[repeat(auto-fill,minmax(168px,1fr))] gap-x-4 gap-y-5 {dirCount > 0 ? 'pt-7' : 'pt-4'}">
              {#each visibles.filter((e) => !e.is_dir) as entry (entry.path)}
                {@render fichierTuile(entry)}
              {/each}
            </div>
          {/if}

        {:else}
          <!-- Entrées en liste : nom, format, poids ; lecture au survol des fichiers. -->
          <div class="flex flex-col pt-2">
            {#each visibles as entry (entry.path)}
              <button type="button"
                      class="group relative grid grid-cols-[40px_minmax(0,1fr)_88px_90px_40px] items-center gap-4 py-1.5 pl-2 pr-3 rounded-xl text-left cursor-pointer transition-colors
                             {isEntryChecked(entry) ? 'bg-(--rg-creux-on)' : 'hover:bg-(--rg-carte)'}
                             before:absolute before:left-16 before:right-3 before:top-0 before:h-px [button+&]:before:bg-(--rg-line) hover:before:opacity-0 [&:hover+button]:before:opacity-0"
                      onclick={(e) => openEntry(entry, e)}
                      oncontextmenu={(e) => { if (!entry.is_dir) { e.preventDefault(); contextMenu = { x: e.clientX, y: e.clientY, entry }; } }}>
                {#if entry.is_dir && pochettesDe(entry).length}
                  <!-- Le sous-dossier montre sa première pochette, marquée d'un coin de dossier. -->
                  <span class="relative w-10 h-10">
                    <CoverMosaic covers={pochettesDe(entry).slice(0, 1)} class="absolute inset-0 rounded-[10px] overflow-hidden border border-(--rg-bd)" />
                    <span class="absolute -right-1 -bottom-1 w-4.5 h-4.5 flex items-center justify-center rounded-[5px] bg-(--rg-ambg) border border-(--rg-ambd) text-(--rg-am)">
                      <Icon icon="material-symbols:folder-rounded" width={11} />
                    </span>
                    {#if selection.active}<SelectionCheck coche={isEntryChecked(entry)} class="absolute -top-1.5 -left-1.5" />{/if}
                  </span>
                {:else if entry.is_dir}
                  <span class="relative w-10 h-10 flex items-center justify-center">
                    <FolderArt music={!!entry.preview} class="w-10 h-10 drop-shadow-[0_2px_3px_rgb(0_0_0/0.2)]" />
                    {#if selection.active}<SelectionCheck coche={isEntryChecked(entry)} class="absolute -top-1.5 -left-1.5" />{/if}
                  </span>
                {:else}
                  <!-- Fichier : sa pochette (option), sinon le fichier illustré. -->
                  <span class="relative w-10 h-10 flex items-center justify-center">
                    {#if avecPochettes && entry.thumbnail_path}
                      <CoverImg path={entry.thumbnail_path} size="1x" alt="" class="w-10 h-10 rounded-[10px] object-cover border border-(--rg-bd)" />
                    {:else}
                      <FileArt format={entry.extension ?? ""} palier={paliersFichier((entry.extension ?? "").toLowerCase())} class="w-10 h-10 drop-shadow-[0_2px_3px_rgb(0_0_0/0.2)]" />
                    {/if}
                    {#if selection.active}<SelectionCheck coche={isEntryChecked(entry)} class="absolute -top-1.5 -left-1.5" />{/if}
                  </span>
                {/if}
                <span class="min-w-0 truncate text-sm {entry.is_dir ? 'font-semibold text-(--rg-tx)' : 'text-(--rg-tx2)'}" title={entry.name}>{entry.name}</span>
                <span class="justify-self-end">
                  {#if entry.is_dir && qualite(entry.preview)}
                    {@const q = qualite(entry.preview)!}
                    <QualityBadge palier={q.palier} texte={q.texte} class="px-1.5 py-0.5" />
                  {:else if entry.extension}
                    <span class="px-1.5 py-0.5 rounded-[5px] border text-[10px] font-bold uppercase tracking-[0.06em] bg-(--rg-s2) border-(--rg-bd) text-(--rg-mu)">{entry.extension}</span>
                  {/if}
                </span>
                <span class="text-right text-[13px] tabular-nums text-(--rg-mu) whitespace-nowrap">
                  {#if entry.is_dir}{entry.preview ? nb(entry.preview.track_count, "library_head.tracks_one", "library_head.tracks_n") : ""}{:else}{entry.size > 0 ? readableSize(entry.size, $currentLocale) : ""}{/if}
                </span>
                <span class="justify-self-end flex items-center justify-center w-8 h-8 rounded-full
                             {entry.is_dir ? 'text-(--rg-mu2) group-hover:text-(--rg-tx2)' : 'opacity-0 group-hover:opacity-100 bg-(--rg-g) text-(--rg-on-g)'}"
                      title={entry.is_dir ? $t("folders_view.open") : $t("folders_view.play")}>
                  <Icon icon={entry.is_dir ? "material-symbols:chevron-right-rounded" : "material-symbols:play-arrow-rounded"} width={entry.is_dir ? 19 : 20} />
                </span>
              </button>
            {/each}
          </div>
        {/if}
      </div>
    </div>
  </div>
{/if}

{#if contextMenu}
  <TrackContextMenu
    track={{ path: contextMenu.entry.path, title: contextMenu.entry.name }}
    x={contextMenu.x}
    y={contextMenu.y}
    {libraryId}
    onclose={() => (contextMenu = null)}
  />
{/if}
