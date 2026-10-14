<script lang="ts">
  // Atelier de tags : un tableau pour repérer et corriger en place, un panneau pour
  // poser une valeur sur toute la sélection. Une page (et non une popin) : on trie,
  // on écoute, on revient. Rien n'est écrit avant « Écrire ».
  import Icon from "@iconify/svelte";
  import { tick } from "svelte";
  import { fly, slide } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import { page } from "$app/state";
  import { goto, beforeNavigate } from "$app/navigation";
  import { t } from "#lib/i18n";
  import { messageErreur } from "#lib/helper/tools/errorTools";
  import TagTable, { type TableColumn } from "#lib/components/library/tags/TagTable.svelte";
  import TagInspector from "#lib/components/library/tags/TagInspector.svelte";
  import TagAuditView from "#lib/components/library/tags/TagAuditView.svelte";
  import Menu from "#lib/components/ui/menu/Menu.svelte";
  import MenuAction from "#lib/components/ui/menu/MenuAction.svelte";
  import TagSourcePopin from "#lib/components/library/common/popin/TagSourcePopin.svelte";
  import RenamePopin from "#lib/components/library/common/popin/RenamePopin.svelte";
  import BatchHistoryPopin from "#lib/components/library/common/popin/BatchHistoryPopin.svelte";
  import CleanTagsPopin from "#lib/components/library/common/popin/CleanTagsPopin.svelte";
  import { popinStore } from "#lib/stores/ui/popin.store";
  import { tagWorkshop, fileIsDirty, isIncomplete, valueOf, WORKSHOP_FIELDS, type WorkshopField, type WorkshopFile } from "#lib/stores/tags/tagWorkshop.store";
  import { batchStore } from "#lib/stores/ui/batch.store";
  import { writeTagsEach } from "#lib/services/batch/batch.service";
  import { loadTracksByAlbum } from "#lib/services/library/library.service";
  import { libraryContentStore } from "#lib/stores/library/libraryContent.store";
  import type { AuditGroup } from "#lib/services/tags/audit.service";
  import type { TrackListView } from "#lib/types/ui/library/track/TrackListView";
  import { prepareImage } from "#lib/services/tags/tagEditor.service";
  import { open } from "@tauri-apps/plugin-dialog";
  import { lireLocal, ecrireLocal } from "#lib/helper/tools/stockage";

  let workshop = $derived($tagWorkshop);
  let libraryId = $derived(Number(page.params.library_id));

  // ─── Préférences d'affichage (gardées d'une visite à l'autre) ───
  const ALL_COLUMNS: TableColumn[] = [...WORKSHOP_FIELDS, "file"];
  const DEFAULT: TableColumn[] = ["track_number", "title", "artist", "album", "year", "genre"];
  function lire<T>(cle: string, defaut: T): T {
    try {
      return JSON.parse(lireLocal(cle, JSON.stringify(defaut))) as T;
    } catch {
      return defaut;
    }
  }
  const garder = (cle: string, v: unknown) => ecrireLocal(cle, JSON.stringify(v));
  let shown: TableColumn[] = $state(lire("atelier:colonnes:v2", DEFAULT).filter((c) => ALL_COLUMNS.includes(c)));
  let panel = $state(lire("atelier:panneau", true));
  $effect(() => garder("atelier:colonnes:v2", shown));
  $effect(() => garder("atelier:panneau", panel));
  let columns = $derived(ALL_COLUMNS.filter((c) => c === "title" || shown.includes(c)));

  let onlyIssues = $state(false);
  let editing: { path: string; field: WorkshopField } | null = $state(null);
  let launching = $state(false);
  let launchError: string | null = $state(null);
  let coverBusy = $state(false);
  let columnsOpen = $state(false);

  // ─── Chargement ───
  /** Album déjà chargé : en revenant d'un autre album, le magasin contient encore le précédent. */
  let loadedAlbum: string | null = $state(null);
  $effect(() => {
    const albumId = page.url.searchParams.get("album");
    if (!albumId || albumId === loadedAlbum) return;
    loadedAlbum = albumId;
    loadAlbum(albumId);
  });

  async function loadAlbum(albumId: string) {
    const tracks: TrackListView[] = await loadTracksByAlbum(libraryId, albumId);
    const paths = tracks.map((track) => track.path).filter((path): path is string => !!path);
    await tagWorkshop.load(paths, tracks[0]?.album ?? $t("workshop.title"));
  }

  /** Une catégorie de l'inventaire versée dans l'atelier (pas d'origine où revenir). */
  async function loadFromAudit(group: AuditGroup, label: string) {
    loadedAlbum = null;
    await tagWorkshop.load(group.paths, label);
  }

  // ─── Sortie protégée ───
  let confirmingLeave = $state(false);
  beforeNavigate((navigation) => {
    if (pending.length === 0 || confirmingLeave) return;
    navigation.cancel();
    confirmingLeave = true;
  });

  let backHref = $derived.by(() => {
    const albumId = page.url.searchParams.get("album");
    return albumId ? `/library/${libraryId}/albums/${albumId}` : workshop.origin;
  });

  function leave() {
    if (pending.length > 0 && !confirmingLeave) {
      confirmingLeave = true;
      return;
    }
    tagWorkshop.clear();
    loadedAlbum = null;
    if (backHref) goto(backHref);
    else history.back();
  }

  // ─── État ───
  let readable = $derived(workshop.files.filter((f) => f.readable));
  let pending = $derived(readable.filter(fileIsDirty));
  let changes = $derived(pending.reduce((n, f) => n + Object.keys(f.edits).length + (f.pendingCover ? 1 : 0), 0));
  let selectedCount = $derived(readable.filter((f) => workshop.selected.has(f.path)).length);

  const FEAT = /\s(feat\.?|ft\.?|featuring)\s/i;
  const empty = (f: WorkshopFile, field: WorkshopField) => valueOf(f, field).trim() === "";
  const hasIssue = (f: WorkshopFile) =>
    isIncomplete(f) || empty(f, "genre") || empty(f, "album_artist") || FEAT.test(valueOf(f, "artist"));
  let rows = $derived(onlyIssues ? workshop.files.filter((f) => !f.readable || hasIssue(f)) : workshop.files);
  let issueCount = $derived(readable.filter(hasIssue).length);

  let issues = $derived([
    { key: "essential", n: readable.filter(isIncomplete).length, action: "online" },
    { key: "genre", n: readable.filter((f) => empty(f, "genre")).length, action: "online" },
    { key: "album_artist", n: readable.filter((f) => empty(f, "album_artist")).length, action: "fill_album_artist" },
    { key: "featuring", n: readable.filter((f) => FEAT.test(valueOf(f, "artist"))).length, action: "clean" },
  ].filter((i) => i.n > 0));

  // En-tête : ce qu'on corrige, tel qu'on le verra après écriture.
  const shared = (field: WorkshopField) => {
    const values = new Set(readable.map((f) => valueOf(f, field)).filter(Boolean));
    return values.size === 1 ? [...values][0] : null;
  };
  let headCover = $derived(
    workshop.files.find((f) => f.pendingCover)?.pendingCover?.src ?? workshop.files.find((f) => f.cover)?.cover ?? null,
  );
  let formats = $derived([...new Set(workshop.files.map((f) => f.path.split(".").pop()?.toUpperCase() ?? ""))].filter(Boolean).join(", "));

  // ─── Avis (avec « Annuler ») ───
  let notice: { text: string; undoable: boolean } | null = $state(null);
  let noticeTimer: ReturnType<typeof setTimeout> | undefined;
  function say(text: string, undoable = false) {
    notice = { text, undoable };
    clearTimeout(noticeTimer);
    noticeTimer = setTimeout(() => (notice = null), undoable ? 4500 : 2600);
  }
  function undo() {
    if (workshop.undoCount === 0) return;
    tagWorkshop.undo();
    editing = null;
    say($t("workshop.undone"));
  }

  const plural = (n: number, one: string, many: string) => `${n} ${n > 1 ? many : one}`;
  const tracks = (n: number) => plural(n, $t("workshop.track_one"), $t("workshop.track_many"));
  const files = (n: number) => plural(n, $t("workshop.file_one"), $t("workshop.file_many"));

  // ─── Actions ───
  async function resync(moved: [string, string][]) {
    if (moved.length === 0) return;
    const replacement = new Map(moved);
    const paths = workshop.files.map((f) => replacement.get(f.path) ?? f.path);
    await libraryContentStore.refresh();
    await tagWorkshop.load(paths, workshop.source);
  }

  function number() {
    const before = workshop.revision;
    tagWorkshop.numberSelected();
    if ($tagWorkshop.revision === before) say($t("workshop.already_numbered"));
    else say($t("workshop.numbered").replace("{n}", String(selectedCount)), true);
  }

  function openClean() {
    popinStore.open($t("clean.title"), CleanTagsPopin, { onapplied: (n: number) => n > 0 && say($t("workshop.cleaned").replace("{n}", String(n)), true) }, { size: "xl", flush: true, icon: "lucide:sparkles" });
  }

  function openSource() {
    popinStore.open($t("workshop.complete_online"), TagSourcePopin, {
      onapplied: (values: number, n: number) => values > 0 && say($t("workshop.completed").replace("{v}", String(values)).replace("{n}", files(n)), true),
    }, { size: "xxl", bare: true });
  }

  function openRename() {
    popinStore.open($t("rename.title"), RenamePopin, { libraryId, onapplied: resync }, { size: "xl", flush: true, icon: "lucide:file-pen-line" });
  }

  function openHistory() {
    popinStore.open($t("history.title"), BatchHistoryPopin, { onundone: resync }, { size: "lg", flush: true, icon: "lucide:history" });
  }

  /** Artiste de l'album vide : l'artiste le plus fréquent, sans ses invités. */
  function fillAlbumArtist() {
    const counts = new Map<string, number>();
    for (const f of readable) {
      const main = valueOf(f, "artist").replace(/\s+(feat\.?|ft\.?|featuring)\s.+$/i, "").trim();
      if (main) counts.set(main, (counts.get(main) ?? 0) + 1);
    }
    const best = [...counts.entries()].sort((a, b) => b[1] - a[1])[0]?.[0];
    if (!best) return;
    const targets = readable.filter((f) => empty(f, "album_artist"));
    tagWorkshop.group(() => targets.forEach((f) => tagWorkshop.set(f.path, "album_artist", best)));
    say($t("workshop.album_artist_filled").replace("{v}", best).replace("{n}", tracks(targets.length)), true);
  }

  function runIssue(action: string) {
    // Les points comptent tous les fichiers : sans sélection, la correction les vise tous.
    if (selectedCount === 0) tagWorkshop.selectAll(readable.map((f) => f.path));
    if (action === "online") openSource();
    else if (action === "clean") openClean();
    else fillAlbumArtist();
  }

  /** Pochette sur la sélection : seule la pochette avant change, les autres images restent. */
  async function pickCover() {
    const chosen = await open({ multiple: false, filters: [{ name: "Images", extensions: ["jpg", "jpeg", "png", "webp", "gif", "bmp"] }] });
    if (typeof chosen !== "string") return;
    coverBusy = true;
    launchError = null;
    try {
      const prepared = await prepareImage(chosen);
      tagWorkshop.setCoverOnSelected({ path: chosen, src: prepared.src });
      say($t("workshop.cover_set").replace("{n}", tracks(selectedCount)), true);
    } catch (e) {
      launchError = messageErreur(e);
    } finally {
      coverBusy = false;
    }
  }

  /** Chaque fichier porte ses modifications ; le suivi part dans le panneau des lots. */
  async function write() {
    if (pending.length === 0 || launching) return;
    if (batchStore.isRunning()) {
      launchError = $t("tags.batch_already_running");
      return;
    }
    launching = true;
    launchError = null;
    try {
      const items = tagWorkshop.pendingWrites();
      const jobId = await writeTagsEach(items);
      batchStore.begin(jobId, $t("batch.writing_tags"));
      tagWorkshop.markWritten(items.map((item) => item.path));
    } catch (e) {
      launchError = messageErreur(e);
    } finally {
      launching = false;
    }
  }

  function revertAll() {
    tagWorkshop.revertAll();
    say($t("workshop.all_reverted"), true);
  }

  // ─── Raccourcis ───
  function keys(e: KeyboardEvent) {
    if ($popinStore.isOpen || workshop.files.length === 0) return;
    const mod = e.ctrlKey || e.metaKey;
    const key = e.key.toLowerCase();
    if (mod && key === "s") {
      e.preventDefault();
      // La saisie en cours ne compte qu'à la sortie du champ : on l'y force avant d'écrire.
      (document.activeElement as HTMLElement | null)?.blur();
      tick().then(write);
      return;
    }
    if ((e.target as HTMLElement).closest("input, textarea, select")) return;
    if (mod && key === "z") {
      e.preventDefault();
      undo();
    } else if (mod && key === "a") {
      e.preventDefault();
      tagWorkshop.selectAll(rows.filter((f) => f.readable).map((f) => f.path));
    } else if (e.key === "F2") {
      const first = rows.find((f) => f.readable && workshop.selected.has(f.path));
      if (first) editing = { path: first.path, field: "title" };
    }
  }

  // Menus déroulants : un seul ouvert, fermé au clic ailleurs.
  let issuesOpen = $state(false);
  let tagsOpen = $state(false);
  let filesOpen = $state(false);
  // Largeur de la page, pas de la fenêtre : la barre latérale en prend une part.
  let pageWidth = $state(1400);
  let wide = $derived(pageWidth >= 1200);
  /** Page étroite : le panneau devient un tiroir, fermé tant qu'on ne l'ouvre pas. */
  let drawer = $state(false);
  let showPanel = $derived(wide ? panel : drawer);
  function togglePanel() {
    if (wide) panel = !panel;
    else drawer = !drawer;
  }

  $effect(() => {
    if (!columnsOpen && !issuesOpen) return;
    const close = () => (columnsOpen = issuesOpen = false);
    const timer = setTimeout(() => document.addEventListener("click", close), 0);
    return () => {
      clearTimeout(timer);
      document.removeEventListener("click", close);
    };
  });

  const outil = "h-9 px-2.5 rounded-[9px] flex items-center gap-2 text-[13.5px] font-semibold whitespace-nowrap cursor-pointer transition-colors text-(--rg-tx2) hover:bg-(--rg-carte) hover:text-(--rg-tx) disabled:opacity-40 disabled:cursor-default disabled:hover:bg-transparent";
  const carre = "relative w-10 h-10 shrink-0 rounded-[10px] flex items-center justify-center cursor-pointer transition-colors border border-(--rg-bd) bg-(--rg-carte) text-(--rg-tx2) hover:text-(--rg-tx) hover:border-(--rg-bd2) disabled:opacity-40 disabled:cursor-default";
</script>

<svelte:window onkeydown={keys} />

<!-- `overflow-clip` : un focus ou un défilement ne peut plus décaler la page entière. -->
<div class="atelier biblio-maquette @container flex flex-col h-full min-h-0 min-w-0 overflow-clip bg-(--rg-champ) text-(--rg-tx)"
     bind:clientWidth={pageWidth}>
  {#if workshop.loading}
    <div class="flex-1 flex items-center justify-center">
      <Icon icon="material-symbols:progress-activity" width="26" class="animate-spin text-(--rg-g)" />
    </div>
  {:else if workshop.files.length === 0}
    <!-- Sans fichiers : l'inventaire dit ce qui cloche, un clic verse les morceaux ici. -->
    <TagAuditView {libraryId} onpick={loadFromAudit} />
  {:else}
    <!-- ─────────── En-tête ─────────── -->
    <header class="shrink-0 flex items-center gap-3 px-5 py-3">
      <button type="button" onclick={leave} title={$t("workshop.back")} aria-label={$t("workshop.back")}
              class="w-9 h-9 shrink-0 rounded-[10px] flex items-center justify-center cursor-pointer transition-colors text-(--rg-tx2) hover:bg-(--rg-carte) hover:text-(--rg-tx)">
        <Icon icon="material-symbols:arrow-back-rounded" width="22" />
      </button>
      <span class="relative w-11 h-11 shrink-0 rounded-[9px] overflow-hidden bg-(--rg-s2) shadow-[0_4px_12px_rgba(0,0,0,0.35)]">
        {#if headCover}
          <img src={headCover} alt="" class="w-full h-full object-cover" />
        {:else}
          <Icon icon="material-symbols:hide-image-outline-rounded" width="22" class="absolute inset-0 m-auto text-(--rg-mu2)" />
        {/if}
      </span>
      <div class="flex-1 min-w-0">
        <h1 class="text-[18px] leading-tight font-extrabold tracking-[-0.01em] truncate" title={$t("workshop.title")}>{workshop.source}</h1>
        <p class="mt-0.5 flex gap-x-2 text-[12.5px] text-(--rg-mu) truncate">
          {#if shared("album_artist") ?? shared("artist")}<b class="font-semibold text-(--rg-tx2) truncate">{shared("album_artist") ?? shared("artist")}</b><span>·</span>{/if}
          {#if shared("year")}<span>{shared("year")}</span><span>·</span>{/if}
          <span class="whitespace-nowrap">{files(workshop.files.length)} {formats}</span>
        </p>
      </div>
      <div class="shrink-0 flex items-center gap-2">
        {#if changes > 0}
          <div class="h-10 flex items-center gap-2 pl-3 pr-1 rounded-[10px] border border-(--rg-gbd) bg-(--rg-gbg) text-[13px] whitespace-nowrap text-(--rg-tx2)">
            <span class="w-2 h-2 rounded-full bg-(--rg-g) shadow-[0_0_0_4px_color-mix(in_oklch,var(--rg-g)_15%,transparent)]"></span>
            <b class="text-(--rg-tx)">{plural(changes, $t("workshop.change_one"), $t("workshop.change_many"))}</b>
            <button type="button" onclick={revertAll}
                    class="h-8 px-2.5 rounded-[7px] text-[13px] font-semibold cursor-pointer transition-colors text-(--rg-tx2) hover:bg-black/10 dark:hover:bg-white/6 hover:text-(--rg-tx)">
              {$t("workshop.revert_all")}
            </button>
          </div>
        {/if}
        <button type="button" class={carre} onclick={undo} disabled={workshop.undoCount === 0}
                title="{$t('workshop.undo')} (Ctrl+Z)" aria-label={$t("workshop.undo")}>
          <Icon icon="material-symbols:undo-rounded" width="20" />
          {#if workshop.undoCount > 0}
            <span class="absolute -top-1.25 -right-1.25 min-w-4.5 h-4.5 px-1.25 rounded-full border border-(--rg-bd2) bg-(--rg-s2) text-[10.5px] font-bold flex items-center justify-center text-(--rg-tx2)">{workshop.undoCount}</span>
          {/if}
        </button>
        <button type="button" onclick={write} disabled={pending.length === 0 || launching}
                title="{$t('workshop.write_hint')} (Ctrl+S)"
                class="h-10 px-4 pl-3 rounded-[10px] flex items-center gap-2 text-[14px] font-extrabold whitespace-nowrap cursor-pointer transition
                       bg-(--rg-g) text-(--rg-on-g) hover:brightness-110
                       disabled:bg-(--rg-carte) disabled:text-(--rg-mu2) disabled:border disabled:border-(--rg-bd) disabled:cursor-default disabled:hover:brightness-100">
          <Icon icon={launching ? "material-symbols:progress-activity" : "material-symbols:check-rounded"} width="20" class={launching ? "animate-spin" : ""} />
          {pending.length > 0 ? `${$t("workshop.write")} ${files(pending.length)}` : $t("workshop.write")}
        </button>
        <button type="button" class="{carre} {showPanel ? 'bg-(--rg-s2)! border-(--rg-bd2)! text-(--rg-tx)!' : ''}"
                onclick={togglePanel} aria-pressed={showPanel} title={$t("workshop.edit_panel")} aria-label={$t("workshop.edit_panel")}>
          <Icon icon="material-symbols:view-sidebar-outline-rounded" width="20" />
        </button>
      </div>
    </header>

    {#if confirmingLeave}
      <div class="mx-5 mb-3 flex items-center gap-3 px-3.5 py-2.5 rounded-xl border border-(--rg-ambd) bg-(--rg-ambg) text-[13.5px]">
        <Icon icon="material-symbols:warning-outline-rounded" width="19" class="shrink-0 text-(--rg-am)" />
        <span class="flex-1 min-w-0 text-(--rg-amtx)">{pending.length} {pending.length > 1 ? $t("workshop.pending_files") : $t("workshop.pending_file")}</span>
        <button type="button" onclick={leave} class="h-8 px-3 rounded-lg font-semibold cursor-pointer text-(--rg-am) hover:bg-black/10 dark:hover:bg-white/6">{$t("tags.discard")}</button>
        <button type="button" onclick={() => (confirmingLeave = false)} class="h-8 px-3 rounded-lg font-bold cursor-pointer bg-(--rg-am) text-black">{$t("tags.keep_editing")}</button>
      </div>
    {/if}

    {#if launchError}
      <p class="mx-5 mb-3 flex items-start gap-2 text-[13px] text-red-500">
        <Icon icon="material-symbols:error-outline-rounded" width="17" class="shrink-0" />{launchError}
      </p>
    {/if}

    <!-- ─────────── Outils : une seule ligne, libellés repliés quand la place manque ─────────── -->
    <div class="shrink-0 flex items-center gap-1 px-5 py-2 border-y border-(--rg-bd) bg-(--rg-carte)/40">
      <span class="mr-1.5 text-[13px] whitespace-nowrap text-(--rg-mu)">
        {#if selectedCount > 0}
          {@html $t("workshop.scope").replace("{n}", `<b class="font-bold text-(--rg-tx)">${selectedCount}</b>`).replace("{total}", String(readable.length))}
        {:else}
          {$t("workshop.no_selection")}
        {/if}
      </span>
      <span class="w-px h-5 mx-1 bg-(--rg-bd2)"></span>
      <!-- Deux menus nommés : ce qui touche aux tags, ce qui touche aux fichiers. -->
      {#snippet menuButton(label: string, icon: string, open: boolean, toggle: () => void)}
        <button type="button" class="{outil} {open ? 'bg-(--rg-carte) text-(--rg-tx)' : ''}" onclick={toggle} aria-expanded={open} aria-haspopup="menu">
          <Icon {icon} width="19" class="text-(--rg-mu)" />{label}
          <Icon icon="material-symbols:expand-more-rounded" width="18" class="-ml-1 text-(--rg-mu) transition-transform {open ? 'rotate-180' : ''}" />
        </button>
      {/snippet}
      <div class="relative">
        {@render menuButton($t("workshop.menu_tags"), "material-symbols:sell-outline-rounded", tagsOpen, () => (tagsOpen = !tagsOpen))}
        <Menu bind:open={tagsOpen} align="left" class="w-84">
          {@const none = selectedCount === 0}
          {@const why = $t("workshop.needs_selection")}
          <MenuAction icon="material-symbols:cloud-download-outline-rounded" title={$t("workshop.complete_online")} description={none ? why : $t("workshop.desc_online")} disabled={none} onclick={() => { tagsOpen = false; openSource(); }} />
          <MenuAction icon="material-symbols:auto-fix-normal-outline-rounded" title={$t("clean.short")} description={none ? why : $t("workshop.desc_clean")} disabled={none} onclick={() => { tagsOpen = false; openClean(); }} />
          <MenuAction icon="material-symbols:format-list-numbered-rounded" title={$t("tags.number_tracks")} description={none ? why : $t("workshop.desc_number")} disabled={none} onclick={() => { tagsOpen = false; number(); }} />
          <MenuAction icon="material-symbols:image-outline-rounded" title={$t("workshop.cover_action")} description={none ? why : $t("workshop.desc_cover")} disabled={none || coverBusy} onclick={() => { tagsOpen = false; pickCover(); }} />
        </Menu>
      </div>
      <div class="relative">
        {@render menuButton($t("workshop.menu_files"), "material-symbols:folder-open-outline-rounded", filesOpen, () => (filesOpen = !filesOpen))}
        <Menu bind:open={filesOpen} align="left" class="w-84">
          <MenuAction icon="material-symbols:drive-file-rename-outline-outline-rounded" title={$t("workshop.rename_files")} description={$t("workshop.desc_rename")} onclick={() => { filesOpen = false; openRename(); }} />
          <MenuAction icon="material-symbols:history-rounded" title={$t("history.title")} description={$t("workshop.desc_history")} onclick={() => { filesOpen = false; openHistory(); }} />
        </Menu>
      </div>
      <span class="flex-1"></span>

      <!-- Points à vérifier : la pastille ouvre leur liste, chacun avec sa correction. -->
      <div class="relative">
        <button type="button" onclick={() => (issuesOpen = !issuesOpen)} aria-expanded={issuesOpen}
                class="h-8.5 pl-2.5 pr-3 rounded-full border flex items-center gap-1.5 text-[13px] font-semibold whitespace-nowrap cursor-pointer transition-colors
                       {issues.length === 0
                         ? 'border-(--rg-bd) text-(--rg-gtx) hover:border-(--rg-bd2)'
                         : onlyIssues || issuesOpen
                           ? 'bg-(--rg-ambg) border-(--rg-ambd) text-(--rg-am)'
                           : 'border-(--rg-ambd) text-(--rg-am) hover:bg-(--rg-ambg)'}">
          <Icon icon={issues.length === 0 ? "material-symbols:done-all-rounded" : "material-symbols:warning-outline-rounded"} width="17" />
          {issues.length === 0 ? $t("workshop.consistent") : plural(issues.length, $t("workshop.issue_one"), $t("workshop.issue_many"))}
          {#if onlyIssues}<Icon icon="material-symbols:filter-alt" width="15" />{/if}
        </button>
        {#if issuesOpen}
          <div role="presentation" onclick={(e) => e.stopPropagation()}
               class="atelier-menu absolute right-0 top-full mt-1.5 z-30 w-90 p-2 rounded-xl">
            {#if issues.length === 0}
              <p class="px-2 py-2.5 text-[13px] text-(--rg-mu)">{changes > 0 ? $t("workshop.remember_write") : $t("workshop.no_issue")}</p>
            {:else}
              {#each issues as issue (issue.key)}
                <div class="flex items-center gap-2 px-2 py-2 rounded-lg text-[13.5px] text-(--rg-tx2)">
                  <span class="flex-1 min-w-0">{$t(`workshop.issue_${issue.key}`)}</span>
                  <b class="font-bold tabular-nums text-(--rg-tx)">{issue.n}</b>
                  <button type="button" onclick={() => { issuesOpen = false; runIssue(issue.action); }}
                          class="h-7 px-2.5 rounded-md flex items-center gap-1 text-[12.5px] font-bold cursor-pointer text-(--rg-gtx) hover:bg-(--rg-gbg)">
                    <Icon icon={issue.action === "online" ? "material-symbols:cloud-download-outline-rounded" : "material-symbols:auto-fix-normal-outline-rounded"} width="15" />
                    {$t(`workshop.fix_${issue.action}`)}
                  </button>
                </div>
              {/each}
              <hr class="my-1.5 border-(--rg-bd)" />
              <button type="button" onclick={() => (onlyIssues = !onlyIssues)}
                      class="w-full flex items-center gap-2.5 px-2 py-2 rounded-lg text-[13.5px] text-left cursor-pointer hover:bg-(--rg-s2)">
                <span class="w-4.5 h-4.5 rounded-[5px] border-2 flex items-center justify-center {onlyIssues ? 'bg-(--rg-g) border-(--rg-g) text-(--rg-on-g)' : 'border-(--rg-bd2) text-transparent'}">
                  <Icon icon="material-symbols:check-rounded" width="14" />
                </span>
                {$t("workshop.show_only_issues").replace("{n}", tracks(issueCount))}
              </button>
            {/if}
          </div>
        {/if}
      </div>

      <div class="relative">
        <button type="button" class="{outil} {columnsOpen ? 'bg-(--rg-carte) text-(--rg-tx)' : ''}" onclick={() => (columnsOpen = !columnsOpen)}
                aria-expanded={columnsOpen} title={$t("workshop.columns")} aria-label={$t("workshop.columns")}>
          <Icon icon="material-symbols:view-column-outline-rounded" width="19" class="text-(--rg-mu)" /><span class="@max-[1380px]:hidden">{$t("workshop.columns")}</span>
        </button>
        {#if columnsOpen}
          <div role="presentation" class="atelier-menu absolute right-0 top-full mt-1.5 z-30 w-58 p-1.5 rounded-xl" onclick={(e) => e.stopPropagation()}>
            <p class="px-2.5 pt-2 pb-1.5 text-[11px] font-bold tracking-widest uppercase text-(--rg-mu)">{$t("workshop.shown_columns")}</p>
            {#each ALL_COLUMNS as column (column)}
              {@const on = column === "title" || shown.includes(column)}
              <button type="button" disabled={column === "title"}
                      onclick={() => (shown = on ? shown.filter((c) => c !== column) : [...shown, column])}
                      class="w-full flex items-center gap-2.5 px-2.5 py-2 rounded-lg text-[13px] text-left cursor-pointer transition-colors hover:bg-(--rg-s2) disabled:opacity-40 disabled:cursor-default">
                <span class="w-4.5 h-4.5 rounded-[5px] border-2 flex items-center justify-center {on ? 'bg-(--rg-g) border-(--rg-g) text-(--rg-on-g)' : 'border-(--rg-bd2) text-transparent'}">
                  <Icon icon="material-symbols:check-rounded" width="14" />
                </span>
                {column === "file" ? $t("workshop.file") : $t(`tags.${column}`)}
              </button>
            {/each}
          </div>
        {/if}
      </div>
    </div>

    <!-- ─────────── Tableau et panneau ─────────── -->
    <div class="flex-1 min-h-0 min-w-0 flex relative">
      <TagTable {columns} {rows} bind:editing
                onhide={(c) => (shown = shown.filter((x) => x !== c))}
                onnotice={say} />
      <!-- Large : colonne qui s'ouvre en glissant. Étroite : tiroir par-dessus le tableau. -->
      {#if showPanel && wide}
        <div class="shrink-0 flex overflow-hidden" transition:slide={{ axis: "x", duration: 220, easing: cubicOut }}>
          <TagInspector source={workshop.source} {coverBusy} onpickcover={pickCover} onnotice={say} />
        </div>
      {:else if showPanel}
        <div class="absolute inset-y-0 right-0 z-20 flex shadow-[-20px_0_50px_rgba(0,0,0,0.45)]"
             transition:fly={{ x: 320, duration: 220, easing: cubicOut, opacity: 1 }}>
          <TagInspector source={workshop.source} {coverBusy} onpickcover={pickCover} onnotice={say} />
        </div>
      {/if}
    </div>
  {/if}

  {#if notice}
    <div class="atelier-menu fixed left-1/2 bottom-30 -translate-x-1/2 z-50 flex items-center gap-2.5 py-2.75 pl-3 pr-4 rounded-xl text-[13.5px] font-semibold whitespace-nowrap" role="status">
      <Icon icon="material-symbols:check-rounded" width="20" class="text-(--rg-g)" />{notice.text}
      {#if notice.undoable}
        <button type="button" onclick={undo} class="ml-2 font-bold cursor-pointer text-(--rg-gtx)">{$t("workshop.undo")}</button>
      {/if}
    </div>
  {/if}
</div>

<style>
  .atelier-menu {
    background: var(--rg-carte);
    border: 1px solid var(--rg-bd2);
    box-shadow: 0 18px 40px rgb(0 0 0 / 0.45);
  }
</style>
