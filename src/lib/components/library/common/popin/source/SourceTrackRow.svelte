<script lang="ts">
  // Un morceau apparié : sa ligne (fichier → piste Deezer) et son détail dépliable.
  import Icon from "@iconify/svelte";
  import { slide } from "svelte/transition";
  import { t } from "$lib/i18n";
  import { valueOf } from "$lib/stores/tags/tagWorkshop.store";
  import { formatBytes } from "$lib/services/tags/tagEditor.service";
  import { formatDuration } from "$lib/services/tags/metadata.service";
  import SourceCellRow from "./SourceCellRow.svelte";
  import {
    cellKey,
    coverKey,
    shortName,
    type Pair,
    type SourceSelection,
  } from "./sourceSelection.svelte.js";

  const { pair, selection }: { pair: Pair; selection: SourceSelection } = $props();

  let chosen = $derived(selection.chosen);
  let remoteCover = $derived(selection.remoteCover);
  let keys = $derived(selection.pairKeys(pair));
  let count = $derived(keys.filter((k) => chosen.has(k)).length);
  let open = $derived(selection.opened.has(pair.index));
</script>

<div
  class="rounded-lg transition-colors
         {count > 0 ? 'bg-emerald-500/8' : 'bg-neutral-100/70 dark:bg-white/3'}"
>
  <div class="flex items-center gap-2.5 px-2.5 py-2">
    <input
      type="checkbox"
      checked={count > 0 && count === keys.length}
      indeterminate={count > 0 && count < keys.length}
      disabled={keys.length === 0}
      onchange={() => selection.toggleKeys(keys)}
      aria-label={valueOf(pair.file, 'title') || shortName(pair.file.path)}
      class="checkbox-app"
    />

    <!-- Titre d'ici et de là-bas sur une ligne : on vérifie l'appariement sans déplier. -->
    <span class="flex-1 min-w-0 flex items-center gap-2">
      <!-- Pochette du fichier : repère d'un coup d'œil ceux qui n'en ont pas. -->
      <span class="shrink-0 w-7 h-7 rounded overflow-hidden
                   ring-1 ring-black/5 dark:ring-white/10
                   bg-neutral-200/60 dark:bg-white/5">
        {#if pair.file.cover}
          <img src={pair.file.cover} alt="" class="w-full h-full object-cover" />
        {:else}
          <span class="w-full h-full flex items-center justify-center">
            <Icon icon="lucide:image-off" width="11"
                  class="text-neutral-400 dark:text-neutral-600" />
          </span>
        {/if}
      </span>

      <span class="flex-1 min-w-0 text-[12px] truncate
                   text-neutral-700 dark:text-neutral-200"
            title={shortName(pair.file.path)}>
        {valueOf(pair.file, 'title') || shortName(pair.file.path)}
      </span>
      <Icon icon="lucide:arrow-right" width="10"
            class="shrink-0 text-neutral-300 dark:text-neutral-600" />

      <!-- Celle de Deezer seulement si retenue : sinon elle promettrait un changement. -->
      {#if remoteCover && chosen.has(coverKey(pair.file.path))}
        <span class="shrink-0 w-7 h-7 rounded overflow-hidden
                     ring-1 ring-emerald-500/60">
          <img src={remoteCover.src} alt="" class="w-full h-full object-cover" />
        </span>
      {/if}
      <span class="flex-1 min-w-0 text-[12px] truncate
                   text-neutral-500 dark:text-neutral-400"
            title={pair.track.title}>
        <span class="tabular-nums text-neutral-400 dark:text-neutral-500">
          {pair.track.position}.
        </span>
        {pair.track.title}
        <span class="text-[10px] text-neutral-400 dark:text-neutral-600">
          {formatDuration(pair.track.duration)}
        </span>
      </span>
    </span>

    <!-- Appariement douteux : signalé ici seulement, la ligne à relire avant de cocher. -->
    {#if pair.doubtful}
      <span title={$t('source.doubtful')} class="shrink-0 flex">
        <Icon icon="lucide:circle-help" width="12"
              class="text-amber-600 dark:text-amber-400" />
      </span>
    {/if}

    <span class="shrink-0 px-1.5 rounded text-[10px] tabular-nums
                 {count > 0
                   ? 'bg-emerald-500/15 text-emerald-600 dark:text-emerald-400'
                   : 'bg-neutral-200/70 dark:bg-white/10 text-neutral-400 dark:text-neutral-500'}">
      {#if keys.length === 0}
        {$t('source.identical_short')}
      {:else}
        {count}/{keys.length}
      {/if}
    </span>

    <button
      type="button"
      onclick={() => selection.toggleOpen(pair.index)}
      aria-label={$t('source.detail')}
      title={$t('source.detail')}
      class="shrink-0 w-6 h-6 rounded-md flex items-center justify-center
             cursor-pointer transition-colors
             text-neutral-400 dark:text-neutral-500
             hover:text-neutral-800 dark:hover:text-neutral-100
             hover:bg-neutral-200/70 dark:hover:bg-white/8"
    >
      <Icon icon={open ? 'lucide:chevron-up' : 'lucide:chevron-down'} width="13" />
    </button>
  </div>

  {#if open}
    <div class="px-2.5 pb-2" transition:slide={{ duration: 140 }}>
      <!-- Colonnes nommées : en comparant, il faut savoir laquelle est la sienne. -->
      <div class="flex items-center gap-2.5 px-2 pb-1">
        <span class="shrink-0 w-4"></span>
        <span class="shrink-0 w-24"></span>
        <span class="flex-1 min-w-0 text-[9.5px] font-semibold uppercase
                     tracking-widest text-neutral-400 dark:text-neutral-500">
          {$t('source.your_file')}
        </span>
        <span class="shrink-0 w-2.5"></span>
        <span class="flex-1 min-w-0 text-[9.5px] font-semibold uppercase
                     tracking-widest text-emerald-600/70 dark:text-emerald-400/70">
          Deezer
        </span>
      </div>

      <div class="space-y-px">
        <!-- La pochette d'abord : la seule ligne qu'on juge à l'œil. -->
        {#if remoteCover}
          {@const on = chosen.has(coverKey(pair.file.path))}
          <div class="flex items-center gap-2.5 px-2 py-1 rounded-md
                      {on ? 'bg-emerald-500/10' : ''}">
            <input
              type="checkbox"
              checked={on}
              onchange={() => selection.toggleKeys([coverKey(pair.file.path)])}
              aria-label={$t('tags.cover')}
              class="checkbox-app"
            />
            <span class="shrink-0 w-24 text-[11px] truncate
                         text-neutral-500 dark:text-neutral-400">
              {$t('tags.cover')}
            </span>

            <span class="flex-1 min-w-0 flex items-center gap-2">
              <span class="shrink-0 w-9 h-9 rounded overflow-hidden
                           ring-1 ring-black/5 dark:ring-white/10
                           bg-neutral-200/60 dark:bg-white/5">
                {#if pair.file.cover}
                  <img src={pair.file.cover} alt=""
                       class="w-full h-full object-cover" />
                {/if}
              </span>
              <span class="min-w-0 text-[11px] truncate
                           {pair.file.cover
                             ? 'text-neutral-500 dark:text-neutral-400'
                             : 'italic text-neutral-400 dark:text-neutral-600'}">
                {#if !pair.file.cover}
                  {$t('tags.no_media')}
                {:else if on}
                  <!-- Seul geste destructeur de l'écran : il se dit avant. -->
                  <span class="text-amber-600 dark:text-amber-400">
                    {$t('source.cover_replaced')}
                  </span>
                {/if}
              </span>
            </span>

            <Icon icon="lucide:arrow-right" width="10"
                  class="shrink-0 text-neutral-300 dark:text-neutral-600" />

            <span class="flex-1 min-w-0 flex items-center gap-2">
              <span class="shrink-0 w-9 h-9 rounded overflow-hidden
                           ring-1 {on
                             ? 'ring-emerald-500/60'
                             : 'ring-black/5 dark:ring-white/10'}">
                <img src={remoteCover.src} alt=""
                     class="w-full h-full object-cover" />
              </span>
              <!-- Dimensions et poids après préparation : ce qui sera écrit. -->
              <span class="min-w-0 text-[10.5px] leading-tight truncate
                           text-neutral-500 dark:text-neutral-400">
                {remoteCover.width}×{remoteCover.height}<br />
                {formatBytes(remoteCover.bytes)}
              </span>
            </span>
          </div>
        {/if}

        {#each pair.cells as cell (cell.field)}
          <SourceCellRow
            {cell}
            on={chosen.has(cellKey(pair.file.path, cell.field))}
            ontoggle={() => selection.toggleCell(pair.file.path, cell.field)}
          />
        {/each}
      </div>
    </div>
  {/if}
</div>
