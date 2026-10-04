<script lang="ts">
  import Icon from "@iconify/svelte";
  import RuleGroup from "./RuleGroup.svelte";
  import { t } from "$lib/i18n";
  import type { FieldOption, Vocabulary, Node, Group } from "./types";
  import { operatorsFor, defaultRule, arityOf, libelleChamp, libelleOperateur } from "./types";

  /**
   * Un groupe de conditions, et ses sous-groupes.
   *
   * Le composant s'appelle lui-même : c'est ce qui permet « toutes ces
   * conditions, dont au moins une de celles-ci » sans dupliquer le rendu à
   * chaque niveau. La profondeur est bornée par le moteur, qui refuse au-delà
   * de trois — la limite est rappelée ici pour ne pas proposer un bouton qui
   * mènerait à une erreur.
   */
  let {
    group = $bindable(),
    fields,
    vocabulary,
    depth = 0,
    onremove,
  }: {
    group: Group;
    fields: FieldOption[];
    vocabulary: Vocabulary;
    depth?: number;
    /** Absent pour la racine, qui ne se supprime pas. */
    onremove?: () => void;
  } = $props();

  const PROFONDEUR_MAX = 3;

  function ajouterRegle() {
    group.rules = [...group.rules, defaultRule(fields)];
  }

  function ajouterGroupe() {
    group.rules = [
      ...group.rules,
      { match: group.match === "all" ? "any" : "all", rules: [defaultRule(fields)] },
    ];
  }

  function retirer(i: number) {
    group.rules = group.rules.filter((_, j) => j !== i);
  }

  function estGroupe(n: Node): n is Group {
    return 'rules' in n;
  }

  /** Change le champ d'une règle, en réajustant l'opérateur si besoin. */
  function changerChamp(i: number, cle: string) {
    const n = group.rules[i];
    if (estGroupe(n)) return;
    const ops = operatorsFor(vocabulary, fields, cle);
    // L'opérateur courant peut ne pas exister pour la nouvelle nature de champ
    // — « contient » n'a pas de sens sur une note. On retombe sur le premier
    // proposé plutôt que de laisser une règle que le moteur refusera.
    const garde = ops.some(o => o.key === n.op);
    group.rules[i] = {
      ...n,
      field: cle,
      op: garde ? n.op : ops[0]?.key ?? 'is',
      value: garde ? n.value : null,
    };
  }
</script>

<div class="rounded-lg border {depth > 0
              ? 'border-(--rg-bd) bg-(--rg-carte) p-3'
              : 'border-transparent'}">

  <!-- Le liant du groupe -->
  <div class="flex items-center gap-2 mb-3">
    <span class="text-[13px] text-(--rg-mu)">{$t("smart.match")}</span>
    <select
      bind:value={group.match}
      class="h-8 px-2.5 rounded-lg text-[13px] cursor-pointer bg-(--rg-champ) border border-(--rg-bd) text-(--rg-tx)"
    >
      <option value="all">{$t("smart.match_all")}</option>
      <option value="any">{$t("smart.match_any")}</option>
    </select>

    {#if onremove}
      <button
        type="button"
        onclick={onremove}
        class="ml-auto w-8 h-8 flex items-center justify-center rounded-lg cursor-pointer text-(--rg-mu) hover:text-red-500 hover:bg-red-500/10"
        aria-label={$t("smart.remove_group")}
      >
        <Icon icon="lucide:x" width="14" />
      </button>
    {/if}
  </div>

  <div class="space-y-2">
    {#each group.rules as noeud, i (i)}
      {#if estGroupe(noeud)}
        <RuleGroup
          bind:group={group.rules[i] as Group}
          {fields}
          {vocabulary}
          depth={depth + 1}
          onremove={() => retirer(i)}
        />
      {:else}
        {@const ops = operatorsFor(vocabulary, fields, noeud.field)}
        {@const arite = arityOf(ops, noeud.op)}
        <div class="flex items-center gap-2">
          <!-- Champ -->
          <select
            value={noeud.field}
            onchange={(e) => changerChamp(i, e.currentTarget.value)}
            class="h-9 px-2.5 rounded-lg text-[13px] cursor-pointer min-w-0 flex-1 bg-(--rg-champ) border border-(--rg-bd) text-(--rg-tx)"
          >
            {#each fields as f (f.key)}
              <option value={f.key}>{libelleChamp(f, $t)}</option>
            {/each}
          </select>

          <!-- Opérateur -->
          <select
            bind:value={noeud.op}
            class="h-9 px-2.5 rounded-lg text-[13px] cursor-pointer w-40 shrink-0 bg-(--rg-champ) border border-(--rg-bd) text-(--rg-tx)"
          >
            {#each ops as o (o.key)}
              <option value={o.key}>{libelleOperateur(o, $t)}</option>
            {/each}
          </select>

          <!-- Valeur(s). Rien n'est affiché pour « est vide » : un champ de
               saisie inerte ferait croire qu'on attend quelque chose. -->
          {#if arite === 1}
            <input
              type="text"
              bind:value={noeud.value}
              placeholder={$t("smart.value")}
              class="h-9 px-2.5 rounded-lg text-[13px] w-40 shrink-0 bg-(--rg-champ) border border-(--rg-bd) text-(--rg-tx) outline-none focus:border-(--rg-g)"
            />
          {:else if arite === 2}
            <div class="flex items-center gap-1 w-40 shrink-0">
              <input
                type="number"
                value={Array.isArray(noeud.value) ? noeud.value[0] : ''}
                onchange={(e) => {
                  const b = Array.isArray(noeud.value) ? noeud.value[1] : 0;
                  group.rules[i] = { ...noeud, value: [Number(e.currentTarget.value), b] };
                }}
                class="h-9 px-2.5 rounded-lg text-[13px] w-full min-w-0 bg-(--rg-champ) border border-(--rg-bd) text-(--rg-tx) outline-none focus:border-(--rg-g)"
              />
              <span class="text-xs text-(--rg-mu) shrink-0">{$t("smart.and")}</span>
              <input
                type="number"
                value={Array.isArray(noeud.value) ? noeud.value[1] : ''}
                onchange={(e) => {
                  const a = Array.isArray(noeud.value) ? noeud.value[0] : 0;
                  group.rules[i] = { ...noeud, value: [a, Number(e.currentTarget.value)] };
                }}
                class="h-9 px-2.5 rounded-lg text-[13px] w-full min-w-0 bg-(--rg-champ) border border-(--rg-bd) text-(--rg-tx) outline-none focus:border-(--rg-g)"
              />
            </div>
          {:else}
            <div class="w-40 shrink-0"></div>
          {/if}

          <button
            type="button"
            onclick={() => retirer(i)}
            class="w-8 h-8 flex items-center justify-center rounded-lg cursor-pointer shrink-0 text-(--rg-mu) hover:text-red-500 hover:bg-red-500/10"
            aria-label={$t("smart.remove_condition")}
          >
            <Icon icon="lucide:x" width="14" />
          </button>
        </div>
      {/if}
    {/each}
  </div>

  <div class="flex items-center gap-2 mt-3">
    <button
      type="button"
      onclick={ajouterRegle}
      class="h-8 px-3 rounded-full text-[12.5px] font-semibold flex items-center gap-1.5 cursor-pointer border border-(--rg-bd) text-(--rg-tx2) hover:border-(--rg-bd2) hover:text-(--rg-tx) transition-colors"
    >
      <Icon icon="lucide:plus" width="12" />
      {$t("smart.condition")}
    </button>
    {#if depth < PROFONDEUR_MAX - 1}
      <button
        type="button"
        onclick={ajouterGroupe}
        class="h-8 px-3 rounded-full text-[12.5px] font-semibold flex items-center gap-1.5 cursor-pointer border border-(--rg-bd) text-(--rg-tx2) hover:border-(--rg-bd2) hover:text-(--rg-tx) transition-colors"
      >
        <Icon icon="lucide:brackets" width="12" />
        {$t("smart.group")}
      </button>
    {/if}
  </div>
</div>
