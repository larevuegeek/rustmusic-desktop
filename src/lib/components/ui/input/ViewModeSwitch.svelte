<script lang="ts">
  // Grille ou liste, partagé par toutes les vues (store `viewMode`).
  import { t } from "#lib/i18n";
  import { viewMode, type ViewMode } from "#lib/stores/ui/viewMode.store";
  import SegmentedControl from "./SegmentedControl.svelte";

  let { class: classes = "" }: { class?: string } = $props();
</script>

<SegmentedControl variant="discret" class={classes} label={$t("albums_view.grid") + " / " + $t("albums_view.list")} value={$viewMode}
                  options={[
                    { value: "grid", label: $t("albums_view.grid"), icon: "material-symbols:grid-view-outline-rounded", iconOnly: true },
                    { value: "list", label: $t("albums_view.list"), icon: "material-symbols:view-list-outline-rounded", iconOnly: true },
                  ]}
                  onchange={(v) => viewMode.set(v as ViewMode)} />
