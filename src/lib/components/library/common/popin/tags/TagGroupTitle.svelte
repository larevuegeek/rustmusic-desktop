<script lang="ts">
  // Titre d'un groupe de champs, prolongé d'un filet qui s'estompe.
  let { label }: { label: string } = $props();
</script>

<div class="flex items-center gap-2.5 mb-2">
  <span class="text-[10px] font-semibold uppercase tracking-[0.14em]
               text-neutral-400 dark:text-neutral-500">
    {label}
  </span>
  <span class="flex-1 h-px bg-linear-to-r
               from-neutral-200 to-transparent
               dark:from-white/10 dark:to-transparent"></span>
</div>
