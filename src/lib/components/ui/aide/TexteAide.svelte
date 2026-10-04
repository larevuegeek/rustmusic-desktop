<script lang="ts">
  // Texte traduit dont les termes balisés « [[id|mot]] » ouvrent une bulle.
  import Terme from "./Terme.svelte";
  import { decouper } from "$lib/helper/tools/texteAide";

  let { texte }: { texte: string } = $props();

  const morceaux = $derived(decouper(texte));
</script>

{#each morceaux as m}{#if m.terme}<Terme id={m.terme}>{m.texte}</Terme>{:else}{m.texte}{/if}{/each}
