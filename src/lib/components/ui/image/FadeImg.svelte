<script lang="ts">
  import type { HTMLImgAttributes } from "svelte/elements";
  let { src, alt = "", class: classe = "", ...rest }: { src: string; alt?: string } & Omit<HTMLImgAttributes, "src" | "alt"> = $props();
  let loaded = $state(false);
  let errored = $state(false);
</script>

<!-- `rest` d'abord : sinon sa classe écrasait l'opacité, et une image introuvable montrait l'icône cassée. -->
<img
  {...rest}
  {src}
  {alt}
  loading="lazy"
  onload={() => { loaded = true; errored = false; }}
  onerror={() => errored = true}
  class="{classe} transition-opacity duration-300 {loaded && !errored ? 'opacity-100' : 'opacity-0'}"
/>
