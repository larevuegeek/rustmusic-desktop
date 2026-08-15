<script lang="ts">
  // Édition des tags d'une sélection de morceaux.
  //
  // # Ce qui change par rapport à l'édition d'un seul fichier
  //
  // 1. **Il n'y a pas toujours de valeur d'origine.** Quand les N fichiers
  //    portent des titres différents, le champ n'a rien à afficher. On ne peut
  //    donc plus déduire « a été modifié » d'une comparaison de valeurs : il
  //    faut suivre l'intention, d'où `touched`.
  //
  // 2. **Vider un champ divergent efface N valeurs.** C'est le piège classique
  //    des éditeurs de tags — on clique dans « valeurs multiples », on tape
  //    puis on efface, et cinquante titres disparaissent. Le champ l'annonce
  //    avant, et le pied de page le compte.
  //
  // 3. **On ignore ce que contiennent les fichiers.** Poser une pochette
  //    commune passe donc par `cover` et non par `images` : le premier ne
  //    remplace que la pochette, le second décrirait la liste entière et
  //    supprimerait les livrets qu'on n'a jamais vus.
  //
  // 4. **L'écriture part en lot.** La popin se ferme tout de suite et le suivi
  //    continue dans le panneau flottant : un lot sur un partage réseau dure
  //    des minutes, pendant lesquelles l'application doit rester utilisable.
  import Icon from "@iconify/svelte";
  import { fade } from "svelte/transition";
  import { open } from "@tauri-apps/plugin-dialog";
  import { popinStore } from "$lib/stores/ui/popin.store";
  import { batchStore } from "$lib/stores/ui/batch.store";
  import { t } from "$lib/i18n";
  import TagField from "$lib/components/ui/input/TagField.svelte";
  import { writeTagsBatch, writeTagsEach, fileName } from "$lib/services/batch/batch.service";
  import {
    buildBatchEdit,
    formatBytes,
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
  /** Champs que l'utilisateur a réellement touchés — voir le point 1 ci-dessus. */
  let touched = $state(new Set<string>());
  /** Pochette commune choisie, non encore appliquée. */
  let cover: (PreparedImage & { path: string }) | null = $state(null);

  let isLoading = $state(true);
  let isSubmitting = $state(false);
  let coverBusy = $state(false);
  let error: string | null = $state(null);
  let confirmingClose = $state(false);

  $effect(() =>
    popinStore.guard(() => {
      if (!dirty || isSubmitting) return true;
      confirmingClose = true;
      return false;
    }),
  );

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
            for (const key of FIELDS) snapshot[key] = str((tags as any)[key]);
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

  /**
   * Numérote la sélection de 1 à N, dans l'ordre affiché.
   *
   * La corvée la plus fréquente après un import. C'est la seule modification
   * de cet écran qui ne soit pas commune : chaque fichier reçoit sa propre
   * valeur, d'où un lot individualisé (`writeTagsEach`) au lieu du lot partagé.
   *
   * L'ordre est celui de la sélection, affiché dans la colonne de gauche pour
   * qu'on puisse le vérifier avant d'écrire — un lot mal numéroté se corrige
   * fichier par fichier.
   */
  let numbering = $state(false);

  function toggleNumbering() {
    numbering = !numbering;
    // Numéroter sans renseigner le total laisserait le travail à moitié fait :
    // les deux vont ensemble sur un album.
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
      error = String((e as any)?.message ?? e ?? "");
    } finally {
      coverBusy = false;
    }
  }

  function buildPayload(): TagEditPayload {
    const edit = buildBatchEdit(common, form, touched);
    if (cover) {
      // `cover` et non `images` : voir le point 3 en tête de fichier.
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
      error = String((e as any)?.message ?? e ?? "");
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

{#snippet groupTitle(labelKey: string)}
  <div class="flex items-center gap-2.5 mb-2">
    <span class="text-[10px] font-semibold uppercase tracking-[0.14em]
                 text-neutral-400 dark:text-neutral-500">
      {$t(labelKey)}
    </span>
    <span class="flex-1 h-px bg-linear-to-r
                 from-neutral-200 to-transparent
                 dark:from-white/10 dark:to-transparent"></span>
  </div>
{/snippet}

<div class="flex-1 min-h-0 flex flex-col">
  <div class="flex-1 min-h-0 flex">
    <!-- ─────────── Colonne de gauche : la sélection ─────────── -->
    <aside
      class="shrink-0 w-64 flex flex-col gap-3.5 p-4 overflow-y-auto scrollbar-none
             border-r border-neutral-200/70 dark:border-white/8
             bg-neutral-50/70 dark:bg-black/20"
    >
      <!-- Pochette commune. Elle remplace la pochette de chaque fichier et
           laisse le reste de ses images en place. -->
      <div
        class="relative group aspect-square rounded-xl overflow-hidden shrink-0
               ring-1 ring-black/5 dark:ring-white/10
               bg-neutral-200/60 dark:bg-white/5"
      >
        {#if cover}
          <img src={cover.src} alt="" class="w-full h-full object-cover" />
          <span
            class="absolute top-2 left-2 pointer-events-none
                   flex items-center gap-1 px-1.5 py-0.5 rounded-md
                   text-[10px] font-bold uppercase tracking-wider
                   bg-emerald-500 text-white shadow-md shadow-black/30"
          >
            <Icon icon="lucide:star" width="9" />
            {$t('tags.cover')}
          </span>
          <button
            type="button"
            onclick={() => (cover = null)}
            title={$t('tags.remove_image')}
            aria-label={$t('tags.remove_image')}
            class="absolute top-2 right-2 w-6 h-6 rounded-md cursor-pointer
                   flex items-center justify-center
                   bg-black/70 text-white hover:bg-red-500 transition-colors"
          >
            <Icon icon="lucide:x" width="12" />
          </button>
        {:else}
          <button
            type="button"
            onclick={pickCover}
            disabled={coverBusy || isSubmitting}
            class="w-full h-full flex flex-col items-center justify-center gap-2 px-3
                   cursor-pointer transition-colors text-center
                   text-neutral-400 dark:text-neutral-500
                   hover:text-emerald-600 dark:hover:text-emerald-400
                   hover:bg-emerald-500/8 disabled:opacity-50"
          >
            <Icon
              icon={coverBusy ? 'lucide:loader-circle' : 'lucide:image-plus'}
              width="26"
              class={coverBusy ? 'animate-spin' : ''}
            />
            <span class="text-[11px] font-medium leading-tight">
              {$t('tags.common_cover')}
            </span>
          </button>
        {/if}
      </div>

      {#if cover}
        <p class="shrink-0 text-[10px] text-neutral-400 dark:text-neutral-500">
          {cover.mime_type.replace('image/', '').toUpperCase()} · {formatBytes(cover.bytes)}
          {#if cover.recompressed}
            <span class="text-amber-500">· {$t('tags.reduced')}</span>
          {/if}
          <br />
          {$t('tags.cover_keeps_others')}
        </p>
      {/if}

      <!-- Ce sur quoi on agit. La liste est là pour vérifier avant d'écrire :
           un lot mal ciblé ne se rattrape pas. -->
      <div class="shrink-0">
        <p class="mb-1.5 flex items-center gap-1.5 text-[10px] font-semibold uppercase
                  tracking-[0.09em] text-neutral-400 dark:text-neutral-500">
          {$t('tags.selection')}
          <span class="px-1 rounded bg-neutral-200/70 dark:bg-white/10
                       text-neutral-500 dark:text-neutral-400 tabular-nums">
            {paths.length}
          </span>
        </p>
        <div class="max-h-52 overflow-y-auto scrollbar-none space-y-0.5">
          {#each paths as path, index (path)}
            <p
              class="flex items-baseline gap-1.5 px-2 py-1 rounded text-[11px]
                     text-neutral-600 dark:text-neutral-300
                     hover:bg-neutral-200/50 dark:hover:bg-white/5"
              title={path}
            >
              {#if numbering}
                <span class="shrink-0 w-4 text-right tabular-nums font-medium
                             text-emerald-600 dark:text-emerald-400">
                  {index + 1}
                </span>
              {/if}
              <span class="min-w-0 truncate">{fileName(path)}</span>
            </p>
          {/each}
        </div>

        <button
          type="button"
          onclick={toggleNumbering}
          disabled={isSubmitting}
          class="mt-2 w-full flex items-center justify-center gap-1.5 px-2 h-7 rounded-lg
                 text-[11px] font-medium cursor-pointer transition-colors
                 disabled:opacity-40
                 {numbering
                   ? 'bg-emerald-500/15 text-emerald-600 dark:text-emerald-400'
                   : 'text-neutral-500 dark:text-neutral-400 hover:bg-neutral-200/60 dark:hover:bg-white/6'}"
        >
          <Icon icon={numbering ? 'lucide:check' : 'lucide:list-ordered'} width="12" />
          {$t('tags.number_tracks')}
        </button>
      </div>
    </aside>

    <!-- ─────────── Colonne des champs ─────────── -->
    <div class="flex-1 min-w-0 overflow-y-auto scrollbar-none px-5 py-4 space-y-5">
      {#if isLoading}
        <div class="space-y-5" aria-hidden="true">
          {#each [2, 4, 2] as rows, group (group)}
            <div class="space-y-2">
              <div class="h-2 w-20 rounded bg-neutral-200/80 dark:bg-white/8 animate-pulse"></div>
              <div class="space-y-1.5">
                {#each Array.from({ length: rows }) as _, i (i)}
                  <div class="h-12.5 rounded-lg animate-pulse
                              bg-neutral-100/70 dark:bg-white/4"></div>
                {/each}
              </div>
            </div>
          {/each}
        </div>
      {:else}
        <section>
          {@render groupTitle('tags.section_track')}
          <div class="space-y-1.5">
            {@render field('title', 'tags.title', true, false)}
            {@render field('artist', 'tags.artist', false, false)}
          </div>
        </section>

        <section>
          {@render groupTitle('tags.section_album')}
          <div class="space-y-1.5">
            {@render field('album', 'tags.album', false, false)}
            {@render field('album_artist', 'tags.album_artist', false, false)}
            <div class="grid grid-cols-3 gap-1.5">
              {@render field('year', 'tags.year', false, false)}
              <div class="col-span-2">
                {@render field('genre', 'tags.genre', false, false)}
              </div>
            </div>
            <div class="grid grid-cols-2 gap-1.5">
              {@render field('disc_number', 'tags.disc_short', false, false)}
              {@render field('total_discs', 'tags.total_discs', false, false)}
            </div>
            <!-- Le numéro de piste n'a pas sa place ici : cet écran applique la
                 MÊME valeur à tous les fichiers, il numéroterait tout à
                 l'identique. Seul le total est commun à un album. -->
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
          </div>
        </section>

        <section>
          {@render groupTitle('tags.section_extra')}
          <div class="space-y-1.5">
            {@render field('composer', 'tags.composer', false, false)}
            {@render field('comment', 'tags.comment', false, true)}
          </div>
        </section>
      {/if}
    </div>
  </div>

  {#if error}
    <div
      class="shrink-0 flex items-start gap-2 px-5 py-2
             bg-red-500/10 border-t border-red-500/20 text-red-500 text-[11px]"
      transition:fade={{ duration: 120 }}
    >
      <Icon icon="lucide:alert-triangle" width="13" class="shrink-0 mt-0.5" />
      <span class="min-w-0">{error}</span>
    </div>
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

  {#if confirmingClose && dirty}
    <div
      class="shrink-0 flex items-center gap-3 px-4 py-2.5
             bg-amber-500/10 border-t border-amber-500/25"
      transition:fade={{ duration: 120 }}
    >
      <Icon icon="lucide:triangle-alert" width="14" class="shrink-0 text-amber-500" />
      <span class="flex-1 min-w-0 text-[11.5px] text-amber-700 dark:text-amber-300">
        {$t('tags.unsaved')}
      </span>
      <button
        type="button"
        onclick={() => popinStore.close()}
        class="px-3 h-7 rounded-lg text-[12px] cursor-pointer transition-colors
               text-amber-700 dark:text-amber-300 hover:bg-amber-500/15"
      >
        {$t('tags.discard')}
      </button>
      <button
        type="button"
        onclick={() => (confirmingClose = false)}
        class="px-3 h-7 rounded-lg text-[12px] font-medium cursor-pointer transition-colors
               bg-amber-500 text-white hover:bg-amber-600"
      >
        {$t('tags.keep_editing')}
      </button>
    </div>
  {/if}

  <footer
    class="shrink-0 flex items-center gap-3 px-4 py-2.5
           border-t border-neutral-200/70 dark:border-white/8
           bg-neutral-50/80 dark:bg-black/25"
  >
    <div class="flex-1 min-w-0 flex items-center gap-2">
      {#if dirty}
        <span class="flex items-center gap-1.5 text-[11px] text-neutral-600 dark:text-neutral-300">
          <span class="w-1.5 h-1.5 rounded-full bg-emerald-500"></span>
          {#if changed.length}
            {changed.length}
            {changed.length > 1 ? $t('tags.changed_other') : $t('tags.changed_one')}
          {/if}
          {#if changed.length && cover}·{/if}
          {#if cover}{$t('tags.cover')}{/if}
        </span>
        <button
          type="button"
          onclick={revertAll}
          disabled={isSubmitting}
          class="text-[11px] cursor-pointer underline decoration-dotted underline-offset-2
                 text-neutral-400 dark:text-neutral-500
                 hover:text-neutral-700 dark:hover:text-neutral-200
                 disabled:opacity-40 transition-colors"
        >
          {$t('tags.reset_all')}
        </button>
      {:else}
        <span class="text-[11px] text-neutral-400 dark:text-neutral-500">
          {$t('tags.no_change')}
        </span>
      {/if}

      <span
        title={$t('tags.batch_hint')}
        class="text-neutral-300 dark:text-neutral-600 cursor-help
               hover:text-neutral-500 dark:hover:text-neutral-400 transition-colors"
      >
        <Icon icon="lucide:info" width="12" />
      </span>
    </div>

    <button
      type="button"
      onclick={() => popinStore.requestClose()}
      disabled={isSubmitting}
      class="flex items-center gap-1.5 px-3 h-8 rounded-lg text-[13px]
             cursor-pointer transition-colors
             text-neutral-600 dark:text-neutral-300
             hover:bg-neutral-200/70 dark:hover:bg-white/8 disabled:opacity-50"
    >
      <Icon icon="lucide:x" width="13" />
      {$t('profil.cancel')}
    </button>
    <button
      type="button"
      onclick={submit}
      disabled={!dirty || isSubmitting}
      class="flex items-center gap-1.5 px-3.5 h-8 rounded-lg text-[13px] font-medium
             cursor-pointer transition-all
             bg-emerald-500 text-white
             hover:bg-emerald-600 hover:shadow-lg hover:shadow-emerald-500/40
             disabled:opacity-35 disabled:cursor-not-allowed disabled:shadow-none"
    >
      {#if isSubmitting}
        <Icon icon="lucide:loader-circle" width="13" class="animate-spin" />
      {:else}
        <Icon icon="lucide:check" width="13" />
      {/if}
      {$t('tags.apply_to')} {paths.length}
    </button>
  </footer>
</div>
