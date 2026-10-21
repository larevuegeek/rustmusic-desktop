<script lang="ts">
  // Pochette d'un podcast ; à défaut, un dégradé violet et le micro.
  import Icon from "@iconify/svelte";
  import CoverImg from "#lib/components/ui/image/CoverImg.svelte";
  import type { CoverSize } from "#lib/helper/tools/coverHelper";

  let { path, size = "2x", icone = 40, class: classes = "" }: {
    path: string | null;
    size?: CoverSize;
    icone?: number;
    class?: string;
  } = $props();
</script>

<span class="relative block overflow-hidden isolate bg-linear-140 from-violet-500 to-violet-900 {classes}">
  {#if path}
    <CoverImg {path} alt="" {size} class="absolute inset-0 w-full h-full object-cover" />
  {:else}
    <span class="absolute inset-0 flex items-center justify-center text-white/90">
      <Icon icon="material-symbols:podcasts-rounded" width={icone} />
    </span>
  {/if}
  <span class="absolute inset-0 ring-1 ring-inset ring-black/8 dark:ring-white/8 pointer-events-none" style="border-radius: inherit"></span>
</span>
