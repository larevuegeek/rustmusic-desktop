<script lang="ts">
  import Icon from "@iconify/svelte";
  import { goto } from "$app/navigation";
  import { t } from "#lib/i18n";
  import { podcastStore } from "#lib/stores/podcast/podcast.store";
  import { popinStore } from "#lib/stores/ui/popin.store";
  import { toasts } from "#lib/stores/ui/toast.store";
  import { portal } from "#lib/helper/portal";
  import PodcastPreviewPopin from "#lib/components/podcast/PodcastPreviewPopin.svelte";
  import type { DirectoryResult } from "#lib/types/ui/podcast/Podcast";

  /** Les actions d'une émission (carte de l'annuaire ou abonnement), au « … » ou au clic droit. */
  let { x, y, top = y, podcast, onclose }: {
    x: number;
    y: number;
    /** Bord haut de l'ancre : le menu s'ouvre au-dessus quand la place manque en bas. */
    top?: number;
    podcast: DirectoryResult;
    onclose: () => void;
  } = $props();

  const suivi = $derived($podcastStore.podcasts.find((p) => p.feed_url === podcast.feed_url) ?? null);
  let confirmer = $state(false);

  let hauteur = $state(0);
  let menuStyle = $derived.by(() => {
    const largeur = 230;
    const posX = Math.max(8, Math.min(x, window.innerWidth - largeur - 8));
    const posY = y + hauteur > window.innerHeight - 8 ? Math.max(8, top - hauteur) : y;
    return `left: ${posX}px; top: ${posY}px;`;
  });

  function preview() {
    onclose();
    popinStore.open(podcast.title, PodcastPreviewPopin, { podcast }, { size: "lg", bare: true });
  }

  function open() {
    onclose();
    if (suivi) goto(`/podcasts/${suivi.id}`);
  }

  async function follow() {
    onclose();
    try {
      const p = await podcastStore.follow(podcast.feed_url);
      toasts.push({ type: "success", title: $t("podcasts.followed").replace("{title}", p.title) });
    } catch (err) {
      toasts.push({ type: "error", title: $t("podcasts.error_follow"), message: String(err) });
    }
  }

  async function togglePinned() {
    onclose();
    if (!suivi) return;
    try {
      await podcastStore.setPinned(suivi.id, !suivi.pinned);
    } catch (err) {
      toasts.push({ type: "error", title: $t("notify.error"), message: String(err) });
    }
  }

  async function unfollow() {
    if (!confirmer) {
      confirmer = true;
      return;
    }
    onclose();
    if (!suivi) return;
    try {
      await podcastStore.unfollow(suivi.id);
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
  <button type="button" role="menuitem" class={ligne} onclick={preview}>
    <Icon icon="material-symbols:visibility-outline-rounded" width="17" class="opacity-60" />
    {$t("podcasts.quick_preview")}
  </button>

  {#if suivi}
    <button type="button" role="menuitem" class={ligne} onclick={open}>
      <Icon icon="material-symbols:arrow-forward-rounded" width="17" class="opacity-60" />
      {$t("podcasts.open")}
    </button>
    <button type="button" role="menuitem" class={ligne} onclick={togglePinned}>
      <Icon icon={suivi.pinned ? "material-symbols:star-rounded" : "material-symbols:star-outline-rounded"} width="17"
            class={suivi.pinned ? "text-amber-500 dark:text-amber-300" : "opacity-60"} />
      {suivi.pinned ? $t("podcasts.unfavorite") : $t("podcasts.favorite")}
    </button>
    <div class="h-px mx-2 my-1 bg-(--rg-bd) dark:bg-white/8"></div>
    <!-- Deux temps : le premier clic demande confirmation. -->
    <button type="button" role="menuitem" class="{ligne} text-red-600! dark:text-red-400! hover:bg-red-500/10!" onclick={unfollow}>
      <Icon icon="material-symbols:do-not-disturb-on-outline-rounded" width="17" class="opacity-70" />
      {confirmer ? $t("podcasts.unsubscribe_sure") : $t("podcasts.unsubscribe")}
    </button>
  {:else}
    <button type="button" role="menuitem" class="{ligne} hover:text-violet-600 dark:hover:text-violet-300" onclick={follow}>
      <Icon icon="material-symbols:add-rounded" width="17" class="opacity-60" />
      {$t("podcasts.follow")}
    </button>
  {/if}
</div>
</div>
