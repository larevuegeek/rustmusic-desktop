<script lang="ts">
  import type { HTMLImgAttributes } from "svelte/elements";
  let { src, alt = "", ...rest }: { src: string; alt?: string } & Omit<HTMLImgAttributes, "src" | "alt"> = $props();
  let loaded = $state(false);
  let errored = $state(false);
</script>

<img
  {src}
  {alt}
  loading="lazy"
  onload={() => { loaded = true; errored = false; }}
  onerror={() => errored = true}
  class="{rest.class ?? ''} transition-opacity duration-300 {loaded && !errored ? 'opacity-100' : 'opacity-0'}"
  {...rest}
/>
