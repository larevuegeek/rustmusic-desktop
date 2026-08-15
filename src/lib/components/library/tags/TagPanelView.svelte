<script lang="ts">
  // Vue « liste + panneau » de l'atelier.
  //
  // # À quel moment elle sert
  // Le tableur montre tout et sert à repérer. Celle-ci sert à corriger en
  // profondeur : champs longs (commentaire, compositeur) confortables à saisir,
  // et application d'une même valeur à toute la sélection.
  //
  // # Le même état que le tableur
  // Elle écrit dans le même magasin, fichier par fichier. Poser une valeur sur
  // la sélection n'est qu'une écriture répétée — d'où la bascule sans perte :
  // ce qu'on corrige ici apparaît là-bas, et l'inverse.
  import Icon from "@iconify/svelte";
  import { t } from "$lib/i18n";
  import TagField from "$lib/components/ui/input/TagField.svelte";
  import {
    tagWorkshop,
    isDirty,
    isIncomplete,
    fileIsDirty,
    valueOf,
    WORKSHOP_FIELDS,
    type WorkshopField,
  } from "$lib/stores/tags/tagWorkshop.store";
  import { fileName } from "$lib/services/batch/batch.service";

  let workshop = $derived($tagWorkshop);
  let targets = $derived(
    workshop.files.filter((f) => f.readable && workshop.selected.has(f.path)),
  );

  /**
   * Valeur commune aux fichiers visés, ou `null` s'ils divergent.
   *
   * Même règle que l'éditeur de sélection : afficher la valeur du premier
   * laisserait croire qu'elle vaut pour tous.
   */
  function commonValue(field: WorkshopField): string | null {
    if (targets.length === 0) return "";
    const first = valueOf(targets[0], field);
    return targets.every((f) => valueOf(f, field) === first) ? first : null;
  }

  /** Au moins un des fichiers visés porte une modification sur ce champ. */
  function fieldDirty(field: WorkshopField): boolean {
    return targets.some((f) => isDirty(f, field));
  }

  function apply(field: WorkshopField, value: string) {
    tagWorkshop.setOnSelected(field, value);
  }

  function revert(field: WorkshopField) {
    for (const file of targets) tagWorkshop.revertField(file.path, field);
  }

  /**
   * Champs édités par cette vue.
   *
   * Le numéro de piste en est écarté : appliquer la **même** valeur à toute la
   * sélection numéroterait tout à l'identique. Il se règle dans le tableur,
   * ligne par ligne, ou par le bouton de numérotation automatique.
   */
  const PANEL_FIELDS: WorkshopField[] = WORKSHOP_FIELDS.filter(
    (field) => field !== "track_number",
  );

  const LONG_FIELDS: WorkshopField[] = ["comment"];
</script>

<div class="flex-1 min-h-0 flex">
  <!-- ─────────── La sélection ─────────── -->
  <aside
    class="shrink-0 w-72 flex flex-col overflow-y-auto scrollbar-none
           border-r border-neutral-200/70 dark:border-white/8
           bg-neutral-50/70 dark:bg-black/20"
  >
    <div class="sticky top-0 flex items-center gap-2 px-3 py-2
                bg-neutral-50/95 dark:bg-neutral-950/95
                border-b border-neutral-200/70 dark:border-white/8">
      <span class="flex-1 text-[10px] font-semibold uppercase tracking-[0.09em]
                   text-neutral-400 dark:text-neutral-500">
        {$t('workshop.file')}
        <span class="ml-1 px-1 rounded bg-neutral-200/70 dark:bg-white/10 tabular-nums">
          {targets.length}/{workshop.files.length}
        </span>
      </span>
      <button
        type="button"
        onclick={() => tagWorkshop.selectAll()}
        class="text-[11px] cursor-pointer transition-colors
               text-neutral-400 dark:text-neutral-500
               hover:text-neutral-700 dark:hover:text-neutral-200"
      >
        {$t('workshop.select_all')}
      </button>
    </div>

    <div class="p-1.5 space-y-0.5">
      {#each workshop.files as file (file.path)}
        {@const checked = workshop.selected.has(file.path)}
        <button
          type="button"
          onclick={() => tagWorkshop.toggle(file.path)}
          disabled={!file.readable}
          class="w-full flex items-center gap-2 px-2 py-1.5 rounded-lg text-left
                 cursor-pointer transition-colors disabled:opacity-40
                 {checked
                   ? 'bg-emerald-500/10'
                   : 'hover:bg-neutral-200/60 dark:hover:bg-white/5'}"
          title={file.path}
        >
          <span
            class="shrink-0 w-3.5 h-3.5 rounded flex items-center justify-center
                   ring-1 transition-colors
                   {checked
                     ? 'bg-emerald-500 ring-emerald-500 text-white'
                     : 'ring-neutral-300 dark:ring-white/20'}"
          >
            {#if checked}
              <Icon icon="lucide:check" width="9" />
            {/if}
          </span>

          <span
            class="shrink-0 w-8 h-8 rounded overflow-hidden ring-1
                   {file.pendingCover ? 'ring-emerald-500' : 'ring-black/10 dark:ring-white/10'}"
          >
            {#if file.pendingCover?.src ?? file.cover}
              <img
                src={file.pendingCover?.src ?? file.cover}
                alt=""
                class="w-full h-full object-cover"
              />
            {:else}
              <span class="w-full h-full flex items-center justify-center
                           bg-neutral-100 dark:bg-white/5">
                <Icon icon="lucide:image-off" width="11"
                      class="text-neutral-300 dark:text-neutral-600" />
              </span>
            {/if}
          </span>

          <span class="min-w-0 flex-1">
            <span
              class="block text-[12px] truncate
                     {fileIsDirty(file)
                       ? 'text-emerald-600 dark:text-emerald-400 font-medium'
                       : 'text-neutral-700 dark:text-neutral-300'}"
            >
              {valueOf(file, 'title') || fileName(file.path)}
            </span>
            <span class="block text-[10px] truncate text-neutral-400 dark:text-neutral-500">
              {valueOf(file, 'artist') || fileName(file.path)}
            </span>
          </span>

          {#if !file.readable}
            <Icon icon="lucide:file-x" width="11" class="shrink-0 text-red-500" />
          {:else if isIncomplete(file)}
            <Icon icon="lucide:triangle-alert" width="11" class="shrink-0 text-amber-500" />
          {/if}
        </button>
      {/each}
    </div>
  </aside>

  <!-- ─────────── Les champs ─────────── -->
  <div class="flex-1 min-w-0 overflow-y-auto scrollbar-none px-5 py-4">
    {#if targets.length === 0}
      <p class="py-10 text-center text-[13px] text-neutral-400 dark:text-neutral-500">
        {$t('workshop.select_something')}
      </p>
    {:else}
      <div class="max-w-2xl space-y-1.5">
        {#each PANEL_FIELDS as field (field)}
          {@const common = commonValue(field)}
          <TagField
            label={$t(`tags.${field}`)}
            value={common ?? ''}
            placeholder={common === null ? $t('tags.multiple_values') : '—'}
            dirty={fieldDirty(field)}
            multiline={LONG_FIELDS.includes(field)}
            revertLabel={$t('tags.revert')}
            oninput={(e: Event) =>
              apply(field, (e.currentTarget as HTMLInputElement).value)}
            onrevert={() => revert(field)}
          />
        {/each}
      </div>
    {/if}
  </div>
</div>
