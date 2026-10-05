<script lang="ts">
  // Éditeur de métadonnées d'un fichier : n'envoie que ce qui a changé, images comprises.
  // Popin `flush` : deux colonnes qui défilent chacune et un pied de page fixe.
  import { open } from "@tauri-apps/plugin-dialog";
  import { messageErreur } from "#lib/helper/tools/errorTools";
  import { popinStore } from "#lib/stores/ui/popin.store";
  import TagField from "#lib/components/ui/input/TagField.svelte";
  import ImageLightbox from "#lib/components/ui/image/ImageLightbox.svelte";
  import PopinError from "#lib/components/ui/popin/PopinError.svelte";
  import PopinFooter from "#lib/components/ui/popin/PopinFooter.svelte";
  import TagEditorLayout from "./tags/TagEditorLayout.svelte";
  import TagMediaColumn from "./tags/TagMediaColumn.svelte";
  import TagFetchBar from "./tags/TagFetchBar.svelte";
  import TagFieldsForm from "./tags/TagFieldsForm.svelte";
  import TagPairField from "./tags/TagPairField.svelte";
  import TagChangesSummary from "./tags/TagChangesSummary.svelte";
  import UnsavedChangesBar from "./tags/UnsavedChangesBar.svelte";
  import DeezerLookupPane from "./tags/DeezerLookupPane.svelte";
  import type { TrackHit, TrackValues } from "#lib/services/tags/metadata.service";
  import { t } from "#lib/i18n";
  import {
    buildEdit,
    formatBytes,
    pictureTypeKey,
    prepareImage,
    type DownloadedImage,
    type PreparedImage,
    promoteToCover,
    readTrackImages,
    readTrackTags,
    slotsFromImages,
    slotsSignature,
    slotsToPayload,
    writeTrackTags,
    PICTURE_TYPE_COVER,
    PICTURE_TYPE_OTHER,
    type MediaSlot,
  } from "#lib/services/tags/tagEditor.service";

  /* eslint-disable svelte/valid-prop-names-in-kit-pages */
  const {
    path,
    onsaved = () => {},
  }: {
    path: string;
    onsaved?: () => void;
  } = $props();

  const FIELDS = [
    "title", "artist", "album", "album_artist",
    "year", "genre", "composer", "comment",
    "track_number", "total_tracks", "disc_number", "total_discs",
  ] as const;

  /** Normalise une valeur du backend en chaîne éditable. */
  const str = (v: unknown): string =>
    v === null || v === undefined ? "" : String(v);

  // Lues DANS LE FICHIER : la bibliothèque ignore commentaire, compositeur et
  // totaux, les afficher vides puis réenregistrer les effacerait.
  let original: Record<string, string> = $state({});
  let form = $state<Record<string, string>>({});
  /** Liste des médias en cours d'édition, et son état d'origine. */
  let slots: MediaSlot[] = $state([]);
  let originalSlots: MediaSlot[] = $state([]);
  /** Média affiché en grand par-dessus la popin (`null` = aucun). */
  let zoomed: MediaSlot | null = $state(null);
  let isLoading = $state(true);
  let isSubmitting = $state(false);
  /** Vrai pendant le choix et la préparation d'une image. */
  let mediaBusy = $state(false);
  let error: string | null = $state(null);
  /** Vrai quand une fermeture a été refusée faute d'enregistrement. */
  let confirmingClose = $state(false);
  /** Conteneur des champs, pour y poser le focus dès qu'ils existent. */
  let fieldsPane: HTMLElement | null = $state(null);
  /** Compteur de clés pour les images ajoutées, qui n'ont pas d'identifiant. */
  let addedCount = 0;
  /** Vrai quand la récupération Deezer occupe la popin. */
  let lookup = $state(false);
  /** La piste retenue, dont on compare les valeurs aux nôtres. */
  let picked: { values: TrackValues; hit: TrackHit } | null = $state(null);
  /** Ce que Deezer vient de remplir, pour le dire une fois et l'oublier. */
  let filled: string[] = $state([]);

  // Focus sur le premier champ, une fois chargé : désactivé avant, il ne le prendrait pas.
  $effect(() => {
    if (isLoading || !fieldsPane) return;
    fieldsPane.querySelector("input")?.focus();
  });

  $effect(() => {
    const p = path;
    isLoading = true;
    readTrackTags(p)
      .then((tags) => {
        const snapshot: Record<string, string> = {};
        for (const key of FIELDS) snapshot[key] = str((tags as Record<string, unknown>)[key]);
        original = snapshot;
        form = { ...snapshot };
      })
      .catch((e) => {
        error = messageErreur(e);
      })
      .finally(() => (isLoading = false));

    // Les images intégrées portent déjà la pochette ; leur absence ne doit pas
    // empêcher l'édition du texte.
    readTrackImages(p)
      .then((list) => {
        originalSlots = slotsFromImages(list);
        slots = originalSlots.map((s) => ({ ...s }));
      })
      .catch(() => {
        originalSlots = [];
        slots = [];
      });
  });

  // La pochette est celle **typée** comme telle, pas la première de la liste.
  // À défaut de type, la première fait office d'aperçu.
  let coverSlot = $derived(
    slots.find((s) => s.picture_type === PICTURE_TYPE_COVER) ?? slots[0] ?? null,
  );
  let mediaDirty = $derived(
    slotsSignature(slots) !== slotsSignature(originalSlots),
  );

  let changed = $derived(
    FIELDS.filter((k) => (form[k] ?? "") !== (original[k] ?? "")),
  );
  let dirty = $derived(changed.length > 0 || mediaDirty);

  const isDirty = (key: string) => (form[key] ?? "") !== (original[key] ?? "");

  function revert(key: string) {
    form[key] = original[key] ?? "";
  }

  function revertAll() {
    form = { ...original };
    slots = originalSlots.map((s) => ({ ...s }));
  }

  // ─── Gestes sur les médias : ils ne touchent que la liste locale ───

  /** Emplacement pour une image ajoutée ; le compteur lui donne une clé unique. */
  function newSlot(prefix: string, path: string, image: PreparedImage, pictureType: number): MediaSlot {
    addedCount += 1;
    return {
      key: `${prefix}-${addedCount}`,
      id: null,
      path,
      src: image.src,
      picture_type: pictureType,
      description: "",
      mime_type: image.mime_type,
      bytes: image.bytes,
      originalBytes: image.original_bytes,
      recompressed: image.recompressed,
    };
  }

  /** Choisit puis prépare une image. Seulement à l'ajout : une image conservée
   *  est recopiée telle quelle, sinon chaque correction la réencoderait. */
  async function pickImage(): Promise<MediaSlot | null> {
    const selected = await open({
      multiple: false,
      filters: [
        { name: "Images", extensions: ["jpg", "jpeg", "png", "webp", "gif", "bmp"] },
      ],
    });
    if (typeof selected !== "string") return null;

    mediaBusy = true;
    error = null;
    try {
      const prepared = await prepareImage(selected);
      return newSlot("nouveau", selected, prepared, PICTURE_TYPE_OTHER);
    } catch (e) {
      error = messageErreur(e);
      return null;
    } finally {
      mediaBusy = false;
    }
  }

  async function addImage() {
    const slot = await pickImage();
    if (!slot) return;
    // Première image d'un fichier qui n'en avait aucune : presque toujours une pochette.
    if (slots.length === 0) slot.picture_type = PICTURE_TYPE_COVER;
    slots = [...slots, slot];
  }

  async function replaceImage(index: number) {
    const picked = await pickImage();
    if (!picked) return;
    const current = slots[index];
    // Rôle et description appartiennent à l'emplacement : remplacer la pochette donne une pochette.
    slots = slots.map((s, i) =>
      i === index
        ? { ...picked, picture_type: current.picture_type, description: current.description }
        : s,
    );
  }

  function removeImage(index: number) {
    slots = slots.filter((_, i) => i !== index);
  }

  function setCover(index: number) {
    slots = promoteToCover(slots, index);
  }

  function move(index: number, delta: number) {
    const target = index + delta;
    if (target < 0 || target >= slots.length) return;
    const next = [...slots];
    [next[index], next[target]] = [next[target], next[index]];
    slots = next;
  }

  /** Libellé d'un type d'image, ou repli numérique pour les types rares. */
  function typeLabel(type: number): string {
    const key = pictureTypeKey(type);
    return key ? $t(key) : `${$t('tags.pic.other')} (${type})`;
  }

  let filename = $derived(path.split(/[\\/]/).pop() ?? path);

  // ─── Récupération depuis Deezer ───

  /** Ferme la récupération et oublie la piste retenue. */
  function closeLookup() {
    lookup = false;
    picked = null;
  }

  /** Verse dans le formulaire les champs cochés dans la comparaison ; rien
   *  n'est écrit avant l'enregistrement, et chaque champ reste rétablissable. */
  function applyFromSource(
    fields: Record<string, string>,
    cover: DownloadedImage | null,
  ) {
    const touched: string[] = [];
    for (const [key, value] of Object.entries(fields)) {
      if (form[key] === value) continue;
      form[key] = value;
      touched.push(key);
    }

    if (cover) {
      applyCover(cover);
      touched.push("cover");
    }

    filled = touched;
    closeLookup();
  }

  /** Remplace la pochette typée à sa place (livret et dos survivent), ou la
   *  pose en tête d'un fichier qui n'en avait pas. */
  function applyCover(image: DownloadedImage) {
    const slot = newSlot("deezer", image.path, image, PICTURE_TYPE_COVER);
    const index = slots.findIndex((s) => s.picture_type === PICTURE_TYPE_COVER);
    slots = index >= 0
      ? slots.map((s, i) => (i === index ? slot : s))
      : [slot, ...slots];
  }

  async function submit() {
    if (!dirty || isSubmitting) return;
    error = null;
    isSubmitting = true;
    try {
      const edit = buildEdit(original, form);
      // Champ absent = « ne touche pas aux images » : sans ça, corriger un
      // titre réécrirait toutes les pochettes.
      if (mediaDirty) edit.images = slotsToPayload(slots);

      await writeTrackTags(path, edit);
      onsaved();
      popinStore.close();
    } catch (e) {
      // Message explicite du backend (format, verrou, image illisible…), montré tel quel.
      error = messageErreur(e);
    } finally {
      isSubmitting = false;
    }
  }

  // En **capture** : on passe avant l'Échap de la popin, pour refermer d'abord
  // la couche du dessus (loupe, comparaison, recherche, confirmation).
  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Escape" && zoomed) {
      e.stopPropagation();
      zoomed = null;
      return;
    }
    // La comparaison d'abord, la recherche ensuite : on voulait peut-être juste changer de piste.
    if (e.key === "Escape" && picked) {
      e.stopPropagation();
      picked = null;
      return;
    }
    if (e.key === "Escape" && lookup) {
      e.stopPropagation();
      closeLookup();
      return;
    }
    if (e.key === "Escape" && confirmingClose) {
      e.stopPropagation();
      confirmingClose = false;
      return;
    }
    if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) submit();
  }
</script>

<svelte:window onkeydowncapture={onKeydown} />

<!-- Le commentaire, multiligne, ne rappelle pas son ancienne valeur au rétablissement. -->
{#snippet field(key: string, labelKey: string, large: boolean, multiline: boolean)}
  <TagField
    label={$t(labelKey)}
    bind:value={form[key]}
    dirty={isDirty(key)}
    disabled={isSubmitting || isLoading}
    revertLabel={multiline ? $t('tags.revert') : `${$t('tags.revert')} : ${original[key] || '—'}`}
    {large}
    {multiline}
    onrevert={() => revert(key)}
  />
{/snippet}

{#snippet numbers()}
  <div class="grid grid-cols-2 gap-1.5">
    <TagPairField
      label={$t('tags.track_short')}
      bind:number={form.track_number}
      bind:total={form.total_tracks}
      dirty={isDirty('track_number') || isDirty('total_tracks')}
      disabled={isSubmitting || isLoading}
    />
    <TagPairField
      label={$t('tags.disc_short')}
      bind:number={form.disc_number}
      bind:total={form.total_discs}
      dirty={isDirty('disc_number') || isDirty('total_discs')}
      disabled={isSubmitting || isLoading}
    />
  </div>
{/snippet}

<div class="flex-1 min-h-0 flex flex-col">
  {#if lookup}
    <DeezerLookupPane
      {form}
      {filename}
      {coverSlot}
      bind:picked
      onclose={closeLookup}
      onapply={applyFromSource}
    />
  {:else}
    <TagEditorLayout>
      {#snippet aside()}
        <TagMediaColumn
          {slots}
          {coverSlot}
          busy={mediaBusy}
          submitting={isSubmitting}
          {path}
          {filename}
          {typeLabel}
          onadd={addImage}
          onreplace={replaceImage}
          onremove={removeImage}
          onsetcover={setCover}
          onmove={move}
          onzoom={(slot) => (zoomed = slot)}
        />
      {/snippet}

      <!-- ─────────── Colonne des champs ─────────── -->
      <div class="flex-1 min-w-0 flex flex-col min-h-0">
        <TagFetchBar
          {filled}
          disabled={isLoading || isSubmitting}
          onfetch={() => (lookup = true)}
        />

        <div
          bind:this={fieldsPane}
          class="flex-1 min-w-0 overflow-y-auto scrollbar-none px-5 py-4 space-y-5"
        >
          <TagFieldsForm loading={isLoading} {field} {numbers} />
        </div>
      </div>
    </TagEditorLayout>
  {/if}

  {#if error}
    <PopinError message={error} />
  {/if}

  <UnsavedChangesBar {dirty} busy={isSubmitting} bind:confirming={confirmingClose} />

  <PopinFooter
    cancelLabel={$t('profil.cancel')}
    oncancel={() => popinStore.requestClose()}
    cancelDisabled={isSubmitting}
    submitLabel={$t('profil.save')}
    onsubmit={submit}
    submitDisabled={!dirty || isSubmitting}
    busy={isSubmitting}
  >
    {#snippet start()}
      <TagChangesSummary
        {dirty}
        changedCount={changed.length}
        extraLabel={mediaDirty ? $t('tags.media_changed') : ''}
        hint={$t('tags.clear_hint')}
        disabled={isSubmitting}
        onreset={revertAll}
      />
    {/snippet}
    {#snippet end()}
      <kbd
        class="hidden sm:block px-1.5 py-0.5 rounded text-[9.5px] font-medium tabular-nums
               bg-neutral-200/60 dark:bg-white/8
               text-neutral-400 dark:text-neutral-500
               ring-1 ring-inset ring-neutral-300/50 dark:ring-white/8"
      >
        Ctrl ↵
      </kbd>
    {/snippet}
  </PopinFooter>
</div>

<!-- Loupe au-dessus de la popin : on consulte une image sans perdre la saisie. -->
{#if zoomed}
  <ImageLightbox
    src={zoomed.src}
    alt={typeLabel(zoomed.picture_type)}
    title={typeLabel(zoomed.picture_type)}
    caption="{zoomed.mime_type} · {formatBytes(zoomed.bytes)}{zoomed.description ? ` · ${zoomed.description}` : ''}"
    onclose={() => (zoomed = null)}
  />
{/if}
