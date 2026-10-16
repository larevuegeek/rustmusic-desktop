<script lang="ts">
  // Case de la sélection multiple, la même partout ; `surImage` : posée sur une pochette.
  import Icon from "@iconify/svelte";

  let { coche, surImage = false, class: classes = "" }: { coche: boolean; surImage?: boolean; class?: string } = $props();
</script>

<span aria-hidden="true"
      class="shrink-0 border-2 flex items-center justify-center transition-colors
             {surImage ? 'w-6 h-6 rounded-[7px]' : 'w-5 h-5 rounded-md'}
             {coche ? 'bg-(--rg-g) border-(--rg-g) text-(--rg-on-g)'
                    : surImage ? 'bg-black/35 border-white/80 text-transparent' : 'bg-(--rg-carte) border-(--rg-bd2) text-transparent'} {classes}">
  <Icon icon="material-symbols:check-rounded" width={surImage ? 16 : 14} />
</span>
