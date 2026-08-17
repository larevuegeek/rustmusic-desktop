<script lang="ts">
  // Renommer les fichiers d'après un motif.
  //
  // # Rien ne bouge tant qu'on n'a pas vu
  // L'écran est un aperçu à blanc : les nouveaux noms se calculent à la frappe,
  // ligne par ligne, et rien n'est écrit. Corriger cinq mille fichiers avec un
  // motif erroné n'est pas rattrapable à la main — c'est la seule raison pour
  // laquelle on peut ensuite oser lancer le lot.
  //
  // # Le motif s'applique aux tags corrigés
  // Les valeurs viennent de l'atelier, modifications en attente comprises.
  // C'est l'enchaînement naturel : on corrige les tags, on constate que les
  // noms de fichiers ne suivent plus, on renomme d'après ce qu'on vient
  // d'écrire.
  import Icon from "@iconify/svelte";
  import { fade } from "svelte/transition";
  import { popinStore } from "$lib/stores/ui/popin.store";
  import { t } from "$lib/i18n";
  import { tagWorkshop, WORKSHOP_FIELDS } from "$lib/stores/tags/tagWorkshop.store";
  import {
    applyRename,
    checkPattern,
    orderMoves,
    previewRename,
    undoBatch,
    libraryDirs,
    PATTERN_FIELDS,
    PRESETS,
    TREE_PRESETS,
    type LibraryDir,
    type MoveOutcome,
    type RenamePreview,
  } from "$lib/services/tags/rename.service";

  /* eslint-disable svelte/valid-prop-names-in-kit-pages */
  const {
    libraryId = null,
    onapplied = () => {},
  }: {
    libraryId?: number | null;
    /**
     * Les fichiers ont bougé : à l'appelant de recharger ce qu'il tient.
     *
     * On lui passe les couples avant/après plutôt qu'un simple signal : sans
     * eux il rechargerait avec les anciens chemins, qui ne désignent plus rien.
     */
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

  /**
   * Renommer, ou recomposer l'arborescence.
   *
   * Deux motifs mémorisés séparément : celui d'un nom de fichier et celui d'une
   * arborescence n'ont rien à voir, et basculer d'un mode à l'autre en gardant
   * le motif de l'autre ne produit jamais rien de bon.
   */
  let restructure = $state(false);
  let namePattern = $state(
    (typeof localStorage !== "undefined" && localStorage.getItem(STORAGE_KEY)) ||
      PRESETS[0].pattern,
  );
  let treePattern = $state(
    (typeof localStorage !== "undefined" && localStorage.getItem(TREE_KEY)) ||
      TREE_PRESETS[0].pattern,
  );
  let pattern = $derived(restructure ? treePattern : namePattern);
  const setPattern = (value: string) => {
    if (restructure) treePattern = value;
    else namePattern = value;
  };

  let presets = $derived(restructure ? TREE_PRESETS : PRESETS);

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
  let input: HTMLInputElement | null = $state(null);

  /**
   * Fichiers écartés à la main.
   *
   * On mémorise les **exclus** et non les inclus : le motif change à la frappe,
   * donc la liste des lignes modifiables change avec lui. Une ligne qu'on vient
   * de décocher doit rester décochée, et une ligne qui apparaît doit l'être
   * d'office — c'est ce qu'on est venu faire.
   */
  let excluded = $state(new Set<string>());

  /** Vrai entre la demande de confirmation et la réponse. */
  let confirming = $state(false);
  let applying = $state(false);
  /** Ce que le lot a produit — et par quoi on peut l'annuler. */
  let outcome: MoveOutcome | null = $state(null);
  let undoing = $state(false);
  let applyError: string | null = $state(null);

  // Recalcul à la frappe, mais pas à chaque touche : sur un partage réseau,
  // l'aperçu interroge le disque pour chaque cible.
  let timer: ReturnType<typeof setTimeout> | null = null;
  $effect(() => {
    const current = pattern;
    const files = targets;
    // Le mode et la destination font partie du calcul : sans eux ici, basculer
    // en arborescence n'actualiserait pas l'aperçu.
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

    // Le motif se vérifie d'abord : un message de syntaxe doit s'afficher sans
    // attendre le calcul sur des milliers de fichiers.
    try {
      await checkPattern(value);
      patternError = null;
    } catch (e) {
      patternError = String((e as any)?.message ?? e ?? "");
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
      localStorage.setItem(restructure ? TREE_KEY : STORAGE_KEY, value);
    } catch (e) {
      patternError = String((e as any)?.message ?? e ?? "");
      preview = null;
    } finally {
      computing = false;
    }
  }

  /** Insère un champ à la position du curseur. */
  function insert(field: string) {
    const token = `{${field}}`;
    if (!input) {
      setPattern(pattern + token);
      return;
    }
    const start = input.selectionStart ?? pattern.length;
    const end = input.selectionEnd ?? start;
    setPattern(pattern.slice(0, start) + token + pattern.slice(end));
    // Le curseur reste après le champ inséré : on enchaîne les insertions
    // sans repointer à la souris entre chaque.
    queueMicrotask(() => {
      input?.focus();
      input?.setSelectionRange(start + token.length, start + token.length);
    });
  }

  /**
   * Lance le lot.
   *
   * Les déplacements sont d'abord **ordonnés** par le backend : `a → b` et
   * `b → c` ne se dénouent que dans le bon sens, et une permutation circulaire
   * est refusée franchement plutôt qu'improvisée au milieu du lot.
   */
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
      applyError = String((e as any)?.message ?? e ?? "");
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
      applyError = String((e as any)?.message ?? e ?? "");
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
  let allSelected = $derived(
    changeable.length > 0 && selected.length === changeable.length,
  );

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
  <!-- ─────────── Le motif ─────────── -->
  <div class="shrink-0 px-4 py-3 space-y-2.5
              border-b border-neutral-200/70 dark:border-white/8
              bg-neutral-50/70 dark:bg-black/20">
    <!-- Deux modes, nommés par ce qu'ils font : l'un touche au nom, l'autre
         range l'arborescence. -->
    <div class="flex gap-1">
      {#each [[false, "rename.mode_name"], [true, "rename.mode_tree"]] as [value, key] (key)}
        <button
          type="button"
          onclick={() => (restructure = value as boolean)}
          class="px-2.5 h-7 rounded-lg text-[11.5px] cursor-pointer transition-colors
                 {restructure === value
                   ? 'bg-emerald-500/15 text-emerald-600 dark:text-emerald-400 font-medium'
                   : 'text-neutral-500 dark:text-neutral-400 hover:bg-neutral-200/70 dark:hover:bg-white/8'}"
        >
          {$t(key as string)}
        </button>
      {/each}

      {#if restructure}
        <span class="flex-1"></span>
        <label class="flex items-center gap-1.5 text-[11px]
                      text-neutral-500 dark:text-neutral-400">
          {$t("rename.destination")}
          <select
            bind:value={root}
            class="h-7 px-2 rounded-lg text-[11.5px] outline-none cursor-pointer
                   bg-white dark:bg-white/5
                   ring-1 ring-inset ring-neutral-200 dark:ring-white/10
                   text-neutral-800 dark:text-neutral-100"
          >
            {#each dirs as dir (dir.id)}
              <option value={dir.path}>{dir.path}</option>
            {/each}
          </select>
        </label>
      {/if}
    </div>

    <div class="flex gap-1.5">
      <input
        bind:this={input}
        type="text"
        value={pattern}
        oninput={(e) => setPattern(e.currentTarget.value)}
        spellcheck="false"
        placeholder={PRESETS[0].pattern}
        class="flex-1 min-w-0 h-9 px-3 rounded-lg font-mono text-[12.5px] outline-none
               bg-white dark:bg-white/5
               ring-1 ring-inset
               {patternError
                 ? 'ring-red-500/60'
                 : 'ring-neutral-200 dark:ring-white/10 focus:ring-emerald-500/60'}
               text-neutral-900 dark:text-neutral-100
               placeholder:text-neutral-300 dark:placeholder:text-neutral-700"
      />
    </div>

    {#if patternError}
      <p class="flex items-start gap-1.5 text-[11.5px] text-red-500"
         transition:fade={{ duration: 120 }}>
        <Icon icon="lucide:alert-triangle" width="12" class="shrink-0 mt-0.5" />
        <span class="min-w-0">{patternError}</span>
      </p>
    {/if}

    <!-- Les champs disponibles, cliquables : les retaper à la main revient à
         les deviner, et une faute de frappe ne se voit qu'à l'erreur. -->
    <div class="flex flex-wrap items-center gap-1">
      <span class="mr-0.5 text-[10px] font-semibold uppercase tracking-widest
                   text-neutral-400 dark:text-neutral-500">
        {$t("rename.fields")}
      </span>
      {#each PATTERN_FIELDS as field (field)}
        <button
          type="button"
          onclick={() => insert(field)}
          class="px-1.5 h-6 rounded-md font-mono text-[10.5px] cursor-pointer
                 transition-colors
                 bg-neutral-200/60 dark:bg-white/6
                 text-neutral-500 dark:text-neutral-400
                 hover:bg-emerald-500/15 hover:text-emerald-600 dark:hover:text-emerald-400"
        >
          {field}
        </button>
      {/each}
    </div>

    <div class="flex flex-wrap items-center gap-1">
      <span class="mr-0.5 text-[10px] font-semibold uppercase tracking-widest
                   text-neutral-400 dark:text-neutral-500">
        {$t("rename.presets")}
      </span>
      {#each presets as preset (preset.pattern)}
        <button
          type="button"
          onclick={() => setPattern(preset.pattern)}
          class="px-2 h-6 rounded-md text-[10.5px] cursor-pointer transition-colors
                 {pattern === preset.pattern
                   ? 'bg-emerald-500/15 text-emerald-600 dark:text-emerald-400'
                   : 'bg-neutral-200/60 dark:bg-white/6 text-neutral-500 dark:text-neutral-400 hover:bg-neutral-300/60 dark:hover:bg-white/10'}"
        >
          {preset.label}
        </button>
      {/each}
    </div>

    {#if restructure}
      <!-- Les deux gestes qui accompagnent un déménagement, et qu'on regrette
           de n'avoir pas cochés : les paroles restées derrière, et les dossiers
           vides qui subsistent. -->
      <div class="flex flex-wrap items-center gap-4 pt-0.5">
        <label class="flex items-center gap-2 cursor-pointer">
          <input type="checkbox" bind:checked={satellites} class="checkbox-app" />
          <span class="text-[11.5px] text-neutral-600 dark:text-neutral-300">
            {$t("rename.satellites")}
          </span>
        </label>
        <label class="flex items-center gap-2 cursor-pointer">
          <input type="checkbox" bind:checked={cleanup} class="checkbox-app" />
          <span class="text-[11.5px] text-neutral-600 dark:text-neutral-300">
            {$t("rename.cleanup")}
          </span>
        </label>
      </div>
    {/if}
  </div>

  <!-- ─────────── L'aperçu ─────────── -->
  <div class="flex-1 min-h-0 overflow-y-auto scrollbar-none px-3 py-2.5">
    {#if targets.length === 0}
      <p class="py-16 text-center text-[12.5px] text-neutral-400 dark:text-neutral-500">
        {$t("rename.no_file")}
      </p>
    {:else if !preview}
      <div class="space-y-1" aria-hidden="true">
        {#each Array.from({ length: 6 }) as _, i (i)}
          <div class="h-9 rounded-lg animate-pulse bg-neutral-100/70 dark:bg-white/4"></div>
        {/each}
      </div>
    {:else}
      <!-- Les colonnes nommées une fois : au moment où l'on compare deux noms,
           il faut savoir lequel est l'actuel. -->
      <div class="flex items-center gap-2.5 px-2.5 pb-1.5">
        <!-- La case globale porte l'état intermédiaire : sur cent six lignes
             dont trois décochées, une case simplement vide mentirait. -->
        <input
          type="checkbox"
          checked={allSelected}
          indeterminate={selected.length > 0 && !allSelected}
          disabled={changeable.length === 0}
          onchange={() => selectAll(!allSelected)}
          aria-label={$t("source.select")}
          class="checkbox-app"
        />
        <span class="flex-1 min-w-0 text-[9.5px] font-semibold uppercase tracking-widest
                     text-neutral-400 dark:text-neutral-500">
          {$t("rename.current")}
        </span>
        <span class="shrink-0 w-3"></span>
        <span class="flex-1 min-w-0 text-[9.5px] font-semibold uppercase tracking-widest
                     text-emerald-600/70 dark:text-emerald-400/70">
          {$t("rename.after")}
        </span>
      </div>

      <div class="space-y-0.5">
        {#each preview.moves as move (move.from)}
          {@const on = !move.blocked && !move.unchanged && !excluded.has(move.from)}
          <div
            class="flex items-center gap-2.5 px-2.5 py-1.5 rounded-lg
                   {move.blocked
                     ? 'bg-red-500/8'
                     : move.unchanged
                       ? 'bg-neutral-100/70 dark:bg-white/3 opacity-55'
                       : on
                         ? 'bg-emerald-500/8'
                         : 'bg-neutral-100/70 dark:bg-white/3'}"
          >
            <!-- Case seulement là où il y a un choix : une ligne bloquée ou
                 déjà conforme n'a rien à cocher, et lui donner une case
                 laisserait croire le contraire. -->
            {#if move.blocked || move.unchanged}
              <span class="shrink-0 w-4 flex items-center justify-center">
                <Icon
                  icon={move.blocked ? "lucide:ban" : "lucide:check"}
                  width="11"
                  class={move.blocked
                    ? "text-red-500/70"
                    : "text-neutral-300 dark:text-neutral-600"}
                />
              </span>
            {:else}
              <input
                type="checkbox"
                checked={on}
                onchange={() => toggleRow(move.from)}
                aria-label={move.from_name}
                class="checkbox-app"
              />
            {/if}

            <span class="flex-1 min-w-0 text-[12px] truncate
                         {on || move.blocked
                           ? 'text-neutral-600 dark:text-neutral-300'
                           : 'text-neutral-400 dark:text-neutral-500'}"
                  title={move.from}>
              {move.from_name}
            </span>

            <Icon
              icon={move.blocked
                ? "lucide:x"
                : move.unchanged
                  ? "lucide:equal"
                  : "lucide:arrow-right"}
              width="11"
              class="shrink-0 {move.blocked
                ? 'text-red-500'
                : 'text-neutral-300 dark:text-neutral-600'}"
            />

            <span class="flex-1 min-w-0 text-[12px] truncate" title={move.to}>
              {#if move.blocked}
                <!-- La raison, pas le résultat : un nom cible qu'on ne peut pas
                     écrire n'apprend rien, alors que « deux fichiers visent ce
                     nom » dit quoi changer. -->
                <span class="text-red-500">
                  {move.issues.map((i) => $t(`rename.issue.${i}`)).join(" · ")}
                  {#if move.missing.length > 0}
                    : {move.missing.map((f) => $t(`rename.field.${f}`)).join(", ")}
                  {/if}
                </span>
              {:else if move.unchanged}
                <span class="italic text-neutral-400 dark:text-neutral-500">
                  {$t("rename.unchanged")}
                </span>
              {:else}
                <span class={on
                  ? "font-medium text-emerald-600 dark:text-emerald-400"
                  : "text-neutral-400 dark:text-neutral-500"}>
                  {move.to_name}
                </span>
              {/if}
            </span>
          </div>
        {/each}
      </div>
    {/if}
  </div>

  {#if applyError}
    <div class="shrink-0 flex items-start gap-2 px-5 py-2
                bg-red-500/10 border-t border-red-500/20 text-red-500 text-[11.5px]"
         transition:fade={{ duration: 120 }}>
      <Icon icon="lucide:alert-triangle" width="13" class="shrink-0 mt-0.5" />
      <span class="min-w-0">{applyError}</span>
    </div>
  {/if}

  <!-- ─── Confirmation ───
       Un lot de renommage n'est pas rattrapable à la main : on annonce le
       nombre exact et on laisse le choix. Le bouton par défaut est celui qui
       ne touche à rien. -->
  {#if confirming && preview}
    <div class="shrink-0 flex items-center gap-3 px-4 py-2.5
                bg-amber-500/10 border-t border-amber-500/25"
         transition:fade={{ duration: 120 }}>
      <Icon icon="lucide:triangle-alert" width="14" class="shrink-0 text-amber-500" />
      <span class="flex-1 min-w-0 text-[11.5px] text-amber-700 dark:text-amber-300">
        {selected.length} {$t("rename.confirm")}
      </span>
      <button
        type="button"
        onclick={() => (confirming = false)}
        disabled={applying}
        class="px-3 h-7 rounded-lg text-[12px] cursor-pointer transition-colors
               text-amber-700 dark:text-amber-300 hover:bg-amber-500/15
               disabled:opacity-50"
      >
        {$t("profil.cancel")}
      </button>
      <button
        type="button"
        onclick={apply}
        disabled={applying}
        class="flex items-center gap-1.5 px-3 h-7 rounded-lg text-[12px] font-medium
               cursor-pointer transition-colors
               bg-amber-500 text-white hover:bg-amber-600 disabled:opacity-50"
      >
        {#if applying}
          <Icon icon="lucide:loader-circle" width="12" class="animate-spin" />
        {/if}
        {$t("rename.confirm_yes")}
      </button>
    </div>
  {/if}

  <!-- ─── Compte rendu ───
       Le bouton d'annulation vit ici et nulle part ailleurs : c'est le seul
       moment où l'on sait encore ce qu'on vient de faire. -->
  {#if outcome}
    <div class="shrink-0 flex items-center gap-3 px-4 py-2.5
                border-t border-neutral-200/70 dark:border-white/8
                {outcome.failed.length > 0 ? 'bg-amber-500/10' : 'bg-emerald-500/10'}"
         transition:fade={{ duration: 120 }}>
      <Icon
        icon={outcome.failed.length > 0 ? "lucide:triangle-alert" : "lucide:check-circle-2"}
        width="14"
        class="shrink-0 {outcome.failed.length > 0 ? 'text-amber-500' : 'text-emerald-500'}"
      />
      <span class="flex-1 min-w-0 text-[11.5px] text-neutral-700 dark:text-neutral-200">
        {outcome.succeeded} {$t("rename.renamed")}
        {#if outcome.failed.length > 0}
          · <span class="text-amber-600 dark:text-amber-400">
            {outcome.failed.length} {$t("rename.failed")}
          </span>
        {/if}
      </span>
      <button
        type="button"
        onclick={undo}
        disabled={undoing}
        class="flex items-center gap-1.5 px-3 h-7 rounded-lg text-[12px] font-medium
               cursor-pointer transition-colors
               text-neutral-700 dark:text-neutral-200
               bg-neutral-200/80 dark:bg-white/10
               hover:bg-neutral-300 dark:hover:bg-white/15 disabled:opacity-50"
      >
        {#if undoing}
          <Icon icon="lucide:loader-circle" width="12" class="animate-spin" />
        {:else}
          <Icon icon="lucide:undo-2" width="12" />
        {/if}
        {$t("rename.undo")}
      </button>
    </div>

    {#if outcome.failed.length > 0}
      <div class="shrink-0 max-h-24 overflow-y-auto scrollbar-none px-4 pb-2 space-y-0.5">
        {#each outcome.failed as failure (failure.path)}
          <p class="flex items-baseline gap-2 text-[10.5px]">
            <span class="shrink-0 truncate max-w-[40%] text-neutral-500 dark:text-neutral-400"
                  title={failure.path}>
              {failure.path.split(/[\\/]/).pop()}
            </span>
            <span class="min-w-0 text-amber-600 dark:text-amber-400">{failure.message}</span>
          </p>
        {/each}
      </div>
    {/if}
  {/if}

  <!-- ─────────── Pied de page ─────────── -->
  <footer
    class="shrink-0 flex items-center gap-3 px-4 py-2.5
           border-t border-neutral-200/70 dark:border-white/8
           bg-neutral-50/80 dark:bg-black/25"
  >
    <div class="flex-1 min-w-0 flex items-center gap-3 text-[11.5px]">
      {#if computing}
        <span class="flex items-center gap-1.5 text-neutral-400 dark:text-neutral-500">
          <Icon icon="lucide:loader-circle" width="12" class="animate-spin" />
          {$t("rename.computing")}
        </span>
      {:else if preview}
        <span class="flex items-center gap-1.5 text-neutral-600 dark:text-neutral-300">
          <span class="w-1.5 h-1.5 rounded-full bg-emerald-500"></span>
          <!-- Le dénominateur n'apparaît que si l'on a décoché : sinon les deux
               nombres sont égaux et le second n'apprend rien. -->
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
    </div>

    <button
      type="button"
      onclick={() => popinStore.close()}
      class="flex items-center gap-1.5 px-3 h-8 rounded-lg text-[13px]
             cursor-pointer transition-colors
             text-neutral-600 dark:text-neutral-300
             hover:bg-neutral-200/70 dark:hover:bg-white/8"
    >
      <Icon icon="lucide:x" width="13" />
      {$t("batch.close")}
    </button>

    {#if !outcome}
      <button
        type="button"
        onclick={() => (confirming = true)}
        disabled={selected.length === 0 || applying || confirming}
        class="flex items-center gap-1.5 px-3.5 h-8 rounded-lg text-[13px] font-medium
               cursor-pointer transition-all
               bg-emerald-500 text-white
               hover:bg-emerald-600 hover:shadow-lg hover:shadow-emerald-500/40
               disabled:opacity-35 disabled:cursor-not-allowed disabled:shadow-none"
      >
        <Icon icon="lucide:file-pen-line" width="13" />
        {$t("rename.apply")}
      </button>
    {/if}
  </footer>
</div>
