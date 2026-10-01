<script lang="ts">
  // Colonne de l'éditeur par lot : pochette commune, fichiers visés et numérotation.
  // Purement affichage : la pochette et la numérotation appartiennent au parent.
  import Icon from "@iconify/svelte";
  import { t } from "$lib/i18n";
  import { fileName } from "$lib/services/batch/batch.service";
  import { formatBytes, type PreparedImage } from "$lib/services/tags/tagEditor.service";

  let {
    cover,
    coverBusy,
    submitting,
    paths,
    numbering,
    onpickcover,
    onremovecover,
    ontogglenumbering,
  }: {
    cover: (PreparedImage & { path: string }) | null;
    coverBusy: boolean;
    submitting: boolean;
    paths: string[];
    numbering: boolean;
    onpickcover: () => void;
    onremovecover: () => void;
    ontogglenumbering: () => void;
  } = $props();
</script>

<!-- Pochette commune : remplace celle de chaque fichier, laisse ses autres images. -->
<div
  class="relative group aspect-square rounded-xl overflow-hidden shrink-0
         ring-1 ring-black/5 dark:ring-white/10
         bg-neutral-200/60 dark:bg-white/5"
>
  {#if cover}
    <img src={cover.src} alt="" class="w-full h-full object-cover" />
    <span
      class="absolute top-2 left-2 pointer-events-none
             flex items-center gap-1 px-1.5 py-0.5 rounded-md
             text-[10px] font-bold uppercase tracking-wider
             bg-emerald-500 text-white shadow-md shadow-black/30"
    >
      <Icon icon="lucide:star" width="9" />
      {$t('tags.cover')}
    </span>
    <button
      type="button"
      onclick={onremovecover}
      title={$t('tags.remove_image')}
      aria-label={$t('tags.remove_image')}
      class="absolute top-2 right-2 w-6 h-6 rounded-md cursor-pointer
             flex items-center justify-center
             bg-black/70 text-white hover:bg-red-500 transition-colors"
    >
      <Icon icon="lucide:x" width="12" />
    </button>
  {:else}
    <button
      type="button"
      onclick={onpickcover}
      disabled={coverBusy || submitting}
      class="w-full h-full flex flex-col items-center justify-center gap-2 px-3
             cursor-pointer transition-colors text-center
             text-neutral-400 dark:text-neutral-500
             hover:text-emerald-600 dark:hover:text-emerald-400
             hover:bg-emerald-500/8 disabled:opacity-50"
    >
      <Icon
        icon={coverBusy ? 'lucide:loader-circle' : 'lucide:image-plus'}
        width="26"
        class={coverBusy ? 'animate-spin' : ''}
      />
      <span class="text-[11px] font-medium leading-tight">
        {$t('tags.common_cover')}
      </span>
    </button>
  {/if}
</div>

{#if cover}
  <p class="shrink-0 text-[10px] text-neutral-400 dark:text-neutral-500">
    {cover.mime_type.replace('image/', '').toUpperCase()} · {formatBytes(cover.bytes)}
    {#if cover.recompressed}
      <span class="text-amber-500">· {$t('tags.reduced')}</span>
    {/if}
    <br />
    {$t('tags.cover_keeps_others')}
  </p>
{/if}

<!-- Ce sur quoi on agit, à vérifier avant d'écrire : un lot mal ciblé ne se rattrape pas. -->
<div class="shrink-0">
  <p class="mb-1.5 flex items-center gap-1.5 text-[10px] font-semibold uppercase
            tracking-[0.09em] text-neutral-400 dark:text-neutral-500">
    {$t('tags.selection')}
    <span class="px-1 rounded bg-neutral-200/70 dark:bg-white/10
                 text-neutral-500 dark:text-neutral-400 tabular-nums">
      {paths.length}
    </span>
  </p>
  <div class="max-h-52 overflow-y-auto scrollbar-none space-y-0.5">
    {#each paths as path, index (path)}
      <p
        class="flex items-baseline gap-1.5 px-2 py-1 rounded text-[11px]
               text-neutral-600 dark:text-neutral-300
               hover:bg-neutral-200/50 dark:hover:bg-white/5"
        title={path}
      >
        {#if numbering}
          <span class="shrink-0 w-4 text-right tabular-nums font-medium
                       text-emerald-600 dark:text-emerald-400">
            {index + 1}
          </span>
        {/if}
        <span class="min-w-0 truncate">{fileName(path)}</span>
      </p>
    {/each}
  </div>

  <button
    type="button"
    onclick={ontogglenumbering}
    disabled={submitting}
    class="mt-2 w-full flex items-center justify-center gap-1.5 px-2 h-7 rounded-lg
           text-[11px] font-medium cursor-pointer transition-colors
           disabled:opacity-40
           {numbering
             ? 'bg-emerald-500/15 text-emerald-600 dark:text-emerald-400'
             : 'text-neutral-500 dark:text-neutral-400 hover:bg-neutral-200/60 dark:hover:bg-white/6'}"
  >
    <Icon icon={numbering ? 'lucide:check' : 'lucide:list-ordered'} width="12" />
    {$t('tags.number_tracks')}
  </button>
</div>
