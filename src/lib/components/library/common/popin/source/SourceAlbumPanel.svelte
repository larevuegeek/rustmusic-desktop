<script lang="ts">
  // Colonne de la source : recherche d'album, puis l'album retenu et ses raccourcis par champ.
  import Icon from "@iconify/svelte";
  import { t } from "#lib/i18n";
  import type { AlbumHit } from "#lib/services/tags/metadata.service";
  import type { SourceSelection } from "./sourceSelection.svelte.js";

  let {
    selection,
    query = $bindable(),
    hits,
    searching,
    matching,
    searched,
    onsearch,
    onpick,
  }: {
    selection: SourceSelection;
    query: string;
    hits: AlbumHit[];
    searching: boolean;
    matching: boolean;
    searched: boolean;
    onsearch: () => void;
    onpick: (hit: AlbumHit) => void;
  } = $props();

  let proposal = $derived(selection.proposal);
  let targets = $derived(selection.targets);
</script>

<aside
  class="shrink-0 w-64 flex flex-col min-h-0 overflow-y-auto scrollbar-none
         border-r border-neutral-200/70 dark:border-white/8
         bg-neutral-50/70 dark:bg-black/20"
>
  {#if !proposal}
    <div class="shrink-0 p-4 space-y-2">
      <div class="flex gap-1.5">
        <input
          type="text"
          bind:value={query}
          onkeydown={(e) => e.key === 'Enter' && onsearch()}
          placeholder={$t('source.search_placeholder')}
          class="flex-1 min-w-0 h-8 px-2.5 rounded-lg text-[12px] outline-none
                 bg-white dark:bg-white/5
                 ring-1 ring-inset ring-neutral-200 dark:ring-white/10
                 focus:ring-emerald-500/60
                 text-neutral-900 dark:text-neutral-100
                 placeholder:text-neutral-400 dark:placeholder:text-neutral-600"
        />
        <button
          type="button"
          onclick={() => onsearch()}
          disabled={searching || !query.trim()}
          aria-label={$t('source.search')}
          class="shrink-0 w-8 h-8 rounded-lg flex items-center justify-center
                 cursor-pointer transition-colors disabled:opacity-40
                 bg-emerald-500 text-white hover:bg-emerald-600"
        >
          <Icon icon={searching ? 'lucide:loader-circle' : 'lucide:search'}
                width="13" class={searching ? 'animate-spin' : ''} />
        </button>
      </div>
      <p class="text-[10.5px] text-neutral-400 dark:text-neutral-500">
        {targets.length} {$t('tags.files')}
      </p>
    </div>

    <div class="flex-1 min-h-0 px-2 pb-3 space-y-0.5">
      {#each hits as hit (hit.id)}
        <button
          type="button"
          onclick={() => onpick(hit)}
          disabled={matching}
          class="w-full flex items-center gap-2.5 p-1.5 rounded-lg text-left
                 cursor-pointer transition-colors disabled:opacity-50
                 hover:bg-neutral-200/60 dark:hover:bg-white/6"
        >
          <span class="shrink-0 w-10 h-10 rounded overflow-hidden
                       bg-neutral-200 dark:bg-white/5">
            {#if hit.cover}
              <img src={hit.cover} alt="" class="w-full h-full object-cover" />
            {/if}
          </span>
          <span class="min-w-0 flex-1">
            <span class="block text-[12px] font-medium truncate
                         text-neutral-800 dark:text-neutral-100">{hit.title}</span>
            <!-- Le nombre de pistes : premier indice qu'on tient le bon album. -->
            <span class="block text-[10.5px] truncate
                         text-neutral-400 dark:text-neutral-500">
              {hit.artist} · {hit.track_count} {$t('source.tracks')}
              {#if hit.track_count === targets.length}
                <span class="text-emerald-600 dark:text-emerald-400">✓</span>
              {/if}
            </span>
          </span>
        </button>
      {:else}
        {#if searched && !searching}
          <p class="px-2 py-8 text-center text-[12px]
                    text-neutral-400 dark:text-neutral-500">
            {$t('source.no_result')}
          </p>
        {/if}
      {/each}
    </div>
  {:else}
    <!-- L'album retenu en grand : la pochette dit d'un coup d'œil si c'est le bon disque. -->
    <div class="shrink-0 p-4 space-y-3">
      <div class="aspect-square rounded-xl overflow-hidden
                  ring-1 ring-black/5 dark:ring-white/10
                  shadow-lg shadow-black/10 dark:shadow-black/50
                  bg-neutral-200/60 dark:bg-white/5">
        {#if proposal.album.cover}
          <img src={proposal.album.cover} alt="" class="w-full h-full object-cover" />
        {/if}
      </div>

      <div>
        <p class="text-[13px] font-medium leading-tight
                  text-neutral-800 dark:text-neutral-100">
          {proposal.album.title}
        </p>
        <p class="mt-0.5 text-[11.5px] text-neutral-500 dark:text-neutral-400">
          {proposal.album.artist}
        </p>
        <div class="mt-1.5 flex flex-wrap gap-1">
          {#each [proposal.album.year, proposal.album.genre].filter(Boolean) as badge (badge)}
            <span class="px-1.5 py-0.5 rounded text-[10px]
                         bg-neutral-200/70 dark:bg-white/10
                         text-neutral-500 dark:text-neutral-400">{badge}</span>
          {/each}
          <!-- Pistes / fichiers : c'est ce badge qui dit si on a désigné le bon disque. -->
          <span
            title="{proposal.album.tracks.length} {$t('source.tracks')} · {targets.length} {$t('tags.files')}"
            class="flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] tabular-nums
                   {proposal.album.tracks.length === targets.length
                     ? 'bg-emerald-500/15 text-emerald-600 dark:text-emerald-400'
                     : 'bg-amber-500/15 text-amber-600 dark:text-amber-400'}"
          >
            <Icon
              icon={proposal.album.tracks.length === targets.length
                ? 'lucide:check'
                : 'lucide:triangle-alert'}
              width="9"
            />
            {proposal.album.tracks.length} / {targets.length}
          </span>
        </div>
      </div>

      <button
        type="button"
        onclick={() => (selection.proposal = null)}
        class="w-full flex items-center justify-center gap-1.5 px-2 h-7 rounded-lg
               text-[11.5px] cursor-pointer transition-colors
               text-neutral-500 dark:text-neutral-400
               hover:bg-neutral-200/70 dark:hover:bg-white/8"
      >
        <Icon icon="lucide:repeat" width="12" />
        {$t('source.change')}
      </button>
    </div>

    <!-- Raccourcis par champ : « toutes les années » d'un clic, sans parcourir les lignes. -->
    {#if selection.fieldSummary.length > 0}
      <div class="shrink-0 mt-auto px-4 py-3 space-y-2
                  border-t border-neutral-200/70 dark:border-white/8">
        <p class="text-[10px] font-semibold uppercase tracking-widest
                  text-neutral-400 dark:text-neutral-500">
          {$t('source.by_field')}
        </p>
        <div class="flex flex-wrap gap-1">
          {#each selection.fieldSummary as summary (summary.field)}
            {@const full = summary.picked === summary.total}
            {@const partial = summary.picked > 0 && !full}
            <button
              type="button"
              onclick={() => selection.toggleField(summary.field)}
              title="{summary.picked} / {summary.total}"
              class="flex items-center gap-1 px-1.5 h-6 rounded-md text-[10.5px]
                     cursor-pointer transition-colors
                     {full
                       ? 'bg-emerald-500/18 text-emerald-600 dark:text-emerald-400'
                       : partial
                         ? 'bg-emerald-500/8 text-emerald-600/80 dark:text-emerald-400/80'
                         : 'bg-neutral-200/60 dark:bg-white/6 text-neutral-400 dark:text-neutral-500'}"
            >
              {$t(summary.label)}
              <span class="tabular-nums opacity-70">
                {partial ? `${summary.picked}/${summary.total}` : summary.total}
              </span>
            </button>
          {/each}
        </div>

        <p class="text-[10px] leading-relaxed text-neutral-400 dark:text-neutral-500">
          {$t('source.limits')}
        </p>
      </div>
    {/if}
  {/if}
</aside>
