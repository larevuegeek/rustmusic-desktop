<script lang="ts">
  // Case de la sélection multiple, la même dans toutes les listes.
  import Icon from "@iconify/svelte";

  let { coche, class: classes = "" }: { coche: boolean; class?: string } = $props();
</script>

<span aria-hidden="true"
      class="w-5 h-5 shrink-0 rounded-md border-2 flex items-center justify-center transition-colors
             {coche ? 'bg-(--rg-g) border-(--rg-g) text-(--rg-on-g)' : 'bg-(--rg-carte) border-(--rg-bd2) text-transparent'} {classes}">
  <Icon icon="material-symbols:check-rounded" width="14" />
</span>
