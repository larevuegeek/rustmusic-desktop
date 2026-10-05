<script lang="ts">
  import Icon from "@iconify/svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { goto } from "$app/navigation";
  import { t } from "#lib/i18n";
  import type { TrackListView } from "#lib/types/ui/library/track/TrackListView";
  import { queueState } from "#lib/stores/queue/queueState.store";
  import { playerService } from "#lib/services/player/player.service";
  import { toQueueTracks } from "#lib/helper/tools/queueTools";
  import { toasts } from "#lib/stores/ui/toast.store";
  import { portal } from "#lib/helper/portal";
  import { pinsStore, isPinned, type PinKind } from "#lib/stores/library/pins.store";

  /** Les actions d'un album ou d'un artiste : lire, ouvrir, épingler. */
  let { kind, id, title, libraryId, x, y, onclose }: {
    kind: PinKind;
    id: string;
    title: string;
    libraryId: number;
    x: number;
    y: number;
    onclose: () => void;
  } = $props();

  const epingle = $derived(isPinned($pinsStore, kind, id));
  const lien = $derived(`/library/${libraryId}/${kind === 'album' ? 'albums' : 'artists'}/${id}`);

  let menuStyle = $derived.by(() => {
    const largeur = 232;
    const hauteur = 150;
    const posX = x + largeur > window.innerWidth ? window.innerWidth - largeur - 8 : x;
    const posY = y + hauteur > window.innerHeight ? Math.max(8, y - hauteur) : y;
    return `left: ${posX}px; top: ${posY}px;`;
  });

  // Tout est lu avant `onclose()` : les props peuvent venir d'un état que la
  // fermeture remet à `null` (la barre latérale).
  function figer() {
    return { kind, id, title, libraryId, epingle, lien };
  }

  async function lire() {
    const m = figer();
    onclose();
    try {
      const pistes = m.kind === 'album'
        ? await invoke<TrackListView[]>('get_tracks_by_album', { libraryId: m.libraryId, libraryAlbumId: m.id })
        : await invoke<TrackListView[]>('get_tracks_by_artist', { libraryId: m.libraryId, artistId: m.id });
      if (!pistes?.length) return;
      const file = toQueueTracks(pistes);
      await queueState.loadTracks(file);
      playerService.playFile(file[0]);
    } catch (e) {
      toasts.push({ type: 'error', title: $t('notify.error'), message: String(e) });
    }
  }

  function ouvrir() {
    const m = figer();
    onclose();
    goto(m.lien);
  }

  async function basculer() {
    const m = figer();
    onclose();
    try {
      await pinsStore.setPinned(m.kind, m.libraryId, m.id, !m.epingle);
      toasts.push({ type: 'success', title: m.title, message: $t(m.epingle ? 'sidebar.unpinned' : 'sidebar.pinned') });
    } catch (e) {
      toasts.push({ type: 'error', title: $t('notify.error'), message: String(e) });
    }
  }

  const ligne = "w-full flex items-center gap-2.5 px-3.5 py-2 text-sm text-left cursor-pointer transition-colors";
</script>

<!-- Porté vers `body` : voir `PlaylistContextMenu`. -->
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
  class="fixed z-[9999] w-58 py-1.5
         bg-(--rg-carte) dark:bg-(--rg-s2) 
         border border-(--rg-bd) dark:border-[#2a312d]
         rounded-xl shadow-[0_18px_40px_rgba(0,0,0,0.18)] dark:shadow-[0_18px_40px_rgba(0,0,0,0.6)] 
         overflow-hidden"
  style={menuStyle}
>
  <p class="px-3.5 pt-1 pb-1.5 text-[10px] uppercase tracking-widest text-(--rg-mu) truncate">{title}</p>

  <button class="{ligne} text-(--rg-tx) hover:bg-green-500/15 hover:text-(--rg-gtx)" onclick={lire}>
    <Icon icon="lucide:play" width="14" class="opacity-60" />
    {$t('sidebar.play')}
  </button>

  <button class="{ligne} text-(--rg-tx) hover:bg-(--rg-s2) dark:hover:bg-[#252b28]" onclick={ouvrir}>
    <Icon icon="lucide:arrow-up-right" width="14" class="opacity-60" />
    {$t('sidebar.open')}
  </button>

  <div class="h-px mx-2 my-1 bg-(--rg-bd) dark:bg-white/8"></div>

  <button class="{ligne} text-(--rg-tx) hover:bg-(--rg-s2) dark:hover:bg-[#252b28]" onclick={basculer}>
    <Icon icon={epingle ? 'material-symbols:keep-off-outline-rounded' : 'material-symbols:keep-outline-rounded'} width="15" class="opacity-60" />
    {$t(epingle ? 'sidebar.unpin' : 'sidebar.pin')}
  </button>
</div>
</div>
