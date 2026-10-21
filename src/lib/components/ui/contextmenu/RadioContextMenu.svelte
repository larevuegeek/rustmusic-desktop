<script lang="ts">
  import Icon from "@iconify/svelte";
  import { t } from "#lib/i18n";
  import { radioStore, findFavorite } from "#lib/stores/radio/radio.store";
  import { toasts } from "#lib/stores/ui/toast.store";
  import { portal } from "#lib/helper/portal";
  import { isWebUrl } from "#lib/helper/tools/safeHtml";
  import type { RadioResult } from "#lib/types/ui/radio/Radio";

  /** Les actions d'une station (annuaire ou favorite), au « … » ou au clic droit. */
  let { x, y, top = y, station, onplay, onclose }: {
    x: number;
    y: number;
    /** Bord haut de l'ancre : le menu s'ouvre au-dessus quand la place manque en bas. */
    top?: number;
    station: RadioResult;
    onplay: () => void;
    onclose: () => void;
  } = $props();

  const favorite = $derived(findFavorite($radioStore.radios, station));

  let hauteur = $state(0);
  let menuStyle = $derived.by(() => {
    const largeur = 230;
    const posX = Math.max(8, Math.min(x, window.innerWidth - largeur - 8));
    const posY = y + hauteur > window.innerHeight - 8 ? Math.max(8, top - hauteur) : y;
    return `left: ${posX}px; top: ${posY}px;`;
  });

  function play() {
    onclose();
    onplay();
  }

  async function toggleFavorite() {
    onclose();
    try {
      if (favorite) await radioStore.remove(favorite.id);
      else await radioStore.add(station);
    } catch (err) {
      toasts.push({ type: "error", title: $t("notify.error"), message: String(err) });
    }
  }

  async function copyStream() {
    onclose();
    try {
      await navigator.clipboard.writeText(station.stream_url);
      toasts.push({ type: "success", title: $t("radios.stream_copied") });
    } catch (err) {
      toasts.push({ type: "error", title: $t("notify.error"), message: String(err) });
    }
  }

  const ligne = "w-full flex items-center gap-2.5 px-3.5 py-2 text-sm text-left cursor-pointer transition-colors text-(--rg-tx) hover:bg-(--rg-s2) dark:hover:bg-[#252b28]";
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && onclose()} />

<!-- Porté vers `body` : une carte de carrousel rognerait le menu. -->
<div use:portal>
<button
  type="button"
  class="fixed inset-0 z-9998 cursor-default"
  onclick={onclose}
  oncontextmenu={(e) => { e.preventDefault(); onclose(); }}
  aria-label={$t("menu.close_menu")}
></button>

<div
  role="menu"
  tabindex="-1"
  oncontextmenu={(e) => e.preventDefault()}
  class="fixed z-[9999] w-57.5 py-1.5
         bg-(--rg-carte) dark:bg-(--rg-s2)
         border border-(--rg-bd) dark:border-[#2a312d]
         rounded-xl shadow-[0_18px_40px_rgba(0,0,0,0.18)] dark:shadow-[0_18px_40px_rgba(0,0,0,0.6)]
         overflow-hidden"
  style={menuStyle}
  bind:clientHeight={hauteur}
>
  <button type="button" role="menuitem" class="{ligne} hover:text-rose-600 dark:hover:text-rose-300" onclick={play}>
    <Icon icon="material-symbols:play-arrow-rounded" width="18" class="opacity-60" />
    {$t("radios.listen")}
  </button>
  <button type="button" role="menuitem" class={ligne} onclick={toggleFavorite}>
    <Icon icon={favorite ? "material-symbols:star-rounded" : "material-symbols:star-outline-rounded"} width="17"
          class={favorite ? "text-amber-500 dark:text-amber-300" : "opacity-60"} />
    {favorite ? $t("radios.unfavorite") : $t("radios.favorite")}
  </button>
  <div class="h-px mx-2 my-1 bg-(--rg-bd) dark:bg-white/8"></div>
  {#if isWebUrl(station.homepage)}
    <a href={station.homepage} role="menuitem" class={ligne} onclick={onclose}>
      <Icon icon="material-symbols:language" width="17" class="opacity-60" />
      {$t("radios.website")}
    </a>
  {/if}
  <button type="button" role="menuitem" class={ligne} onclick={copyStream}>
    <Icon icon="material-symbols:content-copy-outline-rounded" width="16" class="opacity-60" />
    {$t("radios.copy_stream")}
  </button>
</div>
</div>
