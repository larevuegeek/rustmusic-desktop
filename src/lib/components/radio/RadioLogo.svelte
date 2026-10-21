<script lang="ts">
  // Logo d'une station sur fond clair (les logos sont souvent sombres et transparents) ; à défaut, un dégradé et l'antenne.
  import Icon from "@iconify/svelte";
  import { resolveCoverSrc } from "#lib/helper/tools/coverHelper";
  import { whenNearViewport } from "#lib/helper/tools/nearViewport";
  import { radioStore } from "#lib/stores/radio/radio.store";

  let { path = null, favicon = null, icone = 40, class: classes = "" }: {
    /** Logo déjà en cache (favori). */
    path?: string | null;
    /** Adresse du logo dans l'annuaire : demandée au backend à l'approche de l'écran. */
    favicon?: string | null;
    icone?: number;
    class?: string;
  } = $props();

  let anchor = $state<HTMLElement | null>(null);
  let near = $state(false);
  $effect(() => {
    if (near || !anchor) return;
    return whenNearViewport(anchor, () => (near = true));
  });

  let src = $state<string | null>(null);
  let failed = $state(false);
  $effect(() => {
    const local = path;
    const remote = favicon;
    src = null;
    failed = false;
    if (!near) return;
    let alive = true;
    (local ? Promise.resolve(local) : radioStore.logo(remote))
      .then((p) => (p ? resolveCoverSrc(p, "full") : null))
      .then((s) => { if (alive) src = s; });
    return () => { alive = false; };
  });
</script>

<span bind:this={anchor} class="relative block overflow-hidden isolate {src && !failed ? 'bg-white' : 'bg-linear-140 from-rose-500 to-orange-500'} {classes}">
  {#if src && !failed}
    <img {src} alt="" loading="lazy" decoding="async" onerror={() => (failed = true)} class="absolute inset-[12%] w-[76%] h-[76%] object-contain" />
  {:else}
    <span class="absolute inset-0 flex items-center justify-center text-white/90">
      <Icon icon="material-symbols:radio-rounded" width={icone} />
    </span>
  {/if}
  <span class="absolute inset-0 ring-1 ring-inset ring-black/8 dark:ring-white/8 pointer-events-none" style="border-radius: inherit"></span>
</span>
