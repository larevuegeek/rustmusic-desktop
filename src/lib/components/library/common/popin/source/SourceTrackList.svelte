<script lang="ts">
  // Colonne des morceaux : outils de sélection, portée de l'appariement, une ligne par morceau.
  import Icon from "@iconify/svelte";
  import { slide } from "svelte/transition";
  import { t } from "#lib/i18n";
  import { valueOf } from "#lib/stores/tags/tagWorkshop.store";
  import SourceTrackRow from "./SourceTrackRow.svelte";
  import { shortName, type SourceSelection } from "./sourceSelection.svelte.js";

  const { selection }: { selection: SourceSelection } = $props();

  let proposal = $derived(selection.proposal);
  /** Les fichiers sans correspondance sont repliés : ils ne se lisent pas. */
  let showUnmatched = $state(false);
</script>

<div class="flex-1 min-w-0 flex flex-col min-h-0">
  {#if !proposal}
    <div class="flex-1 flex flex-col items-center justify-center gap-2.5">
      <Icon icon="lucide:disc-3" width="30"
            class="text-neutral-300 dark:text-neutral-700" />
      <p class="text-[12.5px] text-neutral-400 dark:text-neutral-500">
        {$t('source.pick_album_first')}
      </p>
    </div>
  {:else}
    <!-- ─── Barre d'outils ─── -->
    <div class="shrink-0 flex items-center gap-1.5 px-4 py-2
                border-b border-neutral-200/70 dark:border-white/8">
      <span class="text-[10px] font-semibold uppercase tracking-widest
                   text-neutral-400 dark:text-neutral-500">
        {$t('source.select')}
      </span>
      {#each [['all', 'source.all'], ['empty', 'source.empty_only'], ['none', 'source.none']] as [mode, key] (mode)}
        <button
          type="button"
          onclick={() => selection.selectAll(mode as 'all' | 'empty' | 'none')}
          class="px-2 h-6 rounded-md text-[11px] cursor-pointer transition-colors
                 text-neutral-500 dark:text-neutral-400
                 hover:bg-neutral-200/70 dark:hover:bg-white/8"
        >
          {$t(key)}
        </button>
      {/each}

      <span class="flex-1"></span>

      <button
        type="button"
        onclick={() => selection.toggleAllOpen()}
        class="flex items-center gap-1.5 px-2 h-6 rounded-md text-[11px]
               cursor-pointer transition-colors
               text-neutral-500 dark:text-neutral-400
               hover:bg-neutral-200/70 dark:hover:bg-white/8"
      >
        <Icon icon={selection.allOpen ? 'lucide:chevrons-down-up' : 'lucide:chevrons-up-down'}
              width="12" />
        {selection.allOpen ? $t('source.collapse_all') : $t('source.expand_all')}
      </button>
    </div>

    <div class="flex-1 min-h-0 overflow-y-auto scrollbar-none px-3 py-2.5 space-y-1.5">
      <!-- Portée : combien de fichiers sélectionnés sont réellement concernés. -->
      {#if selection.unmatched.length > 0}
        <div class="rounded-lg bg-amber-500/10">
          <button
            type="button"
            onclick={() => (showUnmatched = !showUnmatched)}
            class="w-full flex items-center gap-2.5 px-3 py-2 text-left
                   rounded-lg cursor-pointer transition-colors
                   hover:bg-amber-500/8"
          >
            <Icon icon="lucide:triangle-alert" width="14"
                  class="shrink-0 text-amber-600 dark:text-amber-400" />
            <span class="min-w-0 flex-1">
              <span class="block text-[12px] font-medium
                           text-amber-700 dark:text-amber-300">
                {selection.pairs.length} {$t('source.matched_of')} {selection.targets.length}
                {$t('tags.files')}
              </span>
              <!-- Cause habituelle : l'album n'a pas autant de pistes que la sélection. -->
              <span class="block text-[10.5px] text-amber-600/80 dark:text-amber-400/70">
                {$t('source.album_has')} {proposal.album.tracks.length}
                {$t('source.tracks')} · {selection.unmatched.length} {$t('source.left_untouched')}
              </span>
            </span>
            <Icon icon={showUnmatched ? 'lucide:chevron-up' : 'lucide:chevron-down'}
                  width="13" class="shrink-0 text-amber-600/70 dark:text-amber-400/60" />
          </button>

          {#if showUnmatched}
            <div class="px-3 pb-2 space-y-0.5" transition:slide={{ duration: 140 }}>
              {#each selection.unmatched as match (match.local_index)}
                {@const file = selection.localFile(match.local_index)}
                <div class="flex items-center gap-2.5 py-0.5">
                  <span class="flex-1 min-w-0 text-[11.5px] truncate
                               text-neutral-600 dark:text-neutral-300">
                    {file ? valueOf(file, 'title') || shortName(file.path) : '—'}
                  </span>
                  {#if selection.excluded.has(match.local_index)}
                    <button
                      type="button"
                      onclick={() => selection.toggleExcluded(match.local_index)}
                      class="shrink-0 text-[10.5px] cursor-pointer transition-colors
                             text-neutral-400 dark:text-neutral-500
                             hover:text-neutral-800 dark:hover:text-neutral-100"
                    >
                      {$t('source.include')}
                    </button>
                  {/if}
                </div>
              {/each}
            </div>
          {/if}
        </div>
      {/if}

      <!-- ─── Les morceaux ─── -->
      {#each selection.pairs as pair (pair.file.path)}
        <SourceTrackRow {pair} {selection} />
      {/each}
    </div>
  {/if}
</div>
