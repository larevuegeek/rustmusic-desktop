<script lang="ts">
  // Texte traduit dont les termes balisés « [[id|mot]] » ouvrent une bulle.
  import Term from "./Term.svelte";
  import { splitHelp } from "#lib/helper/tools/helpText";

  let { texte }: { texte: string } = $props();

  const morceaux = $derived(splitHelp(texte));
</script>

{#each morceaux as m}{#if m.terme}<Term id={m.terme}>{m.texte}</Term>{:else}{m.texte}{/if}{/each}
