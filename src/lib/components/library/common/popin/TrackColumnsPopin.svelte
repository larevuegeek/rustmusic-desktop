<script lang="ts">
  import Icon from "@iconify/svelte";
  import { invoke } from "@tauri-apps/api/core";
  import {
    COLONNES_FIXES,
    COLONNES_PAR_DEFAUT,
    intituleDeTag,
    tagProposable,
    type TrackColumn,
  } from "$lib/config/trackColumns";

  type TagCandidate = { key: string; filled: number };

  let {
    open = $bindable(true),
    libraryId,
    colonnes,
    onchange,
    onresetwidths,
  }: {
    open: boolean;
    /** `null` : aucun tag à recenser, seuls les champs bâtis sont proposés. */
    libraryId: number | null;
    /** Clés actuellement affichées, dans l'ordre. */
    colonnes: string[];
    onchange: (cles: string[]) => void;
    /** Rend aux colonnes leur largeur d'origine. */
    onresetwidths?: () => void;
  } = $props();

  let choix = $state<string[]>([]);
  let tags = $state<TagCandidate[]>([]);
  let chargement = $state(true);
  let recherche = $state("");

  // À l'ouverture : on prend une copie du choix courant, et on recense les
  // tags. La copie est ce qu'on édite — annuler doit vraiment tout laisser en
  // place, et rien n'est écrit avant « Appliquer ».
  //
  // Le recensement lit toute la bibliothèque : il ne se déclenche qu'ici,
  // jamais au montage de la page.
  $effect(() => {
    if (open) {
      choix = [...colonnes];
      charger();
    }
  });

  async function charger() {
    chargement = true;
    try {
      if (libraryId == null) {
        tags = [];
        return;
      }
      const trouves = await invoke<TagCandidate[]>("get_library_tag_keys", { libraryId });
      tags = trouves.filter((t) => tagProposable(t.key));
    } catch (e) {
      console.error("Failed to load tag keys:", e);
      tags = [];
    } finally {
      chargement = false;
    }
  }

  function estAffichee(cle: string): boolean {
    return choix.includes(cle);
  }

  function basculer(cle: string) {
    // L'ajout se fait en fin de liste : une colonne qu'on vient de cocher
    // apparaît là où le regard la cherche, à droite, sans bousculer les autres.
    choix = estAffichee(cle) ? choix.filter((c) => c !== cle) : [...choix, cle];
  }

  function deplacer(index: number, sens: -1 | 1) {
    const cible = index + sens;
    if (cible < 0 || cible >= choix.length) return;
    const copie = [...choix];
    [copie[index], copie[cible]] = [copie[cible], copie[index]];
    choix = copie;
  }

  /** Intitulé d'une clé, qu'elle vienne des colonnes bâties ou d'un tag. */
  function intitule(cle: string): string {
    if (cle.startsWith("tag:")) return intituleDeTag(cle.slice("tag:".length));
    return COLONNES_FIXES.find((c) => c.key === cle)?.label ?? cle;
  }

  const filtre = $derived(recherche.trim().toLowerCase());

  const fixesVisibles = $derived(
    COLONNES_FIXES.filter((c: TrackColumn) =>
      !filtre || c.label.toLowerCase().includes(filtre)
    )
  );

  const tagsVisibles = $derived(
    tags.filter((t) => !filtre || intituleDeTag(t.key).toLowerCase().includes(filtre))
  );

  function valider() {
    onchange(choix);
    open = false;
  }

  function reinitialiser() {
    choix = [...COLONNES_PAR_DEFAUT];
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="fixed inset-0 z-50 flex items-center justify-center"
  onkeydown={(e) => e.key === 'Escape' && (open = false)}
>
  <button
    type="button"
    class="absolute inset-0 bg-black/60 backdrop-blur-sm cursor-default"
    onclick={() => open = false}
    aria-label="Fermer"
  ></button>

  <div class="relative w-full max-w-3xl mx-4 max-h-[80vh] flex flex-col
              bg-neutral-50 dark:bg-neutral-900
              border border-neutral-200/60 dark:border-white/8
              rounded-2xl shadow-2xl shadow-black/20
              overflow-hidden">

    <!-- En-tête -->
    <div class="flex items-center justify-between px-6 py-4
                border-b border-neutral-200/60 dark:border-white/6">
      <div class="flex items-center gap-3">
        <div class="w-8 h-8 rounded-lg flex items-center justify-center
                    bg-green-500/10 border border-green-500/20">
          <Icon icon="lucide:columns-3" width="16" class="text-green-500" />
        </div>
        <div>
          <h2 class="text-base font-semibold text-neutral-800 dark:text-neutral-100">
            Colonnes affichées
          </h2>
          <p class="text-[11px] text-neutral-400 dark:text-neutral-500">
            Les tags proposés sont ceux que porte réellement cette bibliothèque
          </p>
        </div>
      </div>
      <button
        type="button"
        onclick={() => open = false}
        class="p-1.5 rounded-lg cursor-pointer text-neutral-400
               hover:bg-neutral-200/60 dark:hover:bg-white/5"
        aria-label="Fermer"
      >
        <Icon icon="lucide:x" width="16" />
      </button>
    </div>

    <div class="flex-1 min-h-0 grid grid-cols-1 md:grid-cols-[1fr_18rem]">

      <!-- ─── Ce qu'on peut ajouter ─── -->
      <div class="min-h-0 flex flex-col border-r border-neutral-200/60 dark:border-white/6">
        <div class="px-5 pt-4 pb-2">
          <div class="relative">
            <Icon
              icon="lucide:search"
              width="14"
              class="absolute left-3 top-1/2 -translate-y-1/2 text-neutral-400"
            />
            <input
              type="text"
              bind:value={recherche}
              placeholder="Filtrer les colonnes et les tags…"
              class="w-full text-sm pl-9 pr-3 py-2 rounded-lg
                     bg-white dark:bg-white/5
                     border border-neutral-200 dark:border-white/10
                     text-neutral-800 dark:text-neutral-200
                     focus:outline-none focus:border-emerald-400 dark:focus:border-emerald-500"
            />
          </div>
        </div>

        <div class="flex-1 overflow-y-auto scrollbar-app px-5 pb-5 space-y-5">

          <section>
            <h3 class="text-[10px] uppercase tracking-wider text-neutral-400 mb-2">
              Champs de la bibliothèque
            </h3>
            <div class="grid grid-cols-2 gap-1">
              {#each fixesVisibles as col (col.key)}
                <label class="flex items-center gap-2 px-2 py-1.5 rounded-md cursor-pointer
                              hover:bg-neutral-100 dark:hover:bg-white/5">
                  <input
                    type="checkbox"
                    checked={estAffichee(col.key)}
                    onchange={() => basculer(col.key)}
                    class="accent-emerald-500 cursor-pointer"
                  />
                  <span class="text-xs text-neutral-700 dark:text-neutral-300 truncate">
                    {col.label}
                  </span>
                </label>
              {/each}
            </div>
          </section>

          <section>
            <h3 class="text-[10px] uppercase tracking-wider text-neutral-400 mb-2">
              Tags des fichiers
            </h3>

            {#if chargement}
              <div class="flex items-center gap-2 py-4 text-xs text-neutral-400">
                <Icon icon="lucide:loader-2" width="14" class="animate-spin" />
                Recensement des tags…
              </div>
            {:else if tagsVisibles.length === 0}
              <p class="text-xs text-neutral-400 py-3">
                {filtre ? "Aucun tag ne correspond." : "Aucun tag supplémentaire dans cette bibliothèque."}
              </p>
            {:else}
              <div class="grid grid-cols-2 gap-1">
                {#each tagsVisibles as tag (tag.key)}
                  <label class="flex items-center gap-2 px-2 py-1.5 rounded-md cursor-pointer
                                hover:bg-neutral-100 dark:hover:bg-white/5">
                    <input
                      type="checkbox"
                      checked={estAffichee(`tag:${tag.key}`)}
                      onchange={() => basculer(`tag:${tag.key}`)}
                      class="accent-emerald-500 cursor-pointer"
                    />
                    <span class="text-xs text-neutral-700 dark:text-neutral-300 truncate flex-1">
                      {intituleDeTag(tag.key)}
                    </span>
                    <!-- L'effectif dit d'un coup d'œil si la colonne sera
                         majoritairement vide. -->
                    <span class="text-[10px] tabular-nums text-neutral-400 shrink-0">
                      {tag.filled}
                    </span>
                  </label>
                {/each}
              </div>
            {/if}
          </section>
        </div>
      </div>

      <!-- ─── Ce qui est affiché, dans l'ordre ─── -->
      <div class="min-h-0 flex flex-col">
        <h3 class="text-[10px] uppercase tracking-wider text-neutral-400 px-5 pt-6 pb-2">
          Affichées ({choix.length})
        </h3>

        <div class="flex-1 overflow-y-auto scrollbar-app px-3 pb-3">
          {#if choix.length === 0}
            <p class="text-xs text-neutral-400 px-2 py-3">
              Aucune colonne : seuls le titre et la pochette resteront.
            </p>
          {/if}

          {#each choix as cle, i (cle)}
            <div class="group flex items-center gap-1 px-2 py-1.5 rounded-md
                        hover:bg-neutral-100 dark:hover:bg-white/5">
              <span class="text-xs text-neutral-700 dark:text-neutral-300 truncate flex-1">
                {intitule(cle)}
              </span>
              <button
                type="button"
                onclick={() => deplacer(i, -1)}
                disabled={i === 0}
                class="p-0.5 rounded cursor-pointer text-neutral-400
                       hover:text-neutral-700 dark:hover:text-neutral-200
                       disabled:opacity-20 disabled:cursor-default"
                aria-label="Monter"
              >
                <Icon icon="lucide:chevron-up" width="13" />
              </button>
              <button
                type="button"
                onclick={() => deplacer(i, 1)}
                disabled={i === choix.length - 1}
                class="p-0.5 rounded cursor-pointer text-neutral-400
                       hover:text-neutral-700 dark:hover:text-neutral-200
                       disabled:opacity-20 disabled:cursor-default"
                aria-label="Descendre"
              >
                <Icon icon="lucide:chevron-down" width="13" />
              </button>
              <button
                type="button"
                onclick={() => basculer(cle)}
                class="p-0.5 rounded cursor-pointer text-neutral-400 hover:text-red-500"
                aria-label="Retirer"
              >
                <Icon icon="lucide:x" width="13" />
              </button>
            </div>
          {/each}
        </div>
      </div>
    </div>

    <!-- Pied -->
    <div class="flex items-center justify-between px-6 py-3
                border-t border-neutral-200/60 dark:border-white/6">
      <div class="flex items-center gap-3">
        <button
          type="button"
          onclick={reinitialiser}
          class="text-xs cursor-pointer text-neutral-500 dark:text-neutral-400
                 hover:text-neutral-800 dark:hover:text-neutral-200"
        >
          Rétablir les colonnes d'origine
        </button>
        {#if onresetwidths}
          <!-- Une largeur mal tirée n'a aucun autre moyen de revenir : le geste
               est libre, donc il lui faut une porte de sortie. -->
          <button
            type="button"
            onclick={onresetwidths}
            class="text-xs cursor-pointer text-neutral-500 dark:text-neutral-400
                   hover:text-neutral-800 dark:hover:text-neutral-200"
          >
            Rétablir les largeurs
          </button>
        {/if}
      </div>
      <div class="flex items-center gap-2">
        <button
          type="button"
          onclick={() => open = false}
          class="text-xs px-3 py-1.5 rounded-lg cursor-pointer
                 text-neutral-600 dark:text-neutral-300
                 hover:bg-neutral-200/60 dark:hover:bg-white/5"
        >
          Annuler
        </button>
        <button
          type="button"
          onclick={valider}
          class="text-xs px-3 py-1.5 rounded-lg cursor-pointer
                 bg-emerald-500 hover:bg-emerald-400 text-white font-medium"
        >
          Appliquer
        </button>
      </div>
    </div>
  </div>
</div>
