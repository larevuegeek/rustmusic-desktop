<script lang="ts" module>
  import type { WorkshopField } from "#lib/stores/tags/tagWorkshop.store";

  /** Une colonne : un champ de tag, ou le nom du fichier (lecture seule). */
  export type TableColumn = WorkshopField | "file";

  /** Largeur, minimum (défilement horizontal), alignement à droite des nombres. */
  export const COLUMN_SPECS: Record<TableColumn, { width: string; min: number; numeric?: boolean }> = {
    track_number: { width: "72px", min: 72, numeric: true },
    title: { width: "minmax(200px,1.7fr)", min: 200 },
    artist: { width: "minmax(140px,1fr)", min: 140 },
    album: { width: "minmax(140px,1fr)", min: 140 },
    album_artist: { width: "minmax(140px,1fr)", min: 140 },
    year: { width: "84px", min: 84, numeric: true },
    genre: { width: "minmax(110px,.8fr)", min: 110 },
    disc_number: { width: "80px", min: 80, numeric: true },
    total_tracks: { width: "96px", min: 96, numeric: true },
    total_discs: { width: "96px", min: 96, numeric: true },
    composer: { width: "minmax(170px,1fr)", min: 170 },
    comment: { width: "minmax(200px,1fr)", min: 200 },
    file: { width: "minmax(240px,1.2fr)", min: 240 },
  };

  /** Un vide dans ces champs se signale (« Manquant ») plutôt qu'un simple tiret. */
  export const FLAGGED_WHEN_EMPTY: WorkshopField[] = ["title", "artist", "album", "album_artist", "genre"];
</script>

<script lang="ts">
  // Tableau de l'atelier : sélection comme un explorateur (clic, Maj, Ctrl), édition
  // en place au double-clic ou F2, Entrée/Tab pour passer à la case suivante.
  import Icon from "@iconify/svelte";
  import { tick } from "svelte";
  import { t } from "#lib/i18n";
  import { tagWorkshop, isDirty, valueOf, type WorkshopFile } from "#lib/stores/tags/tagWorkshop.store";
  import { fileName } from "#lib/services/batch/batch.service";

  let {
    columns,
    rows,
    editing = $bindable(),
    onhide,
    onnotice,
  }: {
    columns: TableColumn[];
    /** Lignes affichées (filtre « À corriger » déjà appliqué). */
    rows: WorkshopFile[];
    /** Case en cours d'édition. */
    editing: { path: string; field: WorkshopField } | null;
    onhide: (column: TableColumn) => void;
    /** Message à montrer, avec ou sans « Annuler ». */
    onnotice: (message: string, undoable: boolean) => void;
  } = $props();

  let workshop = $derived($tagWorkshop);
  let selected = $derived(workshop.selected);

  let template = $derived(["48px", "56px", ...columns.map((c) => COLUMN_SPECS[c].width), "32px"].join(" "));
  let minWidth = $derived(48 + 56 + 32 + columns.reduce((sum, c) => sum + COLUMN_SPECS[c].min, 0));

  const label = (column: TableColumn) => (column === "file" ? $t("workshop.file") : $t(`tags.${column}`));
  // Colonnes chiffrées étroites : un intitulé court tient sans être tronqué.
  const SHORT: Partial<Record<TableColumn, string>> = {
    track_number: "workshop.col_track",
    disc_number: "tags.disc_short",
    total_tracks: "workshop.col_tracks",
    total_discs: "workshop.col_discs",
  };
  const heading = (column: TableColumn) => (SHORT[column] ? $t(SHORT[column]!) : label(column));
  const plural = (n: number, one: string, many: string) => `${n} ${n > 1 ? many : one}`;

  // ─── Sélection ───
  let selectable = $derived(rows.filter((f) => f.readable));
  let allSelected = $derived(selectable.length > 0 && selectable.every((f) => selected.has(f.path)));
  let someSelected = $derived(selectable.some((f) => selected.has(f.path)));
  let anchor: string | null = null;

  function toggleAll() {
    const visible = selectable.map((f) => f.path);
    if (allSelected) tagWorkshop.selectAll([...selected].filter((p) => !visible.includes(p)));
    else tagWorkshop.selectAll([...new Set([...selected, ...visible])]);
  }

  function pick(event: MouseEvent, file: WorkshopFile, fromBox = false) {
    if (!file.readable || (event.target as HTMLElement).closest("input")) return;
    if (event.shiftKey && anchor) {
      const paths = rows.map((f) => f.path);
      const [a, b] = [paths.indexOf(anchor), paths.indexOf(file.path)].sort((x, y) => x - y);
      const range = rows.slice(a, b + 1).filter((f) => f.readable).map((f) => f.path);
      tagWorkshop.selectAll([...new Set([...selected, ...range])]);
    } else if (fromBox || event.ctrlKey || event.metaKey) {
      tagWorkshop.toggle(file.path);
    } else {
      tagWorkshop.selectAll([file.path]);
    }
    anchor = file.path;
  }

  // ─── Édition en place ───
  let editor: HTMLInputElement | null = $state(null);
  const editable = $derived(columns.filter((c): c is WorkshopField => c !== "file"));

  async function startEdit(file: WorkshopFile, field: WorkshopField) {
    if (!file.readable) return;
    editing = { path: file.path, field };
    await tick();
    editor?.focus({ preventScroll: true });
    editor?.select();
  }

  // F2 depuis la page : l'éditeur apparaît, on lui donne le focus.
  $effect(() => {
    if (editing && editor && document.activeElement !== editor) {
      editor.focus({ preventScroll: true });
      editor.select();
    }
  });

  function commit(move?: "down" | "up" | "right" | "left") {
    if (!editing || !editor) return;
    const { path, field } = editing;
    const shown = rows;
    let row = shown.findIndex((f) => f.path === path);
    tagWorkshop.set(path, field, editor.value.replace(/\s+/g, " ").trim());
    editing = null;
    if (!move || row < 0) return;

    let col = editable.indexOf(field);
    if (move === "down") row++;
    if (move === "up") row--;
    if (move === "right" && ++col >= editable.length) [col, row] = [0, row + 1];
    if (move === "left" && --col < 0) [col, row] = [editable.length - 1, row - 1];
    // Ligne suivante d'après l'affichage d'avant : elle peut avoir quitté le filtre entre-temps.
    const next = shown[row];
    if (next?.readable && rows.some((f) => f.path === next.path)) startEdit(next, editable[col]);
  }

  function editKeys(event: KeyboardEvent) {
    if (event.key === "Enter") {
      event.preventDefault();
      commit(event.shiftKey ? "up" : "down");
    } else if (event.key === "Tab") {
      event.preventDefault();
      commit(event.shiftKey ? "left" : "right");
    } else if (event.key === "Escape") {
      event.stopPropagation();
      editing = null;
    }
  }

  // ─── En-tête : tri et actions de colonne ───
  let sortField: WorkshopField | null = $state(null);
  let sortAsc = $state(true);
  let menu: TableColumn | null = $state(null);

  function sort(field: WorkshopField) {
    sortAsc = sortField === field ? !sortAsc : true;
    sortField = field;
    tagWorkshop.sortBy(field, sortAsc);
  }

  let targets = $derived(workshop.files.filter((f) => f.readable && selected.has(f.path)));
  const firstValue = (field: WorkshopField) => (targets[0] ? valueOf(targets[0], field) : "");
  const short = (v: string) => (v.length > 18 ? `${v.slice(0, 18)}…` : v);

  function columnAction(field: WorkshopField, action: "fill" | "case" | "clear" | "hide") {
    menu = null;
    const before = workshop.revision;
    const n = targets.length;
    if (action === "hide") return onhide(field);
    if (action === "fill") tagWorkshop.fillDown(field);
    if (action === "case") tagWorkshop.titleCaseOnSelected(field);
    if (action === "clear") tagWorkshop.clearOnSelected(field);
    // Rien n'a changé : on le dit plutôt que de laisser croire à une action.
    if ($tagWorkshop.revision === before) return onnotice($t("workshop.nothing_changed"), false);
    const what = plural(n, $t("workshop.track_one"), $t("workshop.track_many"));
    onnotice(`${label(field)} · ${$t(`workshop.done_${action}`)} · ${what}`, true);
  }

  $effect(() => {
    if (menu === null) return;
    const close = () => (menu = null);
    const timer = setTimeout(() => document.addEventListener("click", close), 0);
    return () => {
      clearTimeout(timer);
      document.removeEventListener("click", close);
    };
  });

  const menuItem = "w-full flex items-center gap-2.5 px-2.5 py-2 rounded-lg text-[13px] text-left normal-case tracking-normal font-medium cursor-pointer transition-colors text-(--rg-tx) hover:bg-(--rg-s2) disabled:opacity-40 disabled:cursor-default disabled:hover:bg-transparent";
</script>

<div class="flex-1 min-w-0 overflow-auto scrollbar-app pb-30" style="--cols: {template}; --minw: {minWidth}px">
  <!-- En-tête collant -->
  <div class="tt-ligne sticky top-0 z-10 h-10 bg-(--rg-champ) border-b border-(--rg-bd)
              text-[11.5px] font-bold tracking-[0.08em] text-(--rg-mu)">
    <button type="button" class="tt-case mx-auto {allSelected ? 'on' : someSelected ? 'on' : ''}"
            aria-label={$t("workshop.select_all")} onclick={toggleAll}>
      <Icon icon={allSelected || !someSelected ? "material-symbols:check-rounded" : "material-symbols:remove-rounded"} width="15" />
    </button>
    <span class="flex justify-center text-(--rg-mu2)"><Icon icon="material-symbols:image-outline-rounded" width="17" /></span>
    {#each columns as column (column)}
      {@const spec = COLUMN_SPECS[column]}
      <div class="group relative h-full min-w-0 flex items-center px-3 {spec.numeric ? 'justify-end' : ''}">
        {#if column === "file"}
          <span class="truncate uppercase">{heading(column)}</span>
        {:else}
          <button type="button" onclick={() => sort(column)}
                  class="min-w-0 flex items-center gap-1 uppercase cursor-pointer transition-colors
                         {sortField === column ? 'text-(--rg-gtx)' : 'hover:text-(--rg-tx)'}">
            <span class="truncate" title={label(column)}>{heading(column)}</span>
            {#if sortField === column}
              <Icon icon={sortAsc ? "material-symbols:arrow-upward-rounded" : "material-symbols:arrow-downward-rounded"} width="13" class="shrink-0" />
            {/if}
          </button>
          <button type="button" onclick={() => (menu = menu === column ? null : column)}
                  title={$t("workshop.column_actions")} aria-label={$t("workshop.column_actions")}
                  class="absolute top-1/2 -translate-y-1/2 {spec.numeric ? 'left-1' : 'right-1'} w-6.5 h-6.5 rounded-md flex items-center justify-center cursor-pointer transition
                         bg-(--rg-champ) text-(--rg-mu2) hover:bg-(--rg-s2) hover:text-(--rg-tx)
                         {menu === column ? 'opacity-100 bg-(--rg-s2) text-(--rg-tx)' : 'opacity-0 group-hover:opacity-100 focus-visible:opacity-100'}">
            <Icon icon="material-symbols:more-vert" width="17" />
          </button>
          {#if menu === column}
            {@const first = firstValue(column)}
            {@const numeric = spec.numeric}
            <div class="tt-menu absolute top-full mt-1 z-20 w-68 p-1.5 rounded-xl {numeric ? 'right-0' : 'left-2'}" role="menu">
              <p class="px-2.5 pt-1.5 pb-1 text-[11px] font-bold tracking-widest uppercase text-(--rg-mu)">{label(column)}</p>
              <button type="button" class={menuItem} disabled={targets.length < 2 || !first} onclick={() => columnAction(column, "fill")}>
                <Icon icon="material-symbols:content-copy-outline-rounded" width="18" class="text-(--rg-mu)" />
                <span class="truncate">{$t("workshop.copy_value").replace("{v}", short(first || "—"))}</span>
              </button>
              <button type="button" class={menuItem} disabled={numeric || targets.length === 0} onclick={() => columnAction(column, "case")}>
                <Icon icon="material-symbols:text-fields-rounded" width="18" class="text-(--rg-mu)" />{$t("workshop.title_case")}
              </button>
              <button type="button" class={menuItem} disabled={targets.length === 0} onclick={() => columnAction(column, "clear")}>
                <Icon icon="material-symbols:backspace-outline-rounded" width="18" class="text-(--rg-mu)" />{$t("workshop.clear_n").replace("{n}", String(targets.length))}
              </button>
              <hr class="my-1 border-(--rg-bd)" />
              <button type="button" class={menuItem} disabled={column === "title"} onclick={() => columnAction(column, "hide")}>
                <Icon icon="material-symbols:visibility-off-outline-rounded" width="18" class="text-(--rg-mu)" />{$t("workshop.hide_column")}
              </button>
            </div>
          {/if}
        {/if}
      </div>
    {/each}
    <span></span>
  </div>

  {#each rows as file (file.path)}
    {@const on = selected.has(file.path)}
    {@const shown = file.pendingCover?.src ?? file.cover}
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div role="presentation" class="tt-ligne tt-rangee h-13 {on ? 'on' : ''} {file.readable ? '' : 'opacity-45'}"
         onclick={(e) => pick(e, file)}>
      <button type="button" class="tt-case mx-auto {on ? 'on' : ''}" disabled={!file.readable}
              aria-label={fileName(file.path)} aria-pressed={on}
              onclick={(e) => { e.stopPropagation(); pick(e, file, true); }}>
        <Icon icon="material-symbols:check-rounded" width="15" />
      </button>
      <span class="relative w-9 h-9 mx-auto rounded-[7px] overflow-hidden bg-(--rg-s2) shadow-[0_3px_10px_rgba(0,0,0,0.3)]
                   {file.pendingCover ? 'ring-2 ring-(--rg-g)' : ''}"
            title={file.pendingCover ? $t("workshop.cover_pending") : undefined}>
        {#if shown}
          <img src={shown} alt="" loading="lazy" class="w-full h-full object-cover" />
        {:else}
          <Icon icon="material-symbols:hide-image-outline-rounded" width="16" class="absolute inset-0 m-auto text-(--rg-mu2)" />
        {/if}
      </span>
      {#each columns as column (column)}
        {@const spec = COLUMN_SPECS[column]}
        {#if column === "file"}
          <div class="h-full min-w-0 flex items-center gap-1.5 px-3 font-mono text-[12.5px] text-(--rg-mu)" title={file.path}>
            {#if !file.readable}<Icon icon="material-symbols:error-outline-rounded" width="15" class="shrink-0 text-red-500" />{/if}
            <span class="truncate">{fileName(file.path)}</span>
          </div>
        {:else if editing?.path === file.path && editing.field === column}
          <!-- L'éditeur épouse la case : même texte au même endroit, pas de saut. -->
          <div class="h-full min-w-0 flex items-center px-1">
            <input bind:this={editor} value={valueOf(file, column)} data-focus-ring="none"
                   onkeydown={editKeys} onblur={() => commit()}
                   class="tt-editeur w-full h-9 px-2 rounded-lg outline-none text-[14px] {spec.numeric ? 'text-right tabular-nums' : ''}" />
          </div>
        {:else}
          {@const value = valueOf(file, column)}
          {@const dirty = isDirty(file, column)}
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div class="tt-cellule h-full min-w-0 flex items-center px-3 text-[14px]
                      {spec.numeric ? 'justify-end tabular-nums text-(--rg-tx2)' : 'text-(--rg-tx)'}
                      {dirty ? 'modifiee' : ''}"
               title={dirty ? `${$t("workshop.before")} : ${file.original[column] || $t("workshop.was_empty")}` : value || undefined}
               ondblclick={() => startEdit(file, column)}>
            {#if value}
              <span class="truncate">{value}</span>
            {:else if FLAGGED_WHEN_EMPTY.includes(column) && file.readable}
              <!-- Discret : sur toute une colonne, une pastille par ligne criait. -->
              <span class="flex items-center gap-1.5 text-[12.5px] font-semibold text-(--rg-am)">
                <span class="w-1.5 h-1.5 rounded-full bg-(--rg-am)"></span>{$t("workshop.missing")}
              </span>
            {:else}
              <span class="text-(--rg-mu2)">—</span>
            {/if}
          </div>
        {/if}
      {/each}
      <span class="flex justify-center">
        {#if file.readable && (Object.keys(file.edits).length > 0 || file.pendingCover)}
          <i class="w-1.75 h-1.75 rounded-full bg-(--rg-g)" title={$t("workshop.modified")}></i>
        {/if}
      </span>
    </div>
  {:else}
    <p class="px-6 py-16 text-center text-[14px] text-(--rg-mu)">{$t("workshop.all_clear")}</p>
  {/each}
</div>

<style>
  .tt-ligne {
    display: grid;
    grid-template-columns: var(--cols);
    align-items: center;
    min-width: var(--minw);
  }
  .tt-rangee {
    position: relative;
    cursor: default;
    user-select: none;
  }
  .tt-rangee + .tt-rangee {
    box-shadow: inset 0 1px 0 var(--rg-line);
  }
  .tt-rangee:hover {
    background: var(--rg-hover);
  }
  .tt-rangee.on {
    background: color-mix(in oklch, var(--rg-g) 7%, var(--rg-champ));
  }
  .tt-rangee.on:hover {
    background: color-mix(in oklch, var(--rg-g) 10%, var(--rg-champ));
  }
  .tt-cellule:hover {
    box-shadow: inset 0 0 0 1px var(--rg-bd2);
    border-radius: 8px;
    cursor: text;
  }
  .tt-cellule.modifiee {
    color: var(--rg-gtx);
    background: color-mix(in oklch, var(--rg-g) 8%, transparent);
    font-weight: 600;
  }
  .tt-case {
    width: 20px;
    height: 20px;
    border-radius: 6px;
    border: 2px solid var(--rg-bd2);
    display: flex;
    align-items: center;
    justify-content: center;
    color: transparent;
    cursor: pointer;
  }
  .tt-case:disabled {
    cursor: default;
  }
  .tt-case.on {
    background: var(--rg-g);
    border-color: var(--rg-g);
    color: var(--rg-on-g);
  }
  .tt-editeur {
    background: color-mix(in oklch, var(--rg-tx) 7%, var(--rg-champ));
    color: var(--rg-tx);
    border: 1.5px solid var(--rg-g);
    box-shadow: 0 4px 14px rgb(0 0 0 / 0.25);
  }
  :global(html:not(.dark)) .tt-editeur {
    background: var(--rg-carte);
  }
  .tt-menu {
    background: var(--rg-carte);
    border: 1px solid var(--rg-bd2);
    box-shadow: 0 18px 40px rgb(0 0 0 / 0.45);
  }
</style>
