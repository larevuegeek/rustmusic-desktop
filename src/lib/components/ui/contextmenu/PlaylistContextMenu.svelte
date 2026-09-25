<script lang="ts">
  import Icon from "@iconify/svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { goto } from "$app/navigation";
  import { t } from "$lib/i18n";
  import type { Playlist } from "$lib/types/db/playlist/Playlist";
  import type { TrackListView } from "$lib/types/ui/library/track/TrackListView";
  import { popinStore } from "$lib/stores/ui/popin.store";
  import { playlistStore } from "$lib/stores/playlist/playlist.store";
  import { queueState } from "$lib/stores/queue/queueState.store";
  import { playerService } from "$lib/services/player/player.service";
  import { toQueueTracks } from "$lib/helper/tools/queueTools";
  import { toasts } from "$lib/stores/ui/toast.store";
  import { portal } from "$lib/helper/portal";
  import EditPlaylistPopin from "$lib/components/playlist/popin/EditPlaylistPopin.svelte";
  import SmartPlaylistPopin from "$lib/components/playlist/smart/SmartPlaylistPopin.svelte";

  /** Les actions d'une playlist, au clic droit sur sa ligne. */
  let { x, y, playlist, onclose }: {
    x: number;
    y: number;
    playlist: Playlist;
    onclose: () => void;
  } = $props();

  let menuStyle = $derived.by(() => {
    const largeur = 220;
    const hauteur = 150;

    const posX = x + largeur > window.innerWidth ? window.innerWidth - largeur - 8 : x;

    // Vers le haut quand la place manque en bas, au lieu de se coller au bord :
    // la zone des playlists touche le lecteur, et un menu rabattu là-bas
    // recouvrirait les commandes de lecture.
    const posY = y + hauteur > window.innerHeight
      ? Math.max(8, y - hauteur)
      : y;

    return `left: ${posX}px; top: ${posY}px;`;
  });

  async function lire() {
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
      toasts.push({ type: 'error', title: 'Erreur', message: String(e) });
    }
  }

  /** Le crayon de la barre latérale ouvre déjà cette distinction. */
  function modifier() {
    onclose();
    if (playlist.is_smart) {
      popinStore.open("Playlist intelligente", SmartPlaylistPopin,
        { playlistId: playlist.id }, { size: 'xl', icon: 'lucide:sparkles', flush: true });
    } else {
      popinStore.open("Modifier la playlist", EditPlaylistPopin, { playlist });
    }
  }

  async function supprimer() {
    onclose();
    const nom = playlist.name;
    try {
      await playlistStore.removePlaylist(playlist.id);
      // On quitte la page si c'est celle qu'on regardait, sinon elle reste
      // affichée en désignant une playlist qui n'existe plus.
      if (window.location.pathname === `/playlist/${playlist.id}`) goto('/');
      toasts.push({ type: 'success', title: nom, message: $t('playlist.deleted') });
    } catch (e) {
      toasts.push({ type: 'error', title: 'Erreur', message: String(e) });
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
  aria-label="Fermer le menu"
></button>

<div
  role="menu"
  tabindex="-1"
  oncontextmenu={(e) => e.preventDefault()}
  class="fixed z-[9999] w-55 py-1.5
         bg-neutral-950/95 backdrop-blur-xl
         border border-white/10
         rounded-xl shadow-2xl shadow-black/30
         overflow-hidden"
  style={menuStyle}
>
  <button class="{ligne} text-neutral-200 hover:bg-green-500/15 hover:text-green-400" onclick={lire}>
    <Icon icon="lucide:play" width="14" class="opacity-60" />
    {$t('playlist.play')}
  </button>

  <button class="{ligne} text-neutral-200 hover:bg-white/5" onclick={modifier}>
    <Icon icon={playlist.is_smart ? 'lucide:sparkles' : 'lucide:pen-line'} width="14" class="opacity-60" />
    {playlist.is_smart ? $t('playlist.edit_rules') : $t('playlist.edit')}
  </button>

  <div class="h-px mx-2 my-1 bg-white/6"></div>

  <button class="{ligne} text-red-400 hover:bg-red-500/10" onclick={supprimer}>
    <Icon icon="lucide:trash-2" width="14" class="opacity-60" />
    {$t('playlist.delete')}
  </button>
</div>
</div>
