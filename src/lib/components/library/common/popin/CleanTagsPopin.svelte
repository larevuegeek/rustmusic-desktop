<script lang="ts">
  import { messageErreur } from "#lib/helper/tools/errorTools";
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
  import { popinStore } from "#lib/stores/ui/popin.store";
  import { t } from "#lib/i18n";
  import {
    tagWorkshop,
    WORKSHOP_FIELDS,
    type WorkshopField,
  } from "#lib/stores/tags/tagWorkshop.store";
  import { cleanTags, CLEAN_RULES, type CleanRule } from "#lib/services/tags/rename.service";
  import PopinFooter from "#lib/components/ui/popin/PopinFooter.svelte";

  /* eslint-disable svelte/valid-prop-names-in-kit-pages */
  /** Appelé après application, avec le nombre de valeurs corrigées. */
  const { onapplied = () => {} }: { onapplied?: (count: number) => void } = $props();

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
      error = messageErreur(e);
    } finally {
      computing = false;
    }
  }

  function toggle(rule: CleanRule) {
    rules = rules.includes(rule) ? rules.filter((r) => r !== rule) : [...rules, rule];
  }

  function apply() {
    // Une seule étape d'annulation pour tout le nettoyage.
    tagWorkshop.group(() => {
      for (const p of proposals) tagWorkshop.set(p.path, p.field as WorkshopField, p.to);
    });
    onapplied(proposals.length);
    popinStore.close();
  }

  const fileName = (path: string) => path.split(/[\\/]/).pop() ?? path;
</script>

<div class="flex-1 min-h-0 flex flex-col">
  <!-- Règles à cocher -->
  <div class="shrink-0 px-6 pt-5 pb-4 space-y-3 border-b border-(--rg-bd) bg-(--rg-creux)">
    <div class="flex flex-wrap gap-2">
      {#each CLEAN_RULES as rule (rule)}
        {@const on = rules.includes(rule)}
        <button
          type="button"
          onclick={() => toggle(rule)}
          aria-pressed={on}
          class="flex items-center gap-1.5 h-8 px-3 rounded-xl border text-[12.5px] cursor-pointer transition-colors
                 {on
                   ? 'border-(--rg-gbd) bg-(--rg-gbg) text-(--rg-gtx) font-semibold'
                   : 'border-(--rg-bd) bg-(--rg-champ) text-(--rg-tx2) font-medium hover:border-(--rg-bd2) hover:text-(--rg-tx)'}"
        >
          {#if on}
            <Icon icon="material-symbols:check-rounded" width="15" class="-ml-0.5 text-(--rg-g)" />
          {/if}
          {$t(`clean.rule.${rule}`)}
        </button>
      {/each}
    </div>
    <p class="flex gap-2 text-xs leading-relaxed text-(--rg-mu) text-pretty">
      <Icon icon="material-symbols:info-outline-rounded" width="15" class="shrink-0 mt-px text-(--rg-mu2)" />
      {$t("clean.hint")}
    </p>
  </div>

  <div class="flex-1 min-h-0 overflow-y-auto scrollbar-app px-6 py-4">
    {#if error}
      <p class="flex items-start gap-2 px-4 py-3 rounded-xl border border-red-500/25 bg-red-500/8
                text-sm text-red-600 dark:text-red-400" transition:fade={{ duration: 120 }}>
        <Icon icon="material-symbols:warning-outline-rounded" width="17" class="shrink-0 mt-px" />
        <span class="min-w-0">{error}</span>
      </p>
    {:else if rules.length === 0 || proposals.length === 0}
      <!-- État vide : aucune règle, calcul en cours, ou rien à changer -->
      <div class="flex flex-col items-center justify-center gap-3 py-16 text-center">
        <span class="w-12 h-12 rounded-2xl flex items-center justify-center border border-(--rg-bd) bg-(--rg-creux) text-(--rg-mu)">
          <Icon icon={computing ? "lucide:loader-2" : "lucide:sparkles"} width="22" class={computing ? "animate-spin" : ""} />
        </span>
        <p class="text-[13px] text-(--rg-mu)">
          {rules.length === 0 ? $t("clean.pick_rule") : computing ? $t("rename.computing") : $t("clean.nothing")}
        </p>
      </div>
    {:else}
      <div class="space-y-1.5">
        {#each proposals as p (p.path + p.field)}
          <div class="flex items-center gap-3 px-3.5 py-2 rounded-xl border border-(--rg-bd) bg-(--rg-creux)">
            <span class="shrink-0 w-32 text-[11px] truncate text-(--rg-mu)" title={fileName(p.path)}>
              {fileName(p.path)}
            </span>
            <span class="shrink-0 w-20 truncate">
              <span class="px-1.5 py-0.5 rounded-md text-[10.5px] font-semibold bg-(--rg-s2) text-(--rg-tx2)">
                {$t(`tags.${p.field}`)}
              </span>
            </span>
            <span class="flex-1 min-w-0 text-[12.5px] truncate line-through text-(--rg-mu2)" title={p.from}>
              {p.from}
            </span>
            <Icon icon="material-symbols:arrow-forward-rounded" width="14" class="shrink-0 text-(--rg-mu2)" />
            <span class="flex-1 min-w-0 text-[12.5px] truncate font-semibold text-(--rg-gtx)" title={p.to}>
              {p.to}
            </span>
          </div>
        {/each}
      </div>
    {/if}
  </div>

  <PopinFooter
    cancelLabel={$t("profil.cancel")}
    oncancel={() => popinStore.close()}
    submitLabel={$t("source.apply")}
    submitIcon="material-symbols:check-rounded"
    onsubmit={apply}
    submitDisabled={proposals.length === 0}
  >
    {#snippet start()}
      {#if proposals.length > 0}
        <span class="px-2 py-0.5 rounded-lg text-[12px] font-bold tabular-nums bg-(--rg-gbg) text-(--rg-gtx)">
          {proposals.length}
        </span>
        <span class="text-[13px] text-(--rg-tx2)">{$t("clean.changes")}</span>
      {/if}
    {/snippet}
  </PopinFooter>
</div>
