<script lang="ts">
import type { Snippet } from "svelte";

let { titre, actions }: { titre: string; actions?: Snippet } = $props();
</script>

<div class="flex items-center justify-between gap-2 pt-3 pb-1 px-1">
  <h3 class="text-xs font-bold uppercase tracking-[.08em] truncate text-(--sb-hd)">{titre}</h3>
  {#if actions}
    <div class="flex gap-0.5">{@render actions()}</div>
  {/if}
</div>
