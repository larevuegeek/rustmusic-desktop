<script lang="ts">
  import SearchField from "$lib/components/ui/input/SearchField.svelte";
  import MenuSelect from "$lib/components/ui/menu/MenuSelect.svelte";
  import ViewModeSwitch from "$lib/components/ui/input/ViewModeSwitch.svelte";
  import Icon from "@iconify/svelte";
  import { onDestroy } from "svelte";
  import { page } from "$app/state";
  import { goto } from "$app/navigation";
  import { invoke } from "@tauri-apps/api/core";
  import { t, currentLocale } from "$lib/i18n";
  import { viewMode } from "$lib/stores/ui/viewMode.store";
  import { libraryHeader } from "$lib/stores/library/libraryHeader";
  import { libraryStore } from "$lib/stores/library/library.store";
  import { tagWorkshop } from "$lib/stores/tags/tagWorkshop.store";
  import { toasts } from "$lib/stores/ui/toast.store";
  import { canWriteTags } from "$lib/services/tags/tagEditor.service";
  import { handlePlayTrack } from "$lib/actions/player/PlayerAction";
  import { versFileDAttente } from "$lib/mapper/queue/mapQueueTrack";
  import { tailleLisible } from "$lib/helper/tools/sizeTools";
  import { ilYA } from "$lib/helper/tools/dateTools";
  import { cleTri } from "$lib/helper/library/cleTri";
  import LibraryImportingLoader from "$lib/components/library/common/loader/LibraryImportingLoader.svelte";
  import TrackContextMenu from "$lib/components/ui/contextmenu/TrackContextMenu.svelte";
  import type { LibraryDir } from "$lib/types/db/library/LibraryDir";
  import SelectionToggle from "$lib/components/ui/selection/SelectionToggle.svelte";
  import SelectionCheck from "$lib/components/ui/selection/SelectionCheck.svelte";
  import { selectionStore } from "$lib/stores/ui/selection.store";
  import { cleGroupe, toggleGroupSelection } from "$lib/helper/tools/selectionGroups";

  type DirEntry = {
    name: string; path: string; is_dir: boolean; size: number; extension: string | null;
    title: string | null; artist: string | null; duration: number | null; thumbnail_path: string | null;
    track_id: string | null;
  };

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
  const idFichier = (e: DirEntry) => e.track_id ?? e.path;
  const commePiste = (e: DirEntry) => ({ id: idFichier(e), path: e.path, title: e.title ?? e.name, artist: e.artist, duration: e.duration, thumbnail_path: e.thumbnail_path });
  const cocheEntree = (e: DirEntry) => (e.is_dir ? selection.groupes.has(cleGroupe("path", libraryId, e.path)) : selection.ids.has(idFichier(e)));
  const cocheRacine = (d: LibraryDir) => selection.groupes.has(cleGroupe("folder", libraryId, d.id));

  // Maj + clic : la plage suit l'ordre affiché des fichiers.
  $effect(() => {
    selectionStore.setOrder(visibles.filter((e) => !e.is_dir).map((e) => ({ id: idFichier(e), track: commePiste(e) })));
  });

  function ouvrirEntree(entry: DirEntry, e: MouseEvent) {
    if (!selection.active) return entry.is_dir ? navigateTo(entry.path, entry.name) : lireFichier(entry);
    if (entry.is_dir) toggleGroupSelection("path", libraryId, entry.path);
    else if (e.shiftKey) selectionStore.selectRange(idFichier(entry));
    else selectionStore.toggle(idFichier(entry), commePiste(entry));
  }
  function ouvrirRacine(dir: LibraryDir) {
    if (selection.active) toggleGroupSelection("folder", libraryId, dir.id);
    else navigateTo(dir.path, dir.name);
  }
  const dirCount = $derived(entries.length - fichiers.length);

  function lireFichier(entry: DirEntry) {
    handlePlayTrack(entry.path, versFileDAttente(fichiers));
  }
  function lireDossier() {
    if (fichiers.length) handlePlayTrack(fichiers[0].path, versFileDAttente(fichiers));
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
      name: (a, b) => comparer.compare(cleTri(a.name), cleTri(b.name)),
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

  const nombre = (n: number) => n.toLocaleString($currentLocale);
  const nb = (n: number, un: string, plusieurs: string) => `${nombre(n)} ${$t(n === 1 ? un : plusieurs)}`;
  const ETATS: Record<string, { cle: string; classes: string }> = {
    scanning: { cle: "library_head.status_scanning", classes: "bg-(--rg-ambg) text-(--rg-am)" },
    error: { cle: "library_head.status_error", classes: "bg-red-500/10 text-red-600 dark:text-red-400" },
  };
  const etatDe = (d: LibraryDir) => ETATS[d.scan_status] ?? { cle: "library_head.status_ok", classes: "bg-(--rg-g)/10 text-(--rg-gtx)" };

  const bouton = "h-8.5 px-3 flex items-center gap-1.5 rounded-[10px] border text-[13px] font-semibold whitespace-nowrap cursor-pointer transition-colors bg-(--rg-carte) border-(--rg-bd) text-(--rg-tx2) hover:border-(--rg-bd2) hover:text-(--rg-tx) disabled:opacity-40";
  const tuileDossier = "shrink-0 rounded-xl flex items-center justify-center border";
</script>

<!-- Icône d'une entrée : dossier en ambre, fichier audio en vert. -->
{#snippet icone(estDossier: boolean, taille: string, largeur: number, coche: boolean | null = null)}
  <span class="relative {tuileDossier} {taille} {estDossier ? 'bg-(--rg-ambg) border-(--rg-ambd) text-(--rg-am)' : 'bg-(--rg-gbg) border-(--rg-gbd) text-(--rg-g)'}">
    <Icon icon={estDossier ? "material-symbols:folder-outline-rounded" : "material-symbols:music-note-rounded"} width={largeur} />
    {#if coche !== null}<SelectionCheck {coche} class="absolute -top-1.5 -left-1.5" />{/if}
  </span>
{/snippet}

{#if $libraryStore.isImporting}
  <LibraryImportingLoader />
{:else}
  <div class="flex flex-col h-full">
    <!-- ─── Outils : recherche, actions du dossier, tri, vue ─── -->
    <div class="@container shrink-0 pl-4 md:pl-8 pr-4 md:pr-14 pt-3 pb-2">
      <div class="flex flex-wrap items-center gap-2.5">
        <SearchField bind:value={recherche} class="w-65 @max-4xl:flex-1 @max-4xl:w-auto min-w-36" clearLabel={$t("folders_view.clear")}
                     placeholder={isRoot ? $t("folders_view.filter_dirs").replace("{n}", nombre(rootDirs.length)) : $t("folders_view.filter_entries")} />

        {#if !isRoot && fichiers.length > 0}
          <button type="button" class={bouton} onclick={lireDossier}>
            <Icon icon="material-symbols:play-arrow-rounded" width="18" class="text-(--rg-g)" /><span class="@max-2xl:hidden">{$t("folders_view.play_folder")}</span>
          </button>
          <button type="button" class={bouton} onclick={openWorkshop} disabled={openingWorkshop} title={$t("workshop.fix_tags")}>
            <Icon icon={openingWorkshop ? "material-symbols:progress-activity" : "material-symbols:table-edit-outline-rounded"} width="18" class={openingWorkshop ? "animate-spin" : ""} />
            <span class="@max-2xl:hidden">{$t("workshop.fix_tags")}</span>
          </button>
        {/if}

        <span class="flex-1"></span>

        <MenuSelect value={triEffectif} options={trisDispo.map((x) => ({ value: x.cle, label: $t(x.libelle), icon: x.icone }))}
                    onchange={(v) => (tri = v as typeof tri)} labelClass="@max-xl:hidden" menuClass="w-55" />

        <ViewModeSwitch />
        <SelectionToggle />
      </div>

      <!-- Fil d'Ariane : racine › dossiers ouverts ; à droite, le contenu du niveau. -->
      {#if !isRoot}
        <div class="mt-3 flex items-center gap-1 min-w-0 text-[13px]">
          <button type="button" class="w-7.5 h-7.5 shrink-0 mr-1 flex items-center justify-center rounded-lg cursor-pointer text-(--rg-mu) hover:bg-(--rg-carte) hover:text-(--rg-tx)"
                  title={$t("common.parent_folder")} aria-label={$t("common.parent_folder")} onclick={goToParent}>
            <Icon icon="material-symbols:arrow-upward-rounded" width="18" />
          </button>
          <button type="button" class="shrink-0 flex items-center gap-1.5 px-1.5 py-1 rounded-md cursor-pointer text-(--rg-mu) hover:text-(--rg-tx) hover:bg-(--rg-carte)" onclick={goToRoot}>
            <Icon icon="material-symbols:hard-drive-outline-rounded" width="16" />{$t("folders_view.root")}
          </button>
          {#each breadcrumb as crumb, i (crumb.path)}
            <Icon icon="material-symbols:chevron-right-rounded" width="16" class="shrink-0 text-(--rg-mu2)" />
            {#if i < breadcrumb.length - 1}
              <button type="button" class="min-w-0 max-w-40 truncate px-1.5 py-1 rounded-md cursor-pointer text-(--rg-mu) hover:text-(--rg-tx) hover:bg-(--rg-carte)"
                      title={crumb.path} onclick={() => navigateTo(crumb.path, crumb.name)}>{crumb.name}</button>
            {:else}
              <span class="min-w-0 truncate px-1.5 font-semibold text-(--rg-tx)" title={crumb.path}>{crumb.name}</span>
            {/if}
          {/each}
          <span class="ml-auto pl-3 shrink-0 whitespace-nowrap text-xs text-(--rg-mu)">
            {#if dirCount > 0}{nb(dirCount, "library_head.folders_one", "library_head.folders_n")}{/if}{#if dirCount > 0 && fichiers.length > 0}{" · "}{/if}{#if fichiers.length > 0}{nb(fichiers.length, "library_head.files_one", "library_head.files_n")}{/if}
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
                {@const etat = etatDe(dir)}
                <button type="button"
                        class="group relative grid grid-cols-[48px_minmax(0,1fr)_100px_90px_150px_40px] max-[900px]:grid-cols-[48px_minmax(0,1fr)_90px_40px] items-center gap-4 py-2 pl-2 pr-3 rounded-xl text-left cursor-pointer transition-colors
                               {cocheRacine(dir) ? 'bg-(--rg-creux-on)' : 'hover:bg-(--rg-carte)'}
                               before:absolute before:left-18 before:right-3 before:top-0 before:h-px [button+&]:before:bg-(--rg-line) hover:before:opacity-0 [&:hover+button]:before:opacity-0"
                        onclick={() => ouvrirRacine(dir)}>
                  {@render icone(true, "w-12 h-12", 22, selection.active ? cocheRacine(dir) : null)}
                  <span class="min-w-0">
                    <span class="block truncate text-base font-semibold text-(--rg-tx)">{dir.name}</span>
                    <span class="block mt-0.5 truncate font-mono text-xs text-(--rg-mu)" title={dir.path}>{dir.path}</span>
                  </span>
                  <span class="text-right text-[13px] tabular-nums text-(--rg-mu)"><b class="font-medium text-(--rg-tx2)">{nombre(dir.total_files)}</b> {$t(dir.total_files === 1 ? "library_head.files_one" : "library_head.files_n")}</span>
                  <span class="text-right text-[13px] tabular-nums text-(--rg-mu) max-[900px]:hidden">{tailleLisible(dir.total_size, $currentLocale)}</span>
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
                {@const etat = etatDe(dir)}
                <button type="button"
                        class="group flex flex-col gap-3.5 p-4 rounded-2xl border text-left cursor-pointer transition-colors
                               {cocheRacine(dir) ? 'bg-(--rg-creux-on) border-(--rg-g)' : 'bg-(--rg-carte) border-(--rg-bd) hover:border-(--rg-bd2)'}"
                        onclick={() => ouvrirRacine(dir)}>
                  <span class="flex items-start justify-between gap-3">
                    {@render icone(true, "w-12 h-12", 24, selection.active ? cocheRacine(dir) : null)}
                    <span class="flex items-center gap-1.5 px-2 py-0.75 rounded-[10px] text-[11px] font-semibold whitespace-nowrap {etat.classes}">
                      <span class="w-1.5 h-1.5 rounded-full bg-current"></span>{$t(etat.cle)}
                    </span>
                  </span>
                  <span class="min-w-0">
                    <span class="block truncate text-[15px] font-bold text-(--rg-tx)" title={dir.name}>{dir.name}</span>
                    <span class="block mt-0.5 truncate font-mono text-[11px] text-(--rg-mu)" title={dir.path}>{dir.path}</span>
                  </span>
                  <span class="flex items-center gap-1.5 text-[13px] text-(--rg-mu) whitespace-nowrap">
                    <span><b class="font-semibold text-(--rg-tx2)">{nombre(dir.total_files)}</b> {$t(dir.total_files === 1 ? "library_head.files_one" : "library_head.files_n")}</span>
                    {#if dir.total_size}<span>· {tailleLisible(dir.total_size, $currentLocale)}</span>{/if}
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
          <!-- Entrées en tuiles : un dossier parmi trente se retrouve à l'œil. -->
          <div class="grid grid-cols-[repeat(auto-fill,minmax(150px,1fr))] gap-3 pt-3">
            {#each visibles as entry (entry.path)}
              <button type="button"
                      class="group flex flex-col items-center gap-2.5 p-4 rounded-2xl border text-center cursor-pointer transition-colors
                             {cocheEntree(entry) ? 'bg-(--rg-creux-on) border-(--rg-g)' : 'bg-(--rg-carte) border-(--rg-bd) hover:border-(--rg-bd2)'}"
                      onclick={(e) => ouvrirEntree(entry, e)}
                      oncontextmenu={(e) => { if (!entry.is_dir) { e.preventDefault(); contextMenu = { x: e.clientX, y: e.clientY, entry }; } }}>
                {@render icone(entry.is_dir, "w-14 h-14", 26, selection.active ? cocheEntree(entry) : null)}
                <span class="w-full min-w-0">
                  <span class="block truncate text-sm font-semibold text-(--rg-tx)" title={entry.name}>{entry.name}</span>
                  <span class="block mt-0.5 text-[11px] text-(--rg-mu)">
                    {#if entry.is_dir}{$t("library_head.folders_one")}{:else}<span class="uppercase tracking-wide">{entry.extension ?? $t("folders_view.audio")}</span>{#if entry.size > 0}{" · "}{tailleLisible(entry.size, $currentLocale)}{/if}{/if}
                  </span>
                </span>
              </button>
            {/each}
          </div>

        {:else}
          <!-- Entrées en liste : nom, format, poids ; lecture au survol des fichiers. -->
          <div class="flex flex-col pt-2">
            {#each visibles as entry (entry.path)}
              <button type="button"
                      class="group relative grid grid-cols-[40px_minmax(0,1fr)_70px_90px_40px] items-center gap-4 py-1.5 pl-2 pr-3 rounded-xl text-left cursor-pointer transition-colors
                             {cocheEntree(entry) ? 'bg-(--rg-creux-on)' : 'hover:bg-(--rg-carte)'}
                             before:absolute before:left-16 before:right-3 before:top-0 before:h-px [button+&]:before:bg-(--rg-line) hover:before:opacity-0 [&:hover+button]:before:opacity-0"
                      onclick={(e) => ouvrirEntree(entry, e)}
                      oncontextmenu={(e) => { if (!entry.is_dir) { e.preventDefault(); contextMenu = { x: e.clientX, y: e.clientY, entry }; } }}>
                {@render icone(entry.is_dir, "w-10 h-10", 19, selection.active ? cocheEntree(entry) : null)}
                <span class="min-w-0 truncate text-sm {entry.is_dir ? 'font-semibold text-(--rg-tx)' : 'text-(--rg-tx2)'}" title={entry.name}>{entry.name}</span>
                <span class="justify-self-end">
                  {#if entry.extension}
                    <span class="px-1.5 py-0.5 rounded-[5px] border text-[10px] font-bold uppercase tracking-[0.06em] bg-(--rg-s2) border-(--rg-bd) text-(--rg-mu)">{entry.extension}</span>
                  {/if}
                </span>
                <span class="text-right text-[13px] tabular-nums text-(--rg-mu)">{!entry.is_dir && entry.size > 0 ? tailleLisible(entry.size, $currentLocale) : ""}</span>
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
