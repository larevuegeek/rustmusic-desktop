<script lang="ts">
  // Bouton « Colonnes » et sa popin : les colonnes et largeurs du tableau des titres (réglages partagés).
  import { t } from "#lib/i18n";
  import { settingsStore } from "#lib/stores/settings/settings.store";
  import { clesAffichees } from "#lib/config/trackColumns";
  import ToolbarButton from "#lib/components/ui/button/ToolbarButton.svelte";
  import TrackColumnsPopin from "#lib/components/library/common/popin/TrackColumnsPopin.svelte";

  let { libraryId, avecLibelle = false }: { libraryId: number | null; avecLibelle?: boolean } = $props();

  let ouverte = $state(false);
</script>

<ToolbarButton icon="material-symbols:view-column-outline-rounded" label={avecLibelle ? $t("album_view.columns") : undefined}
               title={$t("tracks_view.columns")} onclick={() => (ouverte = true)} />

{#if ouverte}
  <TrackColumnsPopin
    bind:open={ouverte}
    {libraryId}
    colonnes={clesAffichees($settingsStore.track_columns)}
    onchange={(c) => settingsStore.set("track_columns", JSON.stringify(c))}
    onresetwidths={() => settingsStore.set("track_column_widths", "{}")}
  />
{/if}
