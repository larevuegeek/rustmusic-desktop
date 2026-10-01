<script lang="ts">
  // Édition des tags d'une sélection : sans valeur d'origine commune, on suit l'intention (`touched`).
  // L'écriture part en lot : la popin se ferme et le suivi continue dans le panneau flottant.
  import Icon from "@iconify/svelte";
  import { fade } from "svelte/transition";
  import { open } from "@tauri-apps/plugin-dialog";
  import { messageErreur } from "$lib/helper/tools/errorTools";
  import { popinStore } from "$lib/stores/ui/popin.store";
  import { batchStore } from "$lib/stores/ui/batch.store";
  import { t } from "$lib/i18n";
  import TagField from "$lib/components/ui/input/TagField.svelte";
  import PopinError from "$lib/components/ui/popin/PopinError.svelte";
  import PopinFooter from "$lib/components/ui/popin/PopinFooter.svelte";
  import TagEditorLayout from "./tags/TagEditorLayout.svelte";
  import TagSelectionColumn from "./tags/TagSelectionColumn.svelte";
  import TagFieldsForm from "./tags/TagFieldsForm.svelte";
  import TagChangesSummary from "./tags/TagChangesSummary.svelte";
  import UnsavedChangesBar from "./tags/UnsavedChangesBar.svelte";
  import { writeTagsBatch, writeTagsEach } from "$lib/services/batch/batch.service";
  import {
    buildBatchEdit,
    mergeTags,
    prepareImage,
    readTrackTags,
    MIXED,
    PICTURE_TYPE_COVER,
    type CommonValue,
    type PreparedImage,
    type TagEditPayload,
  } from "$lib/services/tags/tagEditor.service";

  /* eslint-disable svelte/valid-prop-names-in-kit-pages */
  const {
    paths,
    onsaved = () => {},
  }: {
    paths: string[];
    onsaved?: () => void;
  } = $props();

  const FIELDS = [
    "title", "artist", "album", "album_artist",
    "year", "genre", "composer", "comment",
    "track_number", "total_tracks", "disc_number", "total_discs",
  ] as const;

  const str = (v: unknown): string =>
    v === null || v === undefined ? "" : String(v);

  /** Valeur commune par champ, ou `MIXED`. */
  let common: Record<string, CommonValue> = $state({});
  let form = $state<Record<string, string>>({});
  /** Champs réellement touchés : un champ divergent n'a pas de valeur à comparer. */
  let touched = $state(new Set<string>());
  /** Pochette commune choisie, non encore appliquée. */
  let cover: (PreparedImage & { path: string }) | null = $state(null);

  let isLoading = $state(true);
  let isSubmitting = $state(false);
  let coverBusy = $state(false);
  let error: string | null = $state(null);
  let confirmingClose = $state(false);

  $effect(() => {
    const selection = paths;
    isLoading = true;
    error = null;

    // Les tags sont lus DANS chaque fichier : la vue de la bibliothèque ignore
    // commentaire, compositeur et totaux, et les afficher vides les effacerait.
    Promise.all(selection.map((p) => readTrackTags(p).catch(() => null)))
      .then((results) => {
        const perFile = results
          .filter((tags): tags is NonNullable<typeof tags> => tags !== null)
          .map((tags) => {
            const snapshot: Record<string, string> = {};
            for (const key of FIELDS) snapshot[key] = str((tags as Record<string, unknown>)[key]);
            return snapshot;
          });

        if (perFile.length === 0) {
          error = $t("tags.batch_unreadable");
          return;
        }

        common = mergeTags(perFile);
        const start: Record<string, string> = {};
        // Un champ divergent démarre vide : afficher la valeur du premier
        // fichier laisserait croire qu'elle vaut pour tous.
        for (const key of FIELDS) {
          start[key] = common[key] === MIXED ? "" : (common[key] as string);
        }
        form = start;
        touched = new Set();
      })
      .finally(() => (isLoading = false));
  });

  const isMixed = (key: string) => common[key] === MIXED;

  function isDirty(key: string): boolean {
    if (isMixed(key)) return touched.has(key);
    return (form[key] ?? "") !== ((common[key] as string) ?? "");
  }

  /** Vrai quand valider effacerait des valeurs qui divergent aujourd'hui. */
  function willErase(key: string): boolean {
    return isMixed(key) && touched.has(key) && (form[key] ?? "").trim() === "";
  }

  function markTouched(key: string) {
    if (touched.has(key)) return;
    touched = new Set(touched).add(key);
  }

  function revert(key: string) {
    form[key] = isMixed(key) ? "" : ((common[key] as string) ?? "");
    const next = new Set(touched);
    next.delete(key);
    touched = next;
  }

  function revertAll() {
    for (const key of FIELDS) form[key] = isMixed(key) ? "" : ((common[key] as string) ?? "");
    touched = new Set();
    cover = null;
  }

  /** Numérote la sélection de 1 à N dans l'ordre affiché : seule modification
   *  propre à chaque fichier, d'où un lot individualisé (`writeTagsEach`). */
  let numbering = $state(false);

  function toggleNumbering() {
    numbering = !numbering;
    // Numéroter sans le total laisserait le travail à moitié fait.
    if (numbering && !touched.has("total_tracks")) {
      form.total_tracks = String(paths.length);
      markTouched("total_tracks");
    }
  }

  let changed = $derived(FIELDS.filter((key) => isDirty(key)));
  let erasing = $derived(FIELDS.filter((key) => willErase(key)));
  let dirty = $derived(changed.length > 0 || cover !== null || numbering);

  async function pickCover() {
    const selected = await open({
      multiple: false,
      filters: [{ name: "Images", extensions: ["jpg", "jpeg", "png", "webp", "gif", "bmp"] }],
    });
    if (typeof selected !== "string") return;

    coverBusy = true;
    error = null;
    try {
      const prepared = await prepareImage(selected);
      cover = { ...prepared, path: selected };
    } catch (e) {
      error = messageErreur(e);
    } finally {
      coverBusy = false;
    }
  }

  function buildPayload(): TagEditPayload {
    const edit = buildBatchEdit(common, form, touched);
    if (cover) {
      // `cover` et non `images` : on ignore le contenu des fichiers, `images`
      // supprimerait les livrets jamais vus.
      edit.cover = { path: cover.path, picture_type: PICTURE_TYPE_COVER };
    }
    return edit;
  }

  async function submit() {
    if (!dirty || isSubmitting) return;
    if (batchStore.isRunning()) {
      error = $t("tags.batch_already_running");
      return;
    }

    error = null;
    isSubmitting = true;
    try {
      const edit = buildPayload();
      const targets = [...paths];

      // Une relance ne rejoue que les échecs : les numéros doivent rester ceux
      // du lot d'origine, pas être recalculés sur la liste réduite.
      const numbers = new Map(targets.map((path, i) => [path, i + 1]));

      const launch = async (files: string[]) => {
        const jobId = numbering
          ? await writeTagsEach(
              files.map((path) => ({
                path,
                edit: { ...edit, track_number: String(numbers.get(path) ?? 1) },
              })),
            )
          : await writeTagsBatch(files, edit);
        batchStore.begin(jobId, $t("batch.writing_tags"), launch);
      };

      await launch(targets);
      onsaved();
      popinStore.close();
    } catch (e) {
      error = messageErreur(e);
    } finally {
      isSubmitting = false;
    }
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Escape" && confirmingClose) {
      e.stopPropagation();
      confirmingClose = false;
      return;
    }
    if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) submit();
  }

  const placeholderFor = (key: string) =>
    isMixed(key) ? $t("tags.multiple_values") : "—";
</script>

<svelte:window onkeydowncapture={onKeydown} />

{#snippet field(key: string, labelKey: string, large: boolean, multiline: boolean)}
  <TagField
    label={$t(labelKey)}
    bind:value={form[key]}
    placeholder={placeholderFor(key)}
    dirty={isDirty(key)}
    disabled={isSubmitting || isLoading}
    warning={willErase(key) ? $t("tags.will_erase") : ""}
    revertLabel={$t("tags.revert")}
    {large}
    {multiline}
    oninput={() => markTouched(key)}
    onrevert={() => revert(key)}
  />
{/snippet}

<!-- Pas de numéro de piste ici : la même valeur irait à tous les fichiers.
     Seul le total est commun à un album. -->
{#snippet numbers()}
  <div class="grid grid-cols-2 gap-1.5">
    {@render field('disc_number', 'tags.disc_short', false, false)}
    {@render field('total_discs', 'tags.total_discs', false, false)}
  </div>
  <div class="grid grid-cols-2 gap-1.5">
    <div class="flex items-center gap-2 px-3 py-2 rounded-lg
                bg-neutral-100/50 dark:bg-white/2
                text-[10px] leading-tight
                text-neutral-400 dark:text-neutral-500">
      <Icon icon="lucide:info" width="12" class="shrink-0" />
      {$t('tags.track_number_per_file')}
    </div>
    {@render field('total_tracks', 'tags.total_tracks', false, false)}
  </div>
{/snippet}

<div class="flex-1 min-h-0 flex flex-col">
  <TagEditorLayout>
    {#snippet aside()}
      <TagSelectionColumn
        {cover}
        {coverBusy}
        submitting={isSubmitting}
        {paths}
        {numbering}
        onpickcover={pickCover}
        onremovecover={() => (cover = null)}
        ontogglenumbering={toggleNumbering}
      />
    {/snippet}

    <!-- ─────────── Colonne des champs ─────────── -->
    <div class="flex-1 min-w-0 overflow-y-auto scrollbar-none px-5 py-4 space-y-5">
      <TagFieldsForm loading={isLoading} {field} {numbers} />
    </div>
  </TagEditorLayout>

  {#if error}
    <PopinError message={error} />
  {/if}

  <!-- Ce que valider va effacer, dit avant de valider et non après. -->
  {#if erasing.length > 0}
    <div
      class="shrink-0 flex items-start gap-2 px-5 py-2
             bg-amber-500/10 border-t border-amber-500/25
             text-amber-700 dark:text-amber-300 text-[11px]"
      transition:fade={{ duration: 120 }}
    >
      <Icon icon="lucide:triangle-alert" width="13" class="shrink-0 mt-0.5" />
      <span class="min-w-0">
        {erasing.length}
        {erasing.length > 1 ? $t('tags.fields_will_be_erased') : $t('tags.field_will_be_erased')}
        · {paths.length} {$t('tags.files')}
      </span>
    </div>
  {/if}

  <UnsavedChangesBar {dirty} busy={isSubmitting} bind:confirming={confirmingClose} />

  <PopinFooter
    cancelLabel={$t('profil.cancel')}
    oncancel={() => popinStore.requestClose()}
    cancelDisabled={isSubmitting}
    submitLabel="{$t('tags.apply_to')} {paths.length}"
    onsubmit={submit}
    submitDisabled={!dirty || isSubmitting}
    busy={isSubmitting}
  >
    {#snippet start()}
      <TagChangesSummary
        {dirty}
        changedCount={changed.length}
        extraLabel={cover ? $t('tags.cover') : ''}
        hint={$t('tags.batch_hint')}
        disabled={isSubmitting}
        onreset={revertAll}
      />
    {/snippet}
  </PopinFooter>
</div>
