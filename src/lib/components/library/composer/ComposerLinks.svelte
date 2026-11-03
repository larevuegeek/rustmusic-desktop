<script lang="ts">
  // Le tag compositeur en liens : un par compositeur, vers sa page.
  import { composerSplit, ensureComposerSplit, splitComposer } from "#lib/stores/library/composerSplit.store";
  import { composerHref } from "#lib/types/ui/library/composer/ComposerListView";

  let { libraryId, raw, class: cls = "" }: { libraryId: number; raw: string | null | undefined; class?: string } = $props();

  ensureComposerSplit();
  const noms = $derived(splitComposer(raw, $composerSplit));
</script>

<!-- Dans une ligne cliquable : le lien ne déclenche pas la ligne. -->
{#if noms.length}
  {#each noms as nom, i (nom)}{#if i > 0}{", "}{/if}<a href={composerHref(libraryId, nom)} class={cls}
     onclick={(e) => e.stopPropagation()} ondblclick={(e) => e.stopPropagation()}>{nom}</a>{/each}
{:else}{raw ?? ""}{/if}
