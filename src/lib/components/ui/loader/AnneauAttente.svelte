<script lang="ts">
// Anneau tournant autour d'un bouton rond (le parent doit être `relative`).
// Il n'apparaît qu'après un court délai : un démarrage rapide ne clignote pas.
let { actif, delai = 250 }: { actif: boolean; delai?: number } = $props();

let visible = $state(false);
$effect(() => {
  if (!actif) { visible = false; return; }
  const id = setTimeout(() => (visible = true), delai);
  return () => clearTimeout(id);
});
</script>

{#if visible}
  <span aria-hidden="true"
        class="pointer-events-none absolute -inset-1 rounded-full border-2 border-(--lc-play)/20 border-t-(--lc-play) animate-spin"></span>
{/if}
