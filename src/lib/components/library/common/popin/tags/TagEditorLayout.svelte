<script lang="ts">
  // Deux colonnes défilant chacune de son côté : barre latérale fixe, puis les champs.
  import type { Snippet } from "svelte";

  let { aside, children }: { aside: Snippet; children: Snippet } = $props();
</script>

<div class="flex-1 min-h-0 flex">
  <aside
    class="shrink-0 w-64 flex flex-col gap-3.5 p-4 overflow-y-auto scrollbar-none
           border-r border-neutral-200/70 dark:border-white/8
           bg-neutral-50/70 dark:bg-black/20"
  >
    {@render aside()}
  </aside>

  {@render children()}
</div>
