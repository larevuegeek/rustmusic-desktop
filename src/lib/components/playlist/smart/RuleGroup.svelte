<script lang="ts">
  import Icon from "@iconify/svelte";
  import RuleGroup from "./RuleGroup.svelte";
  import type { FieldOption, Vocabulary, Node, Group } from "./types";
  import { operatorsFor, defaultRule, arityOf } from "./types";

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
              ? 'border-neutral-200 dark:border-white/10 bg-neutral-100/60 dark:bg-white/3 p-3'
              : 'border-transparent'}">

  <!-- Le liant du groupe -->
  <div class="flex items-center gap-2 mb-2">
    <span class="text-xs text-neutral-500 dark:text-neutral-400">Correspond à</span>
    <select
      bind:value={group.match}
      class="text-xs px-2 py-1 rounded-md cursor-pointer
             bg-white dark:bg-neutral-800 border border-neutral-200 dark:border-white/10
             text-neutral-800 dark:text-neutral-200"
    >
      <option value="all">toutes les conditions</option>
      <option value="any">au moins une condition</option>
    </select>

    {#if onremove}
      <button
        type="button"
        onclick={onremove}
        class="ml-auto p-1 rounded cursor-pointer text-neutral-400 hover:text-red-500"
        aria-label="Retirer le groupe"
      >
        <Icon icon="lucide:x" width="14" />
      </button>
    {/if}
  </div>

  <div class="space-y-1.5">
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
        <div class="flex items-center gap-1.5">
          <!-- Champ -->
          <select
            value={noeud.field}
            onchange={(e) => changerChamp(i, e.currentTarget.value)}
            class="text-xs px-2 py-1.5 rounded-md cursor-pointer min-w-0 flex-1
                   bg-white dark:bg-neutral-800 border border-neutral-200 dark:border-white/10
                   text-neutral-800 dark:text-neutral-200"
          >
            {#each fields as f (f.key)}
              <option value={f.key}>{f.label}</option>
            {/each}
          </select>

          <!-- Opérateur -->
          <select
            bind:value={noeud.op}
            class="text-xs px-2 py-1.5 rounded-md cursor-pointer w-40 shrink-0
                   bg-white dark:bg-neutral-800 border border-neutral-200 dark:border-white/10
                   text-neutral-800 dark:text-neutral-200"
          >
            {#each ops as o (o.key)}
              <option value={o.key}>{o.label}</option>
            {/each}
          </select>

          <!-- Valeur(s). Rien n'est affiché pour « est vide » : un champ de
               saisie inerte ferait croire qu'on attend quelque chose. -->
          {#if arite === 1}
            <input
              type="text"
              bind:value={noeud.value}
              placeholder="valeur"
              class="text-xs px-2 py-1.5 rounded-md w-40 shrink-0
                     bg-white dark:bg-white/5 border border-neutral-200 dark:border-white/10
                     text-neutral-800 dark:text-neutral-200
                     focus:outline-none focus:border-emerald-400"
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
                class="text-xs px-2 py-1.5 rounded-md w-full min-w-0
                       bg-white dark:bg-white/5 border border-neutral-200 dark:border-white/10
                       text-neutral-800 dark:text-neutral-200 focus:outline-none focus:border-emerald-400"
              />
              <span class="text-[10px] text-neutral-400 shrink-0">et</span>
              <input
                type="number"
                value={Array.isArray(noeud.value) ? noeud.value[1] : ''}
                onchange={(e) => {
                  const a = Array.isArray(noeud.value) ? noeud.value[0] : 0;
                  group.rules[i] = { ...noeud, value: [a, Number(e.currentTarget.value)] };
                }}
                class="text-xs px-2 py-1.5 rounded-md w-full min-w-0
                       bg-white dark:bg-white/5 border border-neutral-200 dark:border-white/10
                       text-neutral-800 dark:text-neutral-200 focus:outline-none focus:border-emerald-400"
              />
            </div>
          {:else}
            <div class="w-40 shrink-0"></div>
          {/if}

          <button
            type="button"
            onclick={() => retirer(i)}
            class="p-1 rounded cursor-pointer shrink-0 text-neutral-400 hover:text-red-500"
            aria-label="Retirer la condition"
          >
            <Icon icon="lucide:x" width="14" />
          </button>
        </div>
      {/if}
    {/each}
  </div>

  <div class="flex items-center gap-2 mt-2">
    <button
      type="button"
      onclick={ajouterRegle}
      class="text-[11px] flex items-center gap-1 cursor-pointer
             text-neutral-500 dark:text-neutral-400
             hover:text-neutral-800 dark:hover:text-neutral-200"
    >
      <Icon icon="lucide:plus" width="12" />
      Condition
    </button>
    {#if depth < PROFONDEUR_MAX - 1}
      <button
        type="button"
        onclick={ajouterGroupe}
        class="text-[11px] flex items-center gap-1 cursor-pointer
               text-neutral-500 dark:text-neutral-400
               hover:text-neutral-800 dark:hover:text-neutral-200"
      >
        <Icon icon="lucide:brackets" width="12" />
        Groupe
      </button>
    {/if}
  </div>
</div>
