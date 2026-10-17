<script lang="ts">
  import { errorMessage } from "#lib/helper/tools/errorTools";
  // Historique des lots, et retour en arrière.
  //
  // # Pourquoi un écran et pas seulement le bouton du compte rendu
  // Le bouton « Annuler ce lot » n'existe que tant qu'on n'a pas fermé la
  // popin de renommage. Or on se rend compte d'une erreur en regardant sa
  // bibliothèque, cinq minutes plus tard — pas dans la seconde qui suit.
  //
  // # L'annulation peut échouer, et le dit
  // Un fichier déplacé à la main depuis, un disque débranché, un fichier
  // retouché : l'annulation rend compte comme un lot normal. Prétendre avoir
  // tout remis en place serait pire que ne rien proposer.
  import Icon from "@iconify/svelte";
  import { fade } from "svelte/transition";
  import { popinStore } from "#lib/stores/ui/popin.store";
  import { t, currentLocale } from "#lib/i18n";
  import {
    listBatchJournal,
    undoBatch,
    type JournalEntry,
    type MoveOutcome,
  } from "#lib/services/tags/rename.service";
  import PopinFooter from "#lib/components/ui/popin/PopinFooter.svelte";

  /* eslint-disable svelte/valid-prop-names-in-kit-pages */
  const {
    onundone = () => {},
  }: {
    /** Les couples avant/après remis en place, pour que l'appelant se recale. */
    onundone?: (moved: [string, string][]) => void;
  } = $props();

  let entries: JournalEntry[] = $state([]);
  let loading = $state(true);
  let error: string | null = $state(null);
  /** Lot dont on attend la réponse. */
  let working: string | null = $state(null);
  /** Lot pour lequel on demande confirmation. */
  let confirming: string | null = $state(null);
  /** Compte rendu de la dernière annulation. */
  let result: MoveOutcome | null = $state(null);

  $effect(() => {
    load();
  });

  async function load() {
    loading = true;
    try {
      entries = await listBatchJournal(50);
      error = null;
    } catch (e) {
      error = errorMessage(e);
    } finally {
      loading = false;
    }
  }

  async function undo(id: string) {
    working = id;
    error = null;
    try {
      result = await undoBatch(id);
      confirming = null;
      onundone(result.moved);
      await load();
    } catch (e) {
      error = errorMessage(e);
    } finally {
      working = null;
    }
  }

  /** Date lisible, dans la langue de l'utilisateur. */
  function when(value: string): string {
    // SQLite rend « 2026-08-17 14:32:05 » sans fuseau : c'est de l'UTC, et le
    // dire explicitement évite un décalage de deux heures en été.
    const date = new Date(value.replace(" ", "T") + "Z");
    return Number.isNaN(date.getTime()) ? value : date.toLocaleString($currentLocale);
  }
</script>

<div class="flex-1 min-h-0 flex flex-col">
  <div class="flex-1 min-h-0 overflow-y-auto scrollbar-app px-6 py-5">
    {#if error}
      <p class="flex items-start gap-2 px-4 py-3 mb-3 rounded-xl border border-red-500/25 bg-red-500/8
                text-sm text-red-600 dark:text-red-400" transition:fade={{ duration: 120 }}>
        <Icon icon="material-symbols:warning-outline-rounded" width="17" class="shrink-0 mt-px" />
        <span class="min-w-0">{error}</span>
      </p>
    {/if}

    {#if result}
      <p class="flex items-start gap-2 px-4 py-3 mb-3 rounded-xl border text-sm
                {result.failed.length > 0
                  ? 'border-(--rg-ambd) bg-(--rg-ambg) text-(--rg-amtx)'
                  : 'border-(--rg-gbd) bg-(--rg-gbg) text-(--rg-gtx)'}"
         transition:fade={{ duration: 120 }}>
        <Icon
          icon={result.failed.length > 0 ? "material-symbols:warning-outline-rounded" : "material-symbols:check-circle-outline-rounded"}
          width="17"
          class="shrink-0 mt-px {result.failed.length > 0 ? 'text-(--rg-am)' : 'text-(--rg-g)'}"
        />
        <span class="min-w-0 font-medium">
          {result.succeeded} {$t("history.restored")}
          {#if result.failed.length > 0}
            · {result.failed.length} {$t("rename.failed")}
            <span class="block mt-1 text-xs font-normal opacity-80">
              {result.failed[0].message}
            </span>
          {/if}
        </span>
      </p>
    {/if}

    {#if loading && entries.length === 0}
      <div class="space-y-2" aria-hidden="true">
        {#each Array.from({ length: 4 }) as _, i (i)}
          <div class="h-16 rounded-2xl animate-pulse border border-(--rg-bd) bg-(--rg-creux)"></div>
        {/each}
      </div>
    {:else if entries.length === 0}
      <div class="flex flex-col items-center justify-center gap-3 py-20 text-center">
        <span class="w-12 h-12 rounded-2xl flex items-center justify-center border border-(--rg-bd) bg-(--rg-creux) text-(--rg-mu)">
          <Icon icon="lucide:history" width="22" />
        </span>
        <p class="text-[13px] text-(--rg-mu)">
          {$t("history.empty")}
        </p>
      </div>
    {:else}
      <div class="space-y-2">
        {#each entries as entry (entry.id)}
          {@const undone = entry.undone_at !== null}
          <div class="flex items-center gap-3 p-3 rounded-2xl border border-(--rg-bd) bg-(--rg-creux)
                      {undone ? 'opacity-55' : ''}">
            <span class="w-9 h-9 shrink-0 rounded-[10px] border flex items-center justify-center
                         bg-(--rg-s2) border-(--rg-bd) text-(--rg-tx2)">
              <Icon icon={entry.kind === "restructure" ? "lucide:folder-tree" : "lucide:file-pen-line"} width="17" />
            </span>

            <span class="min-w-0 flex-1">
              <span class="block truncate text-[13px] font-semibold text-(--rg-tx)">
                {$t(`history.kind.${entry.kind}`)}
                <span class="font-normal text-(--rg-mu)">· {when(entry.created_at)}</span>
              </span>
              <!-- Le motif : il distingue un lot parmi ceux du même jour. -->
              <span class="block mt-0.5 font-mono text-[11px] truncate text-(--rg-mu)"
                    title={entry.pattern ?? ""}>
                {entry.pattern ?? "—"}
              </span>
            </span>

            <span class="shrink-0 flex items-center gap-1.5 font-mono text-[11px] font-bold tabular-nums">
              <span class="px-1.75 py-0.5 rounded-lg bg-(--rg-gbg) text-(--rg-gtx)">
                {entry.succeeded}
              </span>
              {#if entry.failed > 0}
                <span class="px-1.75 py-0.5 rounded-lg bg-(--rg-ambg) text-(--rg-am)">
                  {entry.failed}
                </span>
              {/if}
            </span>

            {#if undone}
              <span class="shrink-0 text-[11.5px] italic text-(--rg-mu)">
                {$t("history.already_undone")}
              </span>
            {:else if confirming === entry.id}
              <span class="shrink-0 flex items-center gap-1">
                <button
                  type="button"
                  onclick={() => (confirming = null)}
                  class="h-8 px-3 rounded-xl text-[12px] font-semibold cursor-pointer transition-colors
                         text-(--rg-tx2) hover:bg-(--rg-hover) hover:text-(--rg-tx)"
                >
                  {$t("profil.cancel")}
                </button>
                <button
                  type="button"
                  onclick={() => undo(entry.id)}
                  disabled={working !== null}
                  class="flex items-center gap-1.5 h-8 px-3 rounded-xl border text-[12px] font-bold
                         cursor-pointer transition-colors disabled:opacity-50 disabled:cursor-default
                         border-(--rg-ambd) bg-(--rg-ambg) text-(--rg-amtx) hover:border-(--rg-am)"
                >
                  {#if working === entry.id}
                    <Icon icon="lucide:loader-2" width="14" class="animate-spin" />
                  {/if}
                  {$t("history.confirm")}
                </button>
              </span>
            {:else}
              <button
                type="button"
                onclick={() => (confirming = entry.id)}
                disabled={working !== null || entry.succeeded === 0}
                class="shrink-0 flex items-center gap-1.5 h-8 px-3 rounded-xl border text-[12px] font-semibold
                       cursor-pointer transition-colors disabled:opacity-30 disabled:cursor-default
                       border-(--rg-bd) bg-(--rg-champ) text-(--rg-tx2) hover:border-(--rg-bd2) hover:text-(--rg-tx)"
              >
                <Icon icon="lucide:undo-2" width="14" />
                {$t("history.undo")}
              </button>
            {/if}
          </div>
        {/each}
      </div>

      <!-- La limite est dite : sinon un vieux lot absent passerait pour un bug. -->
      <p class="mt-4 flex gap-2 px-1 text-xs text-(--rg-mu)">
        <Icon icon="material-symbols:info-outline-rounded" width="15" class="shrink-0 mt-px text-(--rg-mu2)" />
        {$t("history.retention")}
      </p>
    {/if}
  </div>

  <PopinFooter cancelLabel={$t("batch.close")} oncancel={() => popinStore.close()} />
</div>
