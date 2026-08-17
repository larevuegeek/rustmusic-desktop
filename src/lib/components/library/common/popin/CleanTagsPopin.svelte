<script lang="ts">
  // Règles de nettoyage.
  //
  // # Rien ne s'applique d'office
  // Ce qui est du bruit chez l'un est une intention chez l'autre : un souligné
  // dans un titre de musique industrielle peut être voulu, et la mise en
  // capitales anglaise abîme un catalogue francophone. Les cases partent donc
  // décochées, et l'aperçu montre l'effet avant qu'on décide.
  //
  // # Le résultat va dans les modifications en attente
  // Pas dans les fichiers. On le relit dans le tableur, on écrit quand on veut,
  // et le journal le rattrape si l'on s'est trompé.
  import Icon from "@iconify/svelte";
  import { fade } from "svelte/transition";
  import { popinStore } from "$lib/stores/ui/popin.store";
  import { t } from "$lib/i18n";
  import {
    tagWorkshop,
    WORKSHOP_FIELDS,
    type WorkshopField,
  } from "$lib/stores/tags/tagWorkshop.store";
  import { cleanTags, CLEAN_RULES, type CleanRule } from "$lib/services/tags/rename.service";

  /* eslint-disable svelte/valid-prop-names-in-kit-pages */
  const {} = $props();

  /** Les champs de texte : nettoyer un numéro de piste n'a pas de sens. */
  const TEXT_FIELDS: WorkshopField[] = WORKSHOP_FIELDS.filter(
    (f) => !f.includes("number") && !f.includes("total") && f !== "year",
  ) as WorkshopField[];

  let workshop = $derived($tagWorkshop);
  let targets = $derived(
    workshop.selected.size > 0
      ? workshop.files.filter((f) => f.readable && workshop.selected.has(f.path))
      : workshop.files.filter((f) => f.readable),
  );

  let rules: CleanRule[] = $state([]);
  /** Une proposition par cellule : chemin, champ, avant, après. */
  let proposals: { path: string; field: string; from: string; to: string }[] = $state([]);
  let computing = $state(false);
  let error: string | null = $state(null);

  /** Clé unique d'une cellule — le chemin seul ne suffit pas. */
  const cellKey = (path: string, field: string) => `${path}\u0000${field}`;

  $effect(() => {
    const selected = rules;
    const files = targets;
    run(selected, files);
  });

  async function run(selected: CleanRule[], files: typeof targets) {
    if (selected.length === 0 || files.length === 0) {
      proposals = [];
      return;
    }
    computing = true;
    error = null;
    try {
      const values: [string, string][] = [];
      for (const file of files) {
        for (const field of TEXT_FIELDS) {
          const value = file.edits[field] ?? file.original[field] ?? "";
          if (value) values.push([cellKey(file.path, field), value]);
        }
      }
      const changed = await cleanTags(values, selected);
      const before = new Map(values);
      proposals = changed.map(([key, to]) => {
        const [path, field] = key.split("\u0000");
        return { path, field, from: before.get(key) ?? "", to };
      });
    } catch (e) {
      error = String((e as any)?.message ?? e ?? "");
    } finally {
      computing = false;
    }
  }

  function toggle(rule: CleanRule) {
    rules = rules.includes(rule) ? rules.filter((r) => r !== rule) : [...rules, rule];
  }

  function apply() {
    for (const p of proposals) {
      tagWorkshop.set(p.path, p.field as WorkshopField, p.to);
    }
    popinStore.close();
  }

  const fileName = (path: string) => path.split(/[\\/]/).pop() ?? path;
</script>

<div class="flex-1 min-h-0 flex flex-col">
  <div class="shrink-0 px-4 py-3 space-y-2
              border-b border-neutral-200/70 dark:border-white/8
              bg-neutral-50/70 dark:bg-black/20">
    <div class="flex flex-wrap gap-1.5">
      {#each CLEAN_RULES as rule (rule)}
        <button
          type="button"
          onclick={() => toggle(rule)}
          class="px-2.5 h-7 rounded-lg text-[11.5px] cursor-pointer transition-colors
                 {rules.includes(rule)
                   ? 'bg-emerald-500/15 text-emerald-600 dark:text-emerald-400 font-medium'
                   : 'bg-neutral-200/60 dark:bg-white/6 text-neutral-500 dark:text-neutral-400 hover:bg-neutral-300/60 dark:hover:bg-white/10'}"
        >
          {$t(`clean.rule.${rule}`)}
        </button>
      {/each}
    </div>
    <p class="text-[10.5px] leading-relaxed text-neutral-400 dark:text-neutral-500">
      {$t("clean.hint")}
    </p>
  </div>

  <div class="flex-1 min-h-0 overflow-y-auto scrollbar-none px-3 py-2.5">
    {#if error}
      <p class="flex items-start gap-2 px-3 py-2.5 rounded-lg text-[12px]
                bg-red-500/10 text-red-500" transition:fade={{ duration: 120 }}>
        <Icon icon="lucide:alert-triangle" width="13" class="shrink-0 mt-0.5" />
        <span class="min-w-0">{error}</span>
      </p>
    {:else if rules.length === 0}
      <p class="py-16 text-center text-[12.5px] text-neutral-400 dark:text-neutral-500">
        {$t("clean.pick_rule")}
      </p>
    {:else if proposals.length === 0}
      <p class="py-16 text-center text-[12.5px] text-neutral-400 dark:text-neutral-500">
        {computing ? $t("rename.computing") : $t("clean.nothing")}
      </p>
    {:else}
      <div class="space-y-0.5">
        {#each proposals as p (p.path + p.field)}
          <div class="flex items-center gap-2.5 px-2.5 py-1.5 rounded-lg bg-emerald-500/8">
            <span class="shrink-0 w-32 text-[10.5px] truncate
                         text-neutral-400 dark:text-neutral-500"
                  title={fileName(p.path)}>
              {fileName(p.path)}
            </span>
            <span class="shrink-0 w-20 text-[10.5px] truncate
                         text-neutral-500 dark:text-neutral-400">
              {$t(`tags.${p.field}`)}
            </span>
            <span class="flex-1 min-w-0 text-[12px] truncate line-through
                         text-neutral-400 dark:text-neutral-500" title={p.from}>
              {p.from}
            </span>
            <Icon icon="lucide:arrow-right" width="10"
                  class="shrink-0 text-neutral-300 dark:text-neutral-600" />
            <span class="flex-1 min-w-0 text-[12px] truncate font-medium
                         text-emerald-600 dark:text-emerald-400" title={p.to}>
              {p.to}
            </span>
          </div>
        {/each}
      </div>
    {/if}
  </div>

  <footer
    class="shrink-0 flex items-center gap-3 px-4 py-2.5
           border-t border-neutral-200/70 dark:border-white/8
           bg-neutral-50/80 dark:bg-black/25"
  >
    <span class="flex-1 min-w-0 text-[11.5px] text-neutral-500 dark:text-neutral-400">
      {#if proposals.length > 0}
        {proposals.length} {$t("clean.changes")}
      {/if}
    </span>
    <button
      type="button"
      onclick={() => popinStore.close()}
      class="flex items-center gap-1.5 px-3 h-8 rounded-lg text-[13px]
             cursor-pointer transition-colors
             text-neutral-600 dark:text-neutral-300
             hover:bg-neutral-200/70 dark:hover:bg-white/8"
    >
      <Icon icon="lucide:x" width="13" />
      {$t("profil.cancel")}
    </button>
    <button
      type="button"
      onclick={apply}
      disabled={proposals.length === 0}
      class="flex items-center gap-1.5 px-3.5 h-8 rounded-lg text-[13px] font-medium
             cursor-pointer transition-all
             bg-emerald-500 text-white
             hover:bg-emerald-600 hover:shadow-lg hover:shadow-emerald-500/40
             disabled:opacity-35 disabled:cursor-not-allowed disabled:shadow-none"
    >
      <Icon icon="lucide:check" width="13" />
      {$t("source.apply")}
    </button>
  </footer>
</div>
