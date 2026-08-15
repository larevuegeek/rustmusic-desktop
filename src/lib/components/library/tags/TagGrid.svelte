<script lang="ts">
  // Vue tableur de l'atelier : une ligne par fichier, une colonne par tag.
  //
  // # Pourquoi un tableau et pas un formulaire
  // Un formulaire applique une valeur à tous et ne montre rien. Ici on voit les
  // vraies valeurs côte à côte, donc on repère d'un coup d'œil que trois pistes
  // n'ont pas d'année et qu'une seule a l'artiste mal orthographié — et on
  // corrige ces cases-là, pas les douze.
  //
  // # La densité est le sujet
  // Pas d'intitulé par case : l'en-tête de colonne le porte. Lignes courtes,
  // saisie sans habillage tant qu'on ne la touche pas. L'objectif est de tenir
  // une trentaine de fichiers à l'écran, sinon autant garder un formulaire.
  import Icon from "@iconify/svelte";
  import { t } from "$lib/i18n";
  import {
    tagWorkshop,
    isDirty,
    isIncomplete,
    valueOf,
    type WorkshopField,
    type WorkshopFile,
  } from "$lib/stores/tags/tagWorkshop.store";
  import { fileName } from "$lib/services/batch/batch.service";

  const {
    columns,
    onlyIncomplete = false,
  }: { columns: WorkshopField[]; onlyIncomplete?: boolean } = $props();

  let workshop = $derived($tagWorkshop);

  // Filtrer l'affichage seulement : les lignes masquées gardent leurs
  // modifications et partent à l'écriture comme les autres.
  let rows = $derived(
    onlyIncomplete ? workshop.files.filter((f) => isIncomplete(f) || !f.readable) : workshop.files,
  );

  /** Colonnes courtes et alignées au centre : numéros et années. */
  const NARROW: WorkshopField[] = [
    "track_number",
    "disc_number",
    "total_tracks",
    "total_discs",
    "year",
  ];

  const isNarrow = (field: WorkshopField) => NARROW.includes(field);

  let selectable = $derived(rows.filter((f) => f.readable));
  let allSelected = $derived(
    selectable.length > 0 && selectable.every((f) => workshop.selected.has(f.path)),
  );

  function toggleAll() {
    if (allSelected) tagWorkshop.selectNone();
    else tagWorkshop.selectAll(selectable.map((f) => f.path));
  }

  /**
   * Enregistre à la sortie du champ, pas à chaque frappe.
   *
   * Sur cinq cents lignes, reconstruire l'état à chaque caractère ferait ramer
   * la saisie — et une valeur à moitié tapée n'a de toute façon pas de sens.
   */
  function commit(file: WorkshopFile, field: string, event: Event) {
    tagWorkshop.set(file.path, field, (event.currentTarget as HTMLInputElement).value);
  }

  // ─── En-tête de colonne ───

  let sortField: WorkshopField | null = $state(null);
  let sortAsc = $state(true);
  let openMenu: WorkshopField | null = $state(null);

  function sort(field: WorkshopField) {
    sortAsc = sortField === field ? !sortAsc : true;
    sortField = field;
    tagWorkshop.sortBy(field, sortAsc);
  }

  function runColumnAction(field: WorkshopField, action: "fill" | "clear") {
    openMenu = null;
    if (action === "fill") tagWorkshop.fillDown(field);
    else tagWorkshop.clearOnSelected(field);
  }

  // Le menu se ferme au clic ailleurs : sans ça il resterait ouvert par-dessus
  // les lignes qu'on essaie de corriger.
  $effect(() => {
    if (openMenu === null) return;
    const close = () => (openMenu = null);
    const timer = setTimeout(() => document.addEventListener("click", close), 0);
    return () => {
      clearTimeout(timer);
      document.removeEventListener("click", close);
    };
  });
</script>

<div class="flex-1 min-h-0 overflow-auto scrollbar-none">
  <!-- `w-max` plutôt que `w-full` : les colonnes prennent la place qu'il leur
       faut et le conteneur défile horizontalement. Comprimer pour tout faire
       tenir tronquait « Michael Kamen, Eric Cla| » sur toutes les lignes —
       on ne peut pas vérifier ce qu'on ne lit pas. -->
  <table class="min-w-full w-max border-collapse text-[12px]">
    <thead class="sticky top-0 z-10">
      <tr class="bg-neutral-100 dark:bg-neutral-900">
        <th class="w-8 px-2 py-1.5 border-b border-neutral-200 dark:border-white/10">
          <input
            type="checkbox"
            checked={allSelected}
            onchange={toggleAll}
            aria-label={$t('workshop.select_all')}
            class="checkbox-app"
          />
        </th>

        <th class="w-10 px-1 py-1.5 border-b border-neutral-200 dark:border-white/10">
          <Icon icon="lucide:image" width="11" class="mx-auto text-neutral-400 dark:text-neutral-500" />
        </th>

        {#each columns as field (field)}
          <th
            class="px-1 py-1.5 text-left font-semibold border-b
                   border-neutral-200 dark:border-white/10
                   {isNarrow(field) ? 'w-20' : 'min-w-56'}"
          >
            <div class="relative flex items-center gap-1 px-1.5">
              <!-- Le clic sur l'intitulé trie : c'est le geste attendu d'un
                   tableau, et c'est ce que la flèche laissait croire à tort. -->
              <button
                type="button"
                onclick={() => sort(field)}
                class="flex-1 min-w-0 flex items-center gap-1 cursor-pointer
                       text-[10px] uppercase tracking-[0.08em] text-left
                       transition-colors
                       {sortField === field
                         ? 'text-emerald-600 dark:text-emerald-400'
                         : 'text-neutral-500 dark:text-neutral-400 hover:text-neutral-800 dark:hover:text-neutral-200'}"
              >
                <span class="truncate">{$t(`tags.${field}`)}</span>
                {#if sortField === field}
                  <Icon
                    icon={sortAsc ? 'lucide:chevron-up' : 'lucide:chevron-down'}
                    width="11"
                    class="shrink-0"
                  />
                {/if}
              </button>

              <!-- Les actions de masse sont **nommées**, jamais devinées : une
                   flèche seule s'était fait prendre pour un tri, et a recopié
                   une valeur sur cent lignes. -->
              <button
                type="button"
                onclick={() => (openMenu = openMenu === field ? null : field)}
                title={$t('workshop.column_actions')}
                aria-label={$t('workshop.column_actions')}
                class="shrink-0 w-5 h-5 rounded flex items-center justify-center
                       cursor-pointer transition-colors
                       text-neutral-300 dark:text-neutral-600
                       hover:text-neutral-700 dark:hover:text-neutral-200
                       hover:bg-neutral-200/70 dark:hover:bg-white/10"
              >
                <Icon icon="lucide:ellipsis-vertical" width="12" />
              </button>

              {#if openMenu === field}
                <div
                  class="absolute right-0 top-full mt-1 z-20 w-52 py-1 rounded-lg
                         bg-white dark:bg-neutral-900
                         ring-1 ring-black/5 dark:ring-white/10
                         shadow-xl shadow-black/20"
                >
                  <button
                    type="button"
                    onclick={() => runColumnAction(field, 'fill')}
                    class="w-full flex items-center gap-2 px-3 py-1.5 text-left text-[12px]
                           cursor-pointer normal-case tracking-normal
                           text-neutral-700 dark:text-neutral-200
                           hover:bg-neutral-100 dark:hover:bg-white/8"
                  >
                    <Icon icon="lucide:arrow-down-to-line" width="12" class="opacity-60" />
                    {$t('workshop.fill_down')}
                  </button>
                  <button
                    type="button"
                    onclick={() => runColumnAction(field, 'clear')}
                    class="w-full flex items-center gap-2 px-3 py-1.5 text-left text-[12px]
                           cursor-pointer normal-case tracking-normal
                           text-red-500 hover:bg-red-500/10"
                  >
                    <Icon icon="lucide:eraser" width="12" class="opacity-60" />
                    {$t('workshop.clear_column')}
                  </button>
                </div>
              {/if}
            </div>
          </th>
        {/each}

        <th class="px-2 py-1.5 text-left text-[10px] uppercase tracking-[0.08em] font-semibold
                   border-b border-neutral-200 dark:border-white/10
                   text-neutral-500 dark:text-neutral-400">
          {$t('workshop.file')}
        </th>
      </tr>
    </thead>

    <tbody>
      {#each rows as file (file.path)}
        {@const checked = workshop.selected.has(file.path)}
        {@const shown = file.pendingCover?.src ?? file.cover}
        <tr
          class="border-b border-neutral-100 dark:border-white/5 transition-colors
                 {checked ? '' : 'opacity-45'}
                 {file.readable ? 'hover:bg-neutral-50 dark:hover:bg-white/3' : 'bg-red-500/5'}"
        >
          <td class="px-2 py-1 align-top">
            <input
              type="checkbox"
              {checked}
              disabled={!file.readable}
              onchange={() => tagWorkshop.toggle(file.path)}
              aria-label={fileName(file.path)}
              class="checkbox-app"
            />
          </td>

          <td class="px-1 py-1 align-top">
            <div
              class="relative w-7 h-7 mx-auto rounded overflow-hidden ring-1
                     {file.pendingCover
                       ? 'ring-emerald-500'
                       : 'ring-black/10 dark:ring-white/10'}"
              title={file.pendingCover ? $t('workshop.cover_pending') : undefined}
            >
              {#if shown}
                <img src={shown} alt="" class="w-full h-full object-cover" />
              {:else}
                <!-- L'absence de pochette est une information : c'est souvent
                     ce qu'on est venu corriger. -->
                <div class="w-full h-full flex items-center justify-center
                            bg-neutral-100 dark:bg-white/5">
                  <Icon icon="lucide:image-off" width="11"
                        class="text-neutral-300 dark:text-neutral-600" />
                </div>
              {/if}
            </div>
          </td>

          {#each columns as field (field)}
            {@const dirty = isDirty(file, field)}
            <td class="px-0 py-1 align-top">
              <input
                type="text"
                data-focus-ring="none"
                value={valueOf(file, field)}
                disabled={!file.readable}
                onchange={(e) => commit(file, field, e)}
                class="w-full px-2 py-1 bg-transparent outline-none rounded
                       transition-colors
                       focus:bg-emerald-500/10
                       disabled:opacity-40
                       {isNarrow(field) ? 'tabular-nums text-center' : ''}
                       {dirty
                         ? 'text-emerald-600 dark:text-emerald-400 font-medium'
                         : 'text-neutral-800 dark:text-neutral-100'}"
              />

              <!-- L'ancienne valeur sous la nouvelle : c'est ce qui rend une
                   correction de masse relisible avant écriture. Sans elle, on
                   ne peut plus distinguer ce qu'on a changé de ce qui était
                   déjà là — ni constater qu'on a écrasé la mauvaise colonne. -->
              {#if dirty}
                <button
                  type="button"
                  onclick={() => tagWorkshop.revertField(file.path, field)}
                  title={$t('tags.revert')}
                  class="w-full flex items-center gap-1 px-2 pt-0.5 cursor-pointer
                         text-[10px] leading-tight text-left
                         text-neutral-400 dark:text-neutral-500
                         hover:text-neutral-700 dark:hover:text-neutral-200
                         {isNarrow(field) ? 'justify-center' : ''}"
                >
                  <Icon icon="lucide:rotate-ccw" width="9" class="shrink-0 opacity-70" />
                  <span class="truncate line-through decoration-neutral-400/60">
                    {file.original[field] || $t('workshop.was_empty')}
                  </span>
                </button>
              {/if}
            </td>
          {/each}

          <td class="px-2 py-1 align-top">
            <span
              class="flex items-center gap-1 text-[11px] whitespace-nowrap
                     text-neutral-400 dark:text-neutral-500"
              title={file.path}
            >
              {#if !file.readable}
                <Icon icon="lucide:file-x" width="11" class="shrink-0 text-red-500" />
              {:else if isIncomplete(file)}
                <!-- Un champ essentiel manque : c'est ce qu'on vient corriger. -->
                <Icon icon="lucide:triangle-alert" width="11" class="shrink-0 text-amber-500" />
              {/if}
              <span>{fileName(file.path)}</span>
            </span>
          </td>
        </tr>
      {/each}
    </tbody>
  </table>

  {#if rows.length === 0 && !workshop.loading}
    <p class="px-5 py-10 text-center text-[13px] text-neutral-400 dark:text-neutral-500">
      {$t('workshop.empty')}
    </p>
  {/if}
</div>
