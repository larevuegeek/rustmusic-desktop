<script lang="ts">
  // Renommer les fichiers d'après un motif : aperçu à blanc à la frappe, rien n'est écrit avant confirmation.
  // Le motif s'applique aux tags de l'atelier, modifications en attente comprises.
  import PopinFooter from "#lib/components/ui/popin/PopinFooter.svelte";
  import PopinError from "#lib/components/ui/popin/PopinError.svelte";
  import Icon from "@iconify/svelte";
  import { popinStore } from "#lib/stores/ui/popin.store";
  import { t } from "#lib/i18n";
  import { messageErreur } from "#lib/helper/tools/errorTools";
  import { lireLocal, ecrireLocal } from "#lib/helper/tools/stockage";
  import { tagWorkshop, WORKSHOP_FIELDS } from "#lib/stores/tags/tagWorkshop.store";
  import ConfirmBand from "#lib/components/ui/feedback/ConfirmBand.svelte";
  import RenamePatternEditor from "#lib/components/library/common/popin/rename/RenamePatternEditor.svelte";
  import RenamePreviewTable from "#lib/components/library/common/popin/rename/RenamePreviewTable.svelte";
  import RenameOutcome from "#lib/components/library/common/popin/rename/RenameOutcome.svelte";
  import {
    applyRename, checkPattern, orderMoves, previewRename, undoBatch, libraryDirs, PRESETS, TREE_PRESETS,
    type LibraryDir, type MoveOutcome, type RenamePreview,
  } from "#lib/services/tags/rename.service";

  /* eslint-disable svelte/valid-prop-names-in-kit-pages */
  const { libraryId = null, onapplied = () => {} }: {
    libraryId?: number | null;
    /** Les fichiers ont bougé : on passe les couples avant/après, les anciens chemins ne désignent plus rien. */
    onapplied?: (moved: [string, string][]) => void;
  } = $props();

  /** Le motif retenu la dernière fois : on retape rarement le sien. */
  const STORAGE_KEY = "rustmusic:rename-pattern";
  const TREE_KEY = "rustmusic:tree-pattern";

  let workshop = $derived($tagWorkshop);
  /** Les lignes cochées, ou tout l'atelier si rien n'est coché. */
  let targets = $derived(
    workshop.selected.size > 0
      ? workshop.files.filter((f) => f.readable && workshop.selected.has(f.path))
      : workshop.files.filter((f) => f.readable),
  );

  // Renommer ou recomposer l'arborescence : un motif mémorisé par mode, ils n'ont rien en commun.
  let restructure = $state(false);
  let namePattern = $state(lireLocal(STORAGE_KEY, "") || PRESETS[0].pattern);
  let treePattern = $state(lireLocal(TREE_KEY, "") || TREE_PRESETS[0].pattern);
  let pattern = $derived(restructure ? treePattern : namePattern);
  const setPattern = (value: string) => {
    if (restructure) treePattern = value;
    else namePattern = value;
  };

  /** Destination : l'un des dossiers scannés de la bibliothèque. */
  let dirs: LibraryDir[] = $state([]);
  let root = $state("");
  /** Emporter paroles, feuillet et pochette. */
  let satellites = $state(true);
  /** Supprimer les dossiers devenus vides. */
  let cleanup = $state(true);

  $effect(() => {
    if (libraryId === null) return;
    libraryDirs(libraryId)
      .then((list) => {
        dirs = list;
        if (!root && list.length > 0) root = list[0].path;
      })
      .catch(() => (dirs = []));
  });
  let preview: RenamePreview | null = $state(null);
  let patternError: string | null = $state(null);
  let computing = $state(false);

  // On mémorise les exclus, pas les inclus : une ligne qui apparaît à la frappe est cochée d'office.
  let excluded = $state(new Set<string>());

  /** Vrai entre la demande de confirmation et la réponse. */
  let confirming = $state(false);
  let applying = $state(false);
  /** Ce que le lot a produit — et par quoi on peut l'annuler. */
  let outcome: MoveOutcome | null = $state(null);
  let undoing = $state(false);
  let applyError: string | null = $state(null);

  // Recalcul à la frappe, temporisé : sur un partage réseau, l'aperçu interroge le disque.
  let timer: ReturnType<typeof setTimeout> | null = null;
  $effect(() => {
    const current = pattern;
    const files = targets;
    // Mode et destination font partie du calcul.
    void restructure;
    void root;
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => run(current, files), 250);
    return () => {
      if (timer) clearTimeout(timer);
    };
  });

  async function run(value: string, files: typeof targets) {
    if (!value.trim() || files.length === 0) {
      preview = null;
      return;
    }
    if (restructure && !root) {
      preview = null;
      patternError = $t("rename.no_destination");
      return;
    }

    // Syntaxe d'abord : l'erreur s'affiche sans attendre le calcul sur des milliers de fichiers.
    try {
      await checkPattern(value);
      patternError = null;
    } catch (e) {
      patternError = messageErreur(e);
      preview = null;
      return;
    }

    computing = true;
    try {
      preview = await previewRename(
        files.map((file) => ({
          path: file.path,
          tags: Object.fromEntries(
            WORKSHOP_FIELDS.map((field) => [
              field,
              file.edits[field] ?? file.original[field] ?? "",
            ]),
          ),
        })),
        value,
        restructure,
        restructure ? root : null,
      );
      ecrireLocal(restructure ? TREE_KEY : STORAGE_KEY, value);
    } catch (e) {
      patternError = messageErreur(e);
      preview = null;
    } finally {
      computing = false;
    }
  }

  // Lance le lot ; le backend ordonne d'abord les déplacements (a → b, b → c) et refuse les cycles.
  async function apply() {
    const current: RenamePreview | null = preview;
    if (!current || applying) return;

    const pairs: [string, string][] = selected.map((m) => [m.from, m.to]);
    if (pairs.length === 0) return;

    applying = true;
    applyError = null;
    try {
      const ordered = await orderMoves(pairs);
      outcome = await applyRename(ordered, pattern, libraryId, restructure, {
        satellites,
        cleanup_empty: cleanup,
      });
      confirming = false;
      onapplied(outcome.moved);
    } catch (e) {
      applyError = messageErreur(e);
    } finally {
      applying = false;
    }
  }

  async function undo() {
    const done: MoveOutcome | null = outcome;
    if (!done || undoing) return;
    undoing = true;
    applyError = null;
    try {
      const undone = await undoBatch(done.batch_id);
      // Le lot annulé n'est plus annulable : on repasse à l'aperçu.
      outcome = null;
      onapplied(undone.moved);
      await run(pattern, targets);
    } catch (e) {
      applyError = messageErreur(e);
    } finally {
      undoing = false;
    }
  }

  /** Les lignes qui peuvent bouger — ni bloquées, ni déjà conformes. */
  let changeable = $derived.by(() => {
    const current: RenamePreview | null = preview;
    return (current?.moves ?? []).filter((m) => !m.unchanged && !m.blocked);
  });
  let selected = $derived(changeable.filter((m) => !excluded.has(m.from)));
  let allSelected = $derived(changeable.length > 0 && selected.length === changeable.length);

  function toggleRow(path: string) {
    const next = new Set(excluded);
    if (next.has(path)) next.delete(path);
    else next.add(path);
    excluded = next;
  }

  function selectAll(all: boolean) {
    excluded = all ? new Set() : new Set(changeable.map((m) => m.from));
  }
</script>

<div class="flex-1 min-h-0 flex flex-col">
  <RenamePatternEditor
    bind:restructure
    bind:root
    bind:satellites
    bind:cleanup
    {dirs}
    {pattern}
    {patternError}
    onpatternchange={setPattern}
  />

  <RenamePreviewTable
    empty={targets.length === 0}
    {preview}
    {excluded}
    selectedCount={selected.length}
    changeableCount={changeable.length}
    {allSelected}
    ontoggle={toggleRow}
    onselectall={selectAll}
  />

  {#if applyError}
    <PopinError message={applyError} />
  {/if}

  <!-- Lot non rattrapable à la main : nombre exact annoncé, le choix par défaut ne touche à rien. -->
  <ConfirmBand
    open={confirming && preview !== null}
    busy={applying}
    cancelLabel={$t("profil.cancel")}
    confirmLabel={$t("rename.confirm_yes")}
    oncancel={() => (confirming = false)}
    onconfirm={apply}
  >
    {selected.length} {$t("rename.confirm")}
  </ConfirmBand>

  <RenameOutcome {outcome} {undoing} onundo={undo} />

  <!-- ─────────── Pied de page ─────────── -->
  <PopinFooter cancelLabel={$t("batch.close")} oncancel={() => popinStore.close()}
               submitLabel={outcome ? "" : $t("rename.apply")} submitIcon="lucide:file-pen-line" onsubmit={() => (confirming = true)}
               submitDisabled={selected.length === 0 || applying || confirming}>
    {#snippet start()}
      <span class="flex items-center gap-3 text-[11.5px]">
        {#if computing}
          <span class="flex items-center gap-1.5 text-neutral-400 dark:text-neutral-500">
            <Icon icon="lucide:loader-circle" width="12" class="animate-spin" />
            {$t("rename.computing")}
          </span>
        {:else if preview}
          <span class="flex items-center gap-1.5 text-neutral-600 dark:text-neutral-300">
            <span class="w-1.5 h-1.5 rounded-full bg-emerald-500"></span>
            <!-- Dénominateur seulement si l'on a décoché : sinon il n'apprend rien. -->
            {selected.length}{#if !allSelected} / {changeable.length}{/if}
            {$t("rename.will_change")}
          </span>
          {#if preview.blocked > 0}
            <span class="text-red-500">{preview.blocked} {$t("rename.blocked")}</span>
          {/if}
          {#if preview.unchanged > 0}
            <span class="text-neutral-400 dark:text-neutral-500">
              {preview.unchanged} {$t("rename.already_ok")}
            </span>
          {/if}
        {/if}
      </span>
    {/snippet}
  </PopinFooter>
</div>
