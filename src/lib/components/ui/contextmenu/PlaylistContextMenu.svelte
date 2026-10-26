<script lang="ts">
  import Icon from "@iconify/svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { goto } from "$app/navigation";
  import { t } from "#lib/i18n";
  import type { Playlist } from "#lib/types/db/playlist/Playlist";
  import type { TrackListView } from "#lib/types/ui/library/track/TrackListView";
  import { popinStore } from "#lib/stores/ui/popin.store";
  import { playlistStore } from "#lib/stores/playlist/playlist.store";
  import { queueState } from "#lib/stores/queue/queueState.store";
  import { playerService } from "#lib/services/player/player.service";
  import { toQueueTracks } from "#lib/helper/tools/queueTools";
  import { toasts } from "#lib/stores/ui/toast.store";
  import { offlineStore } from "#lib/stores/mediaServer/offline.store";
  import { portal } from "#lib/helper/portal";
  import { save } from "@tauri-apps/plugin-dialog";
  import EditPlaylistPopin from "#lib/components/playlist/popin/EditPlaylistPopin.svelte";
  import SmartPlaylistPopin from "#lib/components/playlist/smart/SmartPlaylistPopin.svelte";

  /** Les actions d'une playlist, au clic droit sur sa ligne. */
  let { x, y, playlist, onclose }: {
    x: number;
    y: number;
    playlist: Playlist;
    onclose: () => void;
  } = $props();

  let menuStyle = $derived.by(() => {
    const largeur = 220;
    const hauteur = 230;

    const posX = x + largeur > window.innerWidth ? window.innerWidth - largeur - 8 : x;

    // Vers le haut quand la place manque en bas, au lieu de se coller au bord :
    // la zone des playlists touche le lecteur, et un menu rabattu là-bas
    // recouvrirait les commandes de lecture.
    const posY = y + hauteur > window.innerHeight
      ? Math.max(8, y - hauteur)
      : y;

    return `left: ${posX}px; top: ${posY}px;`;
  });

  // Export .m3u : les morceaux sur le disque (ceux d'un serveur sont laissés de côté).
  async function exportM3u() {
    onclose();
    const dest = await save({ title: $t("playlist.export_m3u"), defaultPath: `${playlist.name.replace(/[\\/:*?"<>|]/g, "_")}.m3u8`,
      filters: [{ name: "M3U", extensions: ["m3u8", "m3u"] }] });
    if (!dest) return;
    try {
      const [written, skipped] = await invoke<[number, number]>("playlist_export_m3u", { playlistId: playlist.id, dest });
      toasts.push({ type: "success", title: playlist.name,
        message: $t("playlist.export_m3u_done").replace("{n}", String(written)) + (skipped ? " · " + $t("playlist.export_m3u_skipped").replace("{n}", String(skipped)) : "") });
    } catch (e) {
      toasts.push({ type: "error", title: $t("playlist.export_m3u"), message: String(e) });
    }
  }

  async function play() {
    onclose();
    try {
      const pistes = await invoke<TrackListView[]>('get_playlist_tracks_view', { playlistId: playlist.id });
      if (!pistes?.length) {
        toasts.push({ type: 'info', title: playlist.name, message: $t('playlist.empty') });
        return;
      }
      const file = toQueueTracks(pistes);
      await queueState.loadTracks(file);
      playerService.playFile(file[0]);
    } catch (e) {
      toasts.push({ type: 'error', title: $t('notify.error'), message: String(e) });
    }
  }

  async function pin() {
    onclose();
    await playlistStore.setPinned(playlist.id, !playlist.pinned);
  }

  /** Le crayon de la barre latérale ouvre déjà cette distinction. */
  function edit() {
    onclose();
    if (playlist.is_smart) {
      popinStore.open($t('playlist.new_smart'), SmartPlaylistPopin,
        { playlistId: playlist.id }, { size: 'xl', icon: 'lucide:sparkles', flush: true });
    } else {
      popinStore.open($t('menu.edit_playlist'), EditPlaylistPopin, { playlist });
    }
  }

  // Playlist d'un serveur : tous ses morceaux gardés hors ligne.
  async function keepOffline() {
    onclose();
    try {
      const pistes = await invoke<TrackListView[]>("get_playlist_tracks_view", { playlistId: playlist.id });
      const added = await offlineStore.add(pistes.map((p) => p.path).filter((p): p is string => !!p));
      toasts.push({ type: "success", title: $t("servers.offline_keep"),
                    message: $t(added === 1 ? "servers.offline_added_one" : "servers.offline_added_n").replace("{n}", String(added)) });
    } catch (e) {
      toasts.push({ type: "error", title: $t("notify.error"), message: String(e) });
    }
  }

  // Playlist d'un serveur : un premier clic prévient, le second supprime (aussi sur le serveur).
  let confirmRemove = $state(false);

  async function remove() {
    if (playlist.server_id && !confirmRemove) {
      confirmRemove = true;
      return;
    }
    onclose();
    const nom = playlist.name;
    try {
      await playlistStore.removePlaylist(playlist.id);
      // On quitte la page si c'est celle qu'on regardait, sinon elle reste
      // affichée en désignant une playlist qui n'existe plus.
      if (window.location.pathname === `/playlist/${playlist.id}`) goto('/');
      toasts.push({ type: 'success', title: nom, message: $t('playlist.deleted') });
    } catch (e) {
      toasts.push({ type: 'error', title: $t('notify.error'), message: String(e) });
    }
  }

  const ligne = "w-full flex items-center gap-2.5 px-3.5 py-2 text-sm text-left cursor-pointer transition-colors";
</script>

<!-- Porté vers `body` : la barre latérale porte `backdrop-blur`, qui crée un
     contexte d'empilement. Le `z-index` du menu y restait confiné et le
     lecteur, plus bas dans le document, passait devant. -->
<div use:portal>
<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- Le clic droit est avalé ici aussi : sans ça, un second clic droit pendant
     que le menu est ouvert laissait passer celui du navigateur. -->
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
  class="fixed z-[9999] w-55 py-1.5
         bg-(--rg-carte) dark:bg-(--rg-s2)
         border border-(--rg-bd) dark:border-[#2a312d]
         rounded-xl shadow-[0_18px_40px_rgba(0,0,0,0.18)] dark:shadow-[0_18px_40px_rgba(0,0,0,0.6)]
         overflow-hidden"
  style={menuStyle}
>
  <button class="{ligne} text-(--rg-tx) hover:bg-green-500/15 hover:text-(--rg-gtx)" onclick={play}>
    <Icon icon="lucide:play" width="14" class="opacity-60" />
    {$t('playlist.play')}
  </button>

  <button class="{ligne} text-(--rg-tx) hover:bg-(--rg-s2) dark:hover:bg-[#252b28]" onclick={edit}>
    <Icon icon={playlist.is_smart ? 'lucide:sparkles' : 'lucide:pen-line'} width="14" class="opacity-60" />
    {playlist.is_smart ? $t('playlist.edit_rules') : $t('playlist.edit')}
  </button>

  {#if playlist.server_id}
    <button class="{ligne} text-(--rg-tx) hover:bg-(--rg-s2) dark:hover:bg-[#252b28]" onclick={keepOffline}>
      <Icon icon="material-symbols:download-for-offline-outline-rounded" width="15" class="opacity-60" />
      {$t("servers.offline_keep")}
    </button>
  {/if}

  {#if !playlist.source_id}
    <button class="{ligne} text-(--rg-tx) hover:bg-(--rg-s2) dark:hover:bg-[#252b28]" onclick={exportM3u}>
      <Icon icon="material-symbols:download-rounded" width="15" class="opacity-60" />
      {$t("playlist.export_m3u")}
    </button>
  {/if}

  <button class="{ligne} text-(--rg-tx) hover:bg-(--rg-s2) dark:hover:bg-[#252b28]" onclick={pin}>
    <Icon icon={playlist.pinned ? 'material-symbols:keep-off-outline-rounded' : 'material-symbols:keep-outline-rounded'} width="15" class="opacity-60" />
    {playlist.pinned ? $t('sidebar.unpin') : $t('sidebar.pin')}
  </button>

  <div class="h-px mx-2 my-1 bg-(--rg-bd) dark:bg-white/8"></div>

  <button class="{ligne} text-red-600 dark:text-red-400 hover:bg-red-500/10 {confirmRemove ? 'bg-red-500/10 font-semibold' : ''}" onclick={remove}>
    <Icon icon={confirmRemove ? "material-symbols:cloud-off-outline-rounded" : "lucide:trash-2"} width="14" class="opacity-60" />
    {confirmRemove ? $t('servers.playlist_delete_confirm') : $t('playlist.delete')}
  </button>
</div>
</div>
