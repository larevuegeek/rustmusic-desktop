<script lang="ts">
  // Entre et sort du mode sélection, toujours avec son libellé ; en sélection, tout cocher ou tout décocher.
  import { t } from "#lib/i18n";
  import { selectionStore } from "#lib/stores/ui/selection.store";
  import ToolbarButton from "#lib/components/ui/button/ToolbarButton.svelte";

  const canSelectAll = selectionStore.canSelectAll;
  const anySelected = $derived($selectionStore.count > 0);
</script>

{#if $selectionStore.active && (anySelected || $canSelectAll)}
  <ToolbarButton icon={anySelected ? "material-symbols:deselect-rounded" : "material-symbols:select-all-rounded"}
                 label={$t(anySelected ? "actions.deselect_all" : "actions.select_all")}
                 onclick={() => (anySelected ? selectionStore.deselectAll() : selectionStore.selectEverything())} />
{/if}

<ToolbarButton icon="material-symbols:select-check-box-rounded" pressed={$selectionStore.active}
               label={$t("albums_view.select")} title={$t("albums_view.select")}
               onclick={() => ($selectionStore.active ? selectionStore.stop() : selectionStore.start())} />
