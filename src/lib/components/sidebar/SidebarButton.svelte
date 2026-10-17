<script lang="ts">
import Icon from "@iconify/svelte";
import type { Snippet } from "svelte";

let { icon, label, onclick, taille = 20, class: classe = "", children }: {
  icon: string;
  taille?: number;
  label: string;
  onclick: (e: MouseEvent) => void;
  class?: string;
  children?: Snippet;
} = $props();
</script>

<button
  type="button"
  class="relative w-7.5 h-7.5 shrink-0 rounded-lg flex items-center justify-center cursor-pointer
         text-(--sb-mu) hover:bg-(--sb-s2) hover:text-(--sb-tx) transition-colors {classe}"
  {onclick}
  title={label}
  aria-label={label}
>
  <Icon {icon} width={taille} />
  {@render children?.()}
</button>
