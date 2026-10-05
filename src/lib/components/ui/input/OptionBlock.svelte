<script lang="ts">
  // Bloc libre d'une carte `OptionGroup` (grille de choix…), masqué comme une ligne par la recherche.
  import type { Snippet } from "svelte";
  import { correspond, rechercheReglages } from "#lib/stores/ui/settingsSearch.store";

  let { keywords, class: classe = "", children }: { keywords: string; class?: string; children: Snippet } = $props();

  const visible = $derived(correspond($rechercheReglages, keywords));
</script>

<div data-reglage hidden={!visible} class={classe}>
  {@render children()}
</div>
