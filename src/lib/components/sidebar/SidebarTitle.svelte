<script lang="ts">
import type { Snippet } from "svelte";

let { titre, actions, onclick, aide }: { titre: string; actions?: Snippet; onclick?: () => void; aide?: string } = $props();
</script>

<div class="flex items-center justify-between gap-2 pt-3 pb-1 px-1">
  {#if onclick}
    <!-- Titre cliquable : il mène à la page de la section. -->
    <h3 class="min-w-0">
      <button type="button" class="group max-w-full flex items-center gap-0.5 cursor-pointer text-xs font-bold uppercase tracking-[.08em] text-(--sb-hd) hover:text-(--sb-g)" title={aide} {onclick}>
        <span class="truncate">{titre}</span>
        <span class="opacity-0 -translate-x-0.5 transition group-hover:opacity-100 group-hover:translate-x-0" aria-hidden="true">›</span>
      </button>
    </h3>
  {:else}
    <h3 class="text-xs font-bold uppercase tracking-[.08em] truncate text-(--sb-hd)">{titre}</h3>
  {/if}
  {#if actions}
    <div class="flex gap-0.5">{@render actions()}</div>
  {/if}
</div>
