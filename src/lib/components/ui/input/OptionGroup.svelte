<script lang="ts">
  // Groupe d'options : intitulé en capitales, puis une carte de lignes `OptionItem`.
  import type { Snippet } from "svelte";

  let {
    title,
    hint,
    action,
    children,
  }: { title?: string; hint?: string; action?: Snippet; children: Snippet } = $props();
</script>

<section class="rg-groupe">
  {#if title}
    <div class="flex items-baseline gap-2.5 px-1 pb-2.5 min-w-0 leading-[1.2]">
      <h2 class="shrink-0 text-[13px] font-bold uppercase tracking-[0.08em] text-(--rg-tx2)">{title}</h2>
      {#if hint}<p class="flex-1 min-w-0 text-[13px] text-(--rg-mu2) truncate">{hint}</p>{/if}
      {#if action}<div class="self-center shrink-0 ml-auto">{@render action()}</div>{/if}
    </div>
  {/if}
  <div class="rg-carte rounded-[14px] bg-(--rg-carte) border border-(--rg-bd) overflow-hidden">
    {@render children()}
  </div>
</section>
