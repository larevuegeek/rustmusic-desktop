<script lang="ts">
  // Entre et sort du mode sélection, toujours avec son libellé.
  import { t } from "#lib/i18n";
  import { selectionStore } from "#lib/stores/ui/selection.store";
  import ToolbarButton from "#lib/components/ui/button/ToolbarButton.svelte";
</script>

<ToolbarButton icon="material-symbols:select-check-box-rounded" pressed={$selectionStore.active}
               label={$t("albums_view.select")} title={$t("albums_view.select")}
               onclick={() => ($selectionStore.active ? selectionStore.stop() : selectionStore.start())} />
