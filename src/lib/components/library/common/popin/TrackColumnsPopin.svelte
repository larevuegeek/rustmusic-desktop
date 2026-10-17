<script lang="ts">
  import Icon from "@iconify/svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { t } from "#lib/i18n";
  import { fade, scale } from "svelte/transition";
  import PopinFooter from "#lib/components/ui/popin/PopinFooter.svelte";
  import {
    COLONNES_FIXES,
    COLONNES_PAR_DEFAUT,
    tagTitle,
    columnLabel,
    isOfferableTag,
    type TrackColumn,
  } from "#lib/config/trackColumns";

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
      load();
    }
  });

  async function load() {
    chargement = true;
    try {
      if (libraryId == null) {
        tags = [];
        return;
      }
      const trouves = await invoke<TagCandidate[]>("get_library_tag_keys", { libraryId });
      tags = trouves.filter((tg) => isOfferableTag(tg.key));
    } catch (e) {
      console.error("Failed to load tag keys:", e);
      tags = [];
    } finally {
      chargement = false;
    }
  }

  function isShown(cle: string): boolean {
    return choix.includes(cle);
  }

  function toggleColumn(cle: string) {
    // L'ajout se fait en fin de liste : une colonne qu'on vient de cocher
    // apparaît là où le regard la cherche, à droite, sans bousculer les autres.
    choix = isShown(cle) ? choix.filter((c) => c !== cle) : [...choix, cle];
  }

  function move(index: number, sens: -1 | 1) {
    const cible = index + sens;
    if (cible < 0 || cible >= choix.length) return;
    const copie = [...choix];
    [copie[index], copie[cible]] = [copie[cible], copie[index]];
    choix = copie;
  }

  /** Intitulé d'une clé, qu'elle vienne des colonnes bâties ou d'un tag. */
  function columnTitle(cle: string): string {
    if (cle.startsWith("tag:")) return tagTitle(cle.slice("tag:".length), $t);
    const col = COLONNES_FIXES.find((c) => c.key === cle);
    return col ? columnLabel(col, $t) : cle;
  }

  const filtre = $derived(recherche.trim().toLowerCase());

  const fixesVisibles = $derived(
    COLONNES_FIXES.filter((c: TrackColumn) =>
      !filtre || columnLabel(c, $t).toLowerCase().includes(filtre)
    )
  );

  const tagsVisibles = $derived(
    tags.filter((tg) => !filtre || tagTitle(tg.key, $t).toLowerCase().includes(filtre))
  );

  function confirm() {
    onchange(choix);
    open = false;
  }

  function reset() {
    choix = [...COLONNES_PAR_DEFAUT];
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="fixed inset-0 z-50 flex items-center justify-center"
  onkeydown={(e) => e.key === 'Escape' && (open = false)}
  transition:fade={{ duration: 150 }}
>
  <button
    type="button"
    class="absolute inset-0 bg-black/60 backdrop-blur-sm cursor-default"
    onclick={() => open = false}
    aria-label={$t("common.close")}
  ></button>

  <div
    role="dialog"
    aria-modal="true"
    aria-label={$t("columns.popin_title")}
    class="relative w-full max-w-3xl mx-4 max-h-[80vh] flex flex-col
           rounded-2xl overflow-hidden text-(--rg-tx)
           bg-(--rg-carte) border border-(--rg-bd) dark:border-[#2a312d]
           shadow-[0_24px_60px_-12px_rgba(0,0,0,0.25)] dark:shadow-[0_30px_80px_-20px_rgba(0,0,0,0.9)]"
    transition:scale={{ duration: 180, start: 0.96 }}
  >

    <!-- En-tête -->
    <div class="shrink-0 flex items-center justify-between gap-3 px-6 pt-5 pb-4 border-b border-(--rg-bd)
                bg-linear-135 from-(--rg-g)/8 to-transparent to-60%">
      <div class="flex items-center gap-2.5 min-w-0">
        <span class="shrink-0 w-8 h-8 rounded-[9px] flex items-center justify-center
                     bg-(--rg-gbg) text-(--rg-g) border border-(--rg-gbd)">
          <Icon icon="lucide:columns-3" width="15" />
        </span>
        <div class="min-w-0">
          <h2 class="text-[17px] font-extrabold tracking-[-0.01em] text-(--rg-tx) truncate">
            {$t("columns.popin_title")}
          </h2>
          <p class="text-xs text-(--rg-mu) truncate">
            {$t("columns.popin_hint")}
          </p>
        </div>
      </div>
      <button
        type="button"
        onclick={() => open = false}
        class="shrink-0 -mr-1.5 w-8 h-8 rounded-[9px] flex items-center justify-center
               cursor-pointer text-(--rg-mu) hover:text-(--rg-tx) hover:bg-(--rg-hover) transition-colors"
        aria-label={$t("common.close")}
      >
        <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round">
          <path d="M18 6 6 18M6 6l12 12" />
        </svg>
      </button>
    </div>

    <div class="flex-1 min-h-0 grid grid-cols-1 md:grid-cols-[1fr_18rem]">

      <!-- ─── Ce qu'on peut ajouter ─── -->
      <div class="min-h-0 flex flex-col">
        <div class="px-5 pt-4 pb-3">
          <label class="h-10 flex items-center gap-2 px-3.5 rounded-xl border transition-colors
                        bg-(--rg-champ) border-(--rg-bd) hover:border-(--rg-bd2) focus-within:border-(--rg-g)">
            <Icon icon="material-symbols:search-rounded" width="18" class="shrink-0 text-(--rg-mu)" />
            <input
              type="text"
              bind:value={recherche}
              placeholder={$t("columns.filter")}
              class="flex-1 min-w-0 bg-transparent border-0 outline-0 text-sm text-(--rg-tx) placeholder:text-(--rg-ph)"
            />
          </label>
        </div>

        <div class="flex-1 overflow-y-auto scrollbar-app px-5 pb-5 space-y-6">

          <section>
            <h3 class="mb-2 text-[11px] font-bold uppercase tracking-[0.1em] text-(--rg-mu)">
              {$t("columns.library_fields")}
            </h3>
            <div class="grid grid-cols-2 gap-1">
              {#each fixesVisibles as col (col.key)}
                <label class="relative flex items-center gap-2.5 h-9 px-2.5 rounded-xl cursor-pointer transition-colors
                              hover:bg-(--rg-hover) has-checked:bg-(--rg-creux-on)">
                  <input
                    type="checkbox"
                    checked={isShown(col.key)}
                    onchange={() => toggleColumn(col.key)}
                    class="peer sr-only"
                  />
                  {@render coche()}
                  <span class="text-[13px] truncate text-(--rg-tx2) peer-checked:text-(--rg-tx) peer-checked:font-semibold">
                    {columnLabel(col, $t)}
                  </span>
                </label>
              {/each}
            </div>
          </section>

          <section>
            <h3 class="mb-2 text-[11px] font-bold uppercase tracking-[0.1em] text-(--rg-mu)">
              {$t("columns.file_tags")}
            </h3>

            {#if chargement}
              <div class="flex items-center gap-2 py-4 text-xs text-(--rg-mu)">
                <Icon icon="lucide:loader-2" width="14" class="animate-spin" />
                {$t("columns.scanning_tags")}
              </div>
            {:else if tagsVisibles.length === 0}
              <p class="text-xs text-(--rg-mu) py-3">
                {filtre ? $t("columns.no_tag_match") : $t("columns.no_extra_tags")}
              </p>
            {:else}
              <div class="grid grid-cols-2 gap-1">
                {#each tagsVisibles as tag (tag.key)}
                  <label class="relative flex items-center gap-2.5 h-9 px-2.5 rounded-xl cursor-pointer transition-colors
                                hover:bg-(--rg-hover) has-checked:bg-(--rg-creux-on)">
                    <input
                      type="checkbox"
                      checked={isShown(`tag:${tag.key}`)}
                      onchange={() => toggleColumn(`tag:${tag.key}`)}
                      class="peer sr-only"
                    />
                    {@render coche()}
                    <span class="text-[13px] truncate flex-1 text-(--rg-tx2) peer-checked:text-(--rg-tx) peer-checked:font-semibold">
                      {tagTitle(tag.key, $t)}
                    </span>
                    <!-- L'effectif dit d'un coup d'œil si la colonne sera
                         majoritairement vide. -->
                    <span class="text-[11px] tabular-nums text-(--rg-mu2) shrink-0">
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
      <div class="min-h-0 flex flex-col border-t md:border-t-0 md:border-l border-(--rg-bd) bg-(--rg-creux)">
        <h3 class="px-5 pt-5 pb-3 text-[11px] font-bold uppercase tracking-[0.1em] text-(--rg-mu)">
          {$t("columns.shown").replace("{n}", String(choix.length))}
        </h3>

        <div class="flex-1 overflow-y-auto scrollbar-app px-3 pb-3 space-y-1">
          {#if choix.length === 0}
            <p class="text-xs text-(--rg-mu) px-2 py-3">
              {$t("columns.none_shown")}
            </p>
          {/if}

          {#each choix as cle, i (cle)}
            <div class="flex items-center gap-0.5 h-9 pl-2.5 pr-1 rounded-xl transition-colors
                        bg-(--rg-carte) border border-(--rg-bd) hover:border-(--rg-bd2)">
              <span class="w-5 shrink-0 text-[11px] font-semibold tabular-nums text-(--rg-mu2)">{i + 1}</span>
              <span class="text-[13px] font-medium text-(--rg-tx) truncate flex-1">
                {columnTitle(cle)}
              </span>
              <button
                type="button"
                onclick={() => move(i, -1)}
                disabled={i === 0}
                class="w-7 h-7 rounded-lg flex items-center justify-center cursor-pointer transition-colors
                       text-(--rg-mu) hover:bg-(--rg-hover) hover:text-(--rg-tx)
                       disabled:opacity-25 disabled:cursor-default disabled:hover:bg-transparent"
                aria-label={$t("tags.move_up")}
              >
                <Icon icon="lucide:chevron-up" width="14" />
              </button>
              <button
                type="button"
                onclick={() => move(i, 1)}
                disabled={i === choix.length - 1}
                class="w-7 h-7 rounded-lg flex items-center justify-center cursor-pointer transition-colors
                       text-(--rg-mu) hover:bg-(--rg-hover) hover:text-(--rg-tx)
                       disabled:opacity-25 disabled:cursor-default disabled:hover:bg-transparent"
                aria-label={$t("tags.move_down")}
              >
                <Icon icon="lucide:chevron-down" width="14" />
              </button>
              <button
                type="button"
                onclick={() => toggleColumn(cle)}
                class="w-7 h-7 rounded-lg flex items-center justify-center cursor-pointer transition-colors
                       text-(--rg-mu) hover:bg-red-500/8 hover:text-red-600 dark:hover:text-red-400"
                aria-label={$t("mini.remove")}
              >
                <Icon icon="lucide:x" width="14" />
              </button>
            </div>
          {/each}
        </div>
      </div>
    </div>

    <PopinFooter
      cancelLabel={$t("common.cancel")}
      oncancel={() => (open = false)}
      submitLabel={$t("source.apply")}
      submitIcon="material-symbols:check-rounded"
      onsubmit={confirm}
    >
      {#snippet start()}
        <button
          type="button"
          onclick={reset}
          class="h-8 px-3 rounded-lg flex items-center gap-1.5 text-xs font-semibold cursor-pointer transition-colors
                 text-(--rg-mu) hover:bg-(--rg-hover) hover:text-(--rg-tx)"
        >
          <Icon icon="material-symbols:restart-alt-rounded" width="15" />
          {$t("columns.reset_columns")}
        </button>
        {#if onresetwidths}
          <!-- Une largeur mal tirée n'a aucun autre moyen de revenir : le geste
               est libre, donc il lui faut une porte de sortie. -->
          <button
            type="button"
            onclick={onresetwidths}
            class="h-8 px-3 rounded-lg flex items-center gap-1.5 text-xs font-semibold cursor-pointer transition-colors
                   text-(--rg-mu) hover:bg-(--rg-hover) hover:text-(--rg-tx)"
          >
            <Icon icon="material-symbols:width-rounded" width="15" />
            {$t("columns.reset_widths")}
          </button>
        {/if}
      {/snippet}
    </PopinFooter>
  </div>
</div>

<!-- Case à cocher dessinée, pilotée par l'input frère (peer) -->
{#snippet coche()}
  <span
    class="shrink-0 w-4.5 h-4.5 rounded-md flex items-center justify-center border-[1.5px] transition-colors
           border-(--rg-bd2) text-transparent
           peer-checked:bg-(--rg-g) peer-checked:border-(--rg-g) peer-checked:text-(--rg-on-g)
           peer-focus-visible:outline-2 peer-focus-visible:outline-offset-2 peer-focus-visible:outline-(--rg-g)"
    aria-hidden="true"
  >
    <Icon icon="material-symbols:check-rounded" width="14" />
  </span>
{/snippet}
