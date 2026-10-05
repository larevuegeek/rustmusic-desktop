<script lang="ts">
  // Récupération Deezer : recherche, puis comparaison avec la piste retenue.
  // Pleine largeur : la comparaison montre deux colonnes de valeurs et deux pochettes.
  import Icon from "@iconify/svelte";
  import { t } from "#lib/i18n";
  import TrackLookup from "#lib/components/ui/deezer/TrackLookup.svelte";
  import TrackCompare from "#lib/components/ui/deezer/TrackCompare.svelte";
  import type { TrackHit, TrackValues } from "#lib/services/tags/metadata.service";
  import type { DownloadedImage, MediaSlot } from "#lib/services/tags/tagEditor.service";

  let {
    form,
    filename,
    coverSlot,
    picked = $bindable(),
    onclose,
    onapply,
  }: {
    form: Record<string, string>;
    filename: string;
    coverSlot: MediaSlot | null;
    /** Piste retenue, comparée aux valeurs actuelles ; `null` = recherche. */
    picked: { values: TrackValues; hit: TrackHit } | null;
    onclose: () => void;
    onapply: (fields: Record<string, string>, cover: DownloadedImage | null) => void;
  } = $props();

  // Amorce : artiste et titre actuels, à défaut le nom du fichier.
  let lookupQuery = $derived(
    [form.artist, form.title].filter(Boolean).join(" ").trim() ||
      filename.replace(/\.[^.]+$/, "").replace(/[_-]+/g, " "),
  );
</script>

<div class="flex-1 min-h-0 flex flex-col">
  <div class="shrink-0 flex items-center gap-2.5 px-4 py-2.5
              border-b border-neutral-200/70 dark:border-white/8">
    <Icon icon="lucide:cloud-download" width="14"
          class="shrink-0 text-emerald-600 dark:text-emerald-400" />
    <span class="flex-1 min-w-0 text-[12.5px] font-medium truncate
                 text-neutral-800 dark:text-neutral-100">
      {$t('source.title')}
    </span>
    <button
      type="button"
      onclick={onclose}
      class="flex items-center gap-1.5 px-2 h-7 rounded-lg text-[11.5px]
             cursor-pointer transition-colors
             text-neutral-500 dark:text-neutral-400
             hover:bg-neutral-200/70 dark:hover:bg-white/8"
    >
      <Icon icon="lucide:x" width="12" />
      {$t('source.back_to_fields')}
    </button>
  </div>

  <div class="flex-1 min-h-0">
    {#if picked}
      <TrackCompare
        current={form}
        currentCover={coverSlot}
        values={picked.values}
        hit={picked.hit}
        onback={() => (picked = null)}
        {onapply}
      />
    {:else}
      <TrackLookup
        initialQuery={lookupQuery}
        onpick={(values, hit) => (picked = { values, hit })}
        oncancel={onclose}
      />
    {/if}
  </div>
</div>
