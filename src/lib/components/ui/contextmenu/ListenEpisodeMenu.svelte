<script lang="ts">
  // Menu d'une carte « À écouter » : retirer l'émission de la liste, marquer l'épisode écouté, ouvrir l'émission.
  import Icon from "@iconify/svelte";
  import { goto } from "$app/navigation";
  import { invoke } from "@tauri-apps/api/core";
  import { t } from "#lib/i18n";
  import { toasts } from "#lib/stores/ui/toast.store";
  import { portal } from "#lib/helper/portal";
  import type { LatestEpisode } from "#lib/types/ui/podcast/Podcast";

  let { x, y, top = y, episode, onclose, onchange }: {
    x: number;
    y: number;
    /** Bord haut de l'ancre : le menu s'ouvre au-dessus quand la place manque en bas. */
    top?: number;
    episode: LatestEpisode;
    onclose: () => void;
    /** La liste a changé : à recharger. */
    onchange: () => void;
  } = $props();

  let hauteur = $state(0);
  const menuStyle = $derived.by(() => {
    const largeur = 250;
    const posX = Math.max(8, Math.min(x, window.innerWidth - largeur - 8));
    const posY = y + hauteur > window.innerHeight - 8 ? Math.max(8, top - hauteur) : y;
    return `left: ${posX}px; top: ${posY}px;`;
  });

  async function run(action: () => Promise<unknown>) {
    onclose();
    try {
      await action();
      onchange();
    } catch (err) {
      toasts.push({ type: "error", title: $t("notify.error"), message: String(err) });
    }
  }
  const hide = () => run(() => invoke("podcast_hide_from_listen", { podcastId: episode.podcast_id }));
  const markPlayed = () => run(() => invoke("podcast_set_played", { episodeId: episode.id, played: true }));
  function open() {
    onclose();
    goto(`/podcasts/${episode.podcast_id}`);
  }

  const ligne = "w-full flex items-center gap-2.5 px-3.5 py-2 text-sm text-left cursor-pointer transition-colors text-(--rg-tx) hover:bg-(--rg-s2) dark:hover:bg-[#252b28]";
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && onclose()} />

<!-- Porté vers `body` : le carrousel rognerait le menu. -->
<div use:portal>
<button type="button" class="fixed inset-0 z-9998 cursor-default" onclick={onclose}
        oncontextmenu={(e) => { e.preventDefault(); onclose(); }} aria-label={$t("menu.close_menu")}></button>

<div role="menu" tabindex="-1" oncontextmenu={(e) => e.preventDefault()}
     class="fixed z-[9999] w-62.5 py-1.5 bg-(--rg-carte) dark:bg-(--rg-s2) border border-(--rg-bd) dark:border-[#2a312d]
            rounded-xl shadow-[0_18px_40px_rgba(0,0,0,0.18)] dark:shadow-[0_18px_40px_rgba(0,0,0,0.6)] overflow-hidden"
     style={menuStyle} bind:clientHeight={hauteur}>
  <button type="button" role="menuitem" class={ligne} onclick={hide}>
    <Icon icon="material-symbols:visibility-off-outline-rounded" width="17" class="opacity-60" />
    {$t("podcasts.hide_from_listen")}
  </button>
  <button type="button" role="menuitem" class={ligne} onclick={markPlayed}>
    <Icon icon="material-symbols:check-circle-outline-rounded" width="17" class="opacity-60" />
    {$t("podcasts.mark_played")}
  </button>
  <div class="h-px mx-2 my-1 bg-(--rg-bd) dark:bg-white/8"></div>
  <button type="button" role="menuitem" class={ligne} onclick={open}>
    <Icon icon="material-symbols:arrow-forward-rounded" width="17" class="opacity-60" />
    {$t("podcasts.open")}
  </button>
</div>
</div>
