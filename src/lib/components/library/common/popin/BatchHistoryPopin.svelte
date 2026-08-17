<script lang="ts">
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
  import { popinStore } from "$lib/stores/ui/popin.store";
  import { t } from "$lib/i18n";
  import {
    listBatchJournal,
    undoBatch,
    type JournalEntry,
    type MoveOutcome,
  } from "$lib/services/tags/rename.service";

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
      error = String((e as any)?.message ?? e ?? "");
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
      error = String((e as any)?.message ?? e ?? "");
    } finally {
      working = null;
    }
  }

  /** Date lisible, dans la langue de l'utilisateur. */
  function when(value: string): string {
    // SQLite rend « 2026-08-17 14:32:05 » sans fuseau : c'est de l'UTC, et le
    // dire explicitement évite un décalage de deux heures en été.
    const date = new Date(value.replace(" ", "T") + "Z");
    return Number.isNaN(date.getTime()) ? value : date.toLocaleString();
  }
</script>

<div class="flex-1 min-h-0 flex flex-col">
  <div class="flex-1 min-h-0 overflow-y-auto scrollbar-none px-4 py-3">
    {#if error}
      <p class="flex items-start gap-2 px-3 py-2.5 mb-2 rounded-lg text-[12px]
                bg-red-500/10 text-red-500" transition:fade={{ duration: 120 }}>
        <Icon icon="lucide:alert-triangle" width="13" class="shrink-0 mt-0.5" />
        <span class="min-w-0">{error}</span>
      </p>
    {/if}

    {#if result}
      <p class="flex items-start gap-2 px-3 py-2.5 mb-2 rounded-lg text-[12px]
                {result.failed.length > 0
                  ? 'bg-amber-500/10 text-amber-700 dark:text-amber-300'
                  : 'bg-emerald-500/10 text-emerald-700 dark:text-emerald-300'}"
         transition:fade={{ duration: 120 }}>
        <Icon
          icon={result.failed.length > 0 ? "lucide:triangle-alert" : "lucide:check-circle-2"}
          width="13"
          class="shrink-0 mt-0.5"
        />
        <span class="min-w-0">
          {result.succeeded} {$t("history.restored")}
          {#if result.failed.length > 0}
            · {result.failed.length} {$t("rename.failed")}
            <span class="block mt-1 text-[10.5px] opacity-80">
              {result.failed[0].message}
            </span>
          {/if}
        </span>
      </p>
    {/if}

    {#if loading && entries.length === 0}
      <div class="space-y-1.5" aria-hidden="true">
        {#each Array.from({ length: 4 }) as _, i (i)}
          <div class="h-14 rounded-xl animate-pulse bg-neutral-100/70 dark:bg-white/4"></div>
        {/each}
      </div>
    {:else if entries.length === 0}
      <div class="flex flex-col items-center justify-center gap-2.5 py-20 text-center">
        <Icon icon="lucide:history" width="30" class="text-neutral-300 dark:text-neutral-700" />
        <p class="text-[12.5px] text-neutral-400 dark:text-neutral-500">
          {$t("history.empty")}
        </p>
      </div>
    {:else}
      <div class="space-y-1.5">
        {#each entries as entry (entry.id)}
          {@const undone = entry.undone_at !== null}
          <div class="rounded-xl bg-neutral-100/70 dark:bg-white/3 {undone ? 'opacity-55' : ''}">
            <div class="flex items-center gap-3 px-3 py-2.5">
              <Icon
                icon={entry.kind === "restructure" ? "lucide:folder-tree" : "lucide:file-pen-line"}
                width="15"
                class="shrink-0 text-neutral-400 dark:text-neutral-500"
              />

              <span class="min-w-0 flex-1">
                <span class="block text-[12.5px] font-medium
                             text-neutral-800 dark:text-neutral-100">
                  {$t(`history.kind.${entry.kind}`)}
                  <span class="font-normal text-neutral-400 dark:text-neutral-500">
                    · {when(entry.created_at)}
                  </span>
                </span>
                <!-- Le motif employé : c'est lui qui permet de reconnaître son
                     lot au milieu de dix autres du même jour. -->
                <span class="block font-mono text-[10.5px] truncate
                             text-neutral-400 dark:text-neutral-500"
                      title={entry.pattern ?? ""}>
                  {entry.pattern ?? "—"}
                </span>
              </span>

              <span class="shrink-0 flex items-center gap-1.5 text-[11px] tabular-nums">
                <span class="px-1.5 py-0.5 rounded
                             bg-emerald-500/15 text-emerald-600 dark:text-emerald-400">
                  {entry.succeeded}
                </span>
                {#if entry.failed > 0}
                  <span class="px-1.5 py-0.5 rounded
                               bg-amber-500/15 text-amber-600 dark:text-amber-400">
                    {entry.failed}
                  </span>
                {/if}
              </span>

              {#if undone}
                <span class="shrink-0 text-[10.5px] italic
                             text-neutral-400 dark:text-neutral-500">
                  {$t("history.already_undone")}
                </span>
              {:else if confirming === entry.id}
                <span class="shrink-0 flex items-center gap-1">
                  <button
                    type="button"
                    onclick={() => (confirming = null)}
                    class="px-2 h-7 rounded-lg text-[11.5px] cursor-pointer
                           text-neutral-500 dark:text-neutral-400
                           hover:bg-neutral-200/70 dark:hover:bg-white/8"
                  >
                    {$t("profil.cancel")}
                  </button>
                  <button
                    type="button"
                    onclick={() => undo(entry.id)}
                    disabled={working !== null}
                    class="flex items-center gap-1.5 px-2.5 h-7 rounded-lg text-[11.5px]
                           font-medium cursor-pointer transition-colors
                           bg-amber-500 text-white hover:bg-amber-600 disabled:opacity-50"
                  >
                    {#if working === entry.id}
                      <Icon icon="lucide:loader-circle" width="11" class="animate-spin" />
                    {/if}
                    {$t("history.confirm")}
                  </button>
                </span>
              {:else}
                <button
                  type="button"
                  onclick={() => (confirming = entry.id)}
                  disabled={working !== null || entry.succeeded === 0}
                  class="shrink-0 flex items-center gap-1.5 px-2.5 h-7 rounded-lg text-[11.5px]
                         cursor-pointer transition-colors disabled:opacity-30
                         text-neutral-600 dark:text-neutral-300
                         hover:bg-neutral-200/70 dark:hover:bg-white/8"
                >
                  <Icon icon="lucide:undo-2" width="12" />
                  {$t("history.undo")}
                </button>
              {/if}
            </div>
          </div>
        {/each}
      </div>

      <!-- La borne est dite : sans elle, on chercherait un lot d'il y a trois
           mois en croyant à un bug. -->
      <p class="mt-3 px-1 text-[10.5px] text-neutral-400 dark:text-neutral-500">
        {$t("history.retention")}
      </p>
    {/if}
  </div>

  <footer
    class="shrink-0 flex items-center gap-3 px-4 py-2.5
           border-t border-neutral-200/70 dark:border-white/8
           bg-neutral-50/80 dark:bg-black/25"
  >
    <span class="flex-1"></span>
    <button
      type="button"
      onclick={() => popinStore.close()}
      class="flex items-center gap-1.5 px-3 h-8 rounded-lg text-[13px]
             cursor-pointer transition-colors
             text-neutral-600 dark:text-neutral-300
             hover:bg-neutral-200/70 dark:hover:bg-white/8"
    >
      <Icon icon="lucide:x" width="13" />
      {$t("batch.close")}
    </button>
  </footer>
</div>
