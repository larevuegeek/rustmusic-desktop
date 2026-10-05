<script lang="ts">
  // Entre et sort du mode sélection ; icône seule, ou avec libellé.
  import { t } from "#lib/i18n";
  import { selectionStore } from "#lib/stores/ui/selection.store";
  import ToolbarButton from "#lib/components/ui/button/ToolbarButton.svelte";

  let { avecLibelle = false }: { avecLibelle?: boolean } = $props();
</script>

<ToolbarButton icon="material-symbols:select-check-box-rounded" pressed={$selectionStore.active}
               label={avecLibelle ? $t("albums_view.select") : undefined} title={$t("albums_view.select")}
               onclick={() => ($selectionStore.active ? selectionStore.stop() : selectionStore.start())} />
