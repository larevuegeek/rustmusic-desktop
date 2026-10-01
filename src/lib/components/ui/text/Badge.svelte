<script lang="ts">
  // Étiquette courte : neutre, verte (état actif), ambrée (bêta), ou teintée (paliers de qualité).
  import type { Snippet } from "svelte";

  let { tone = "neutral", children }: { tone?: "neutral" | "green" | "amber" | "purple"; children: Snippet } = $props();

  const tons = {
    neutral: "bg-(--rg-s2) text-(--rg-mu) border-(--rg-bd2)",
    green: "bg-(--rg-gbg) text-(--rg-gtx) border-(--rg-gbd)",
    amber: "bg-(--rg-ambg) text-(--rg-am) border-(--rg-ambd)",
    purple: "bg-purple-500/12 text-purple-700 border-purple-500/30 dark:text-purple-300",
  };
</script>

<span class="shrink-0 inline-flex items-center px-1.75 py-0.5 rounded-[5px] border text-[11px] font-bold leading-[1.2] whitespace-nowrap {tons[tone]}">
  {@render children()}
</span>
