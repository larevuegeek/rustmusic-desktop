<script lang="ts">
  import Icon from "@iconify/svelte";
  import { queueState } from "$lib/stores/queue/queueState.store";
  import { toasts } from "$lib/stores/ui/toast.store";
  import { toQueueTracks, type TrackLike } from "$lib/helper/tools/queueTools";
  import { playlistStore } from "$lib/stores/playlist/playlist.store";
  import { invoke } from "@tauri-apps/api/core";
  import { t } from "$lib/i18n";
  import { open } from "@tauri-apps/plugin-dialog";
  import type { Playlist } from "$lib/types/db/playlist/Playlist";
  import { libraryContentStore } from "$lib/stores/library/libraryContent.store";
  import DeezerCoverSearchPopin from "$lib/components/library/common/popin/DeezerCoverSearchPopin.svelte";

  type Props = {
    title: string;
    type: "album" | "playlist";
    loadTracks: () => Promise<TrackLike[]>;
    x: number;
    y: number;
    onclose: () => void;
    oncover?: () => void;
    albumId?: string | null;
    artistName?: string | null;
    /**
     * Ouvre l'atelier de tags sur cette collection.
     *
     * Fourni par l'appelant plutôt que construit ici : le menu ne sait pas
     * d'où viennent ses morceaux — un album les charge par identifiant, une
     * playlist autrement.
     */
    onedittags?: () => void;
  };

  let { title, type, loadTracks, x, y, onclose, oncover, albumId = null, artistName = null, onedittags }: Props = $props();

  let loading = $state(false);
  let showPlaylistSub = $state(false);
  let showCoverSub = $state(false);
  let showDeezerSearch = $state(false);

  async function handleFetchCover() {
    if (!albumId) return;
    loading = true;
    try {
      const result = await invoke<string | null>('fetch_album_cover', { albumId, albumTitle: title, artistName });
      if (result) {
        await libraryContentStore.refresh();
        oncover?.();
        toasts.push({ type: "success", title: "Pochette", message: "Pochette récupérée depuis Deezer" });
      } else {
        toasts.push({ type: "error", title: "Pochette", message: "Pochette de l'album non trouvée sur Deezer" });
      }
    } catch (e) {
      toasts.push({ type: "error", title: "Erreur", message: String(e) });
    } finally {
      loading = false;
      onclose();
    }
  }

  async function handleChooseCover() {
    if (!albumId) return;
    const selected = await open({
      multiple: false,
      title: "Choisir une pochette",
      filters: [{ name: 'Images', extensions: ['jpg', 'jpeg', 'png', 'webp'] }],
    });
    if (!selected) return;
    loading = true;
    try {
      await invoke('set_album_cover', { albumId, imagePath: selected });
      await libraryContentStore.refresh();
      oncover?.();
      toasts.push({ type: "success", title: "Pochette", message: "Pochette mise à jour" });
    } catch (e) {
      toasts.push({ type: "error", title: "Erreur", message: String(e) });
    } finally {
      loading = false;
      onclose();
    }
  }

  let playlists = $derived($playlistStore.playlists);

  let menuStyle = $derived.by(() => {
    const menuWidth = 220;
    const menuHeight = 280;
    let posX = x;
    let posY = y;

    if (posX + menuWidth > window.innerWidth) posX = window.innerWidth - menuWidth - 8;
    if (posY + menuHeight > window.innerHeight) posY = window.innerHeight - menuHeight - 8;

    return `left: ${posX}px; top: ${posY}px;`;
  });

  const label = $derived(type === "album" ? "l'album" : "la playlist");

  async function handlePlayAll() {
    loading = true;
    try {
      const tracks = await loadTracks();
      if (tracks.length === 0) return;
      const queueTracks = toQueueTracks(tracks);
      await queueState.loadTracks(queueTracks);
    } catch (e) {
      console.error('Failed to play collection:', e);
    } finally {
      loading = false;
      onclose();
    }
  }

  async function handleAddAllNext() {
    loading = true;
    try {
      const tracks = await loadTracks();
      if (tracks.length === 0) return;
      const queueTracks = toQueueTracks(tracks);
      for (const t of queueTracks) {
        queueState.addTrack(t);
      }
      toasts.push({ type: "success", title: "Ajouté en priorité", message: `${tracks.length} morceau(x) seront lus ensuite` });
    } catch (e) {
      console.error('Failed to enqueue collection:', e);
    } finally {
      loading = false;
      onclose();
    }
  }

  async function handleAddAllToQueue() {
    loading = true;
    try {
      const tracks = await loadTracks();
      if (tracks.length === 0) return;
      const queueTracks = toQueueTracks(tracks);
      for (const t of queueTracks) {
        queueState.enqueue(t);
      }
      toasts.push({ type: "success", title: "Ajouté à la file", message: `${tracks.length} morceau(x) ajoutés à la suite` });
    } catch (e) {
      console.error('Failed to add to queue:', e);
    } finally {
      loading = false;
      onclose();
    }
  }

  async function handleAddToPlaylist(pl: Playlist) {
    loading = true;
    try {
      const tracks = await loadTracks();
      if (tracks.length === 0) return;

      let added = 0;
      for (const track of tracks) {
        if (!track.path) continue;
        const params: Record<string, any> = { playlistId: pl.id, path: track.path };
        if ((track as any).id && typeof (track as any).id === 'string') {
          params.libraryTrackId = (track as any).id;
        }
        try {
          await invoke('add_track_to_playlist', params);
          added++;
        } catch { /* skip duplicates */ }
      }

      await playlistStore.refresh();
      toasts.push({ type: "success", title: "Ajouté", message: `${added} morceau(x) ajoutés à ${pl.name}` });
    } catch (e) {
      toasts.push({ type: "error", title: "Erreur", message: String(e) });
    } finally {
      loading = false;
      onclose();
    }
  }
</script>

{#if !showDeezerSearch}
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
  <!-- Header -->
  <div class="px-3.5 py-1.5 mb-1">
    <p class="text-[10px] uppercase tracking-widest text-neutral-500 truncate">{title}</p>
  </div>

  <button
    class="w-full flex items-center gap-2.5 px-3.5 py-2 text-sm text-left cursor-pointer
           text-neutral-200 hover:bg-green-500/15 hover:text-green-400 transition-colors
           disabled:opacity-50"
    onclick={handlePlayAll}
    disabled={loading}
  >
    <Icon icon="lucide:play" width="14" class="opacity-60" />
    Lire tout {label}
  </button>

  <button
    class="w-full flex items-center gap-2.5 px-3.5 py-2 text-sm text-left cursor-pointer
           text-neutral-200 hover:bg-white/10 transition-colors
           disabled:opacity-50"
    onclick={handleAddAllNext}
    disabled={loading}
  >
    <Icon icon="lucide:list-start" width="14" class="opacity-60" />
    Lire ensuite
  </button>

  <button
    class="w-full flex items-center gap-2.5 px-3.5 py-2 text-sm text-left cursor-pointer
           text-neutral-200 hover:bg-white/10 transition-colors
           disabled:opacity-50"
    onclick={handleAddAllToQueue}
    disabled={loading}
  >
    <Icon icon="lucide:list-end" width="14" class="opacity-60" />
    Ajouter tout à la file
  </button>

  {#if onedittags}
    <div class="h-px mx-3 my-1 bg-white/6"></div>
    <button
      class="w-full flex items-center gap-2.5 px-3.5 py-2 text-sm text-left cursor-pointer
             text-neutral-200 hover:bg-white/10 transition-colors"
      onclick={() => { onclose(); onedittags(); }}
    >
      <Icon icon="lucide:table-properties" width="14" class="opacity-60" />
      {$t('workshop.fix_tags')}
    </button>
  {/if}

  <!-- Séparateur -->
  <div class="h-px mx-3 my-1 bg-white/6"></div>

  <!-- Ajouter à une playlist -->
  <button
    class="w-full flex items-center justify-between px-3.5 py-2 text-sm text-left cursor-pointer
           text-neutral-200 hover:bg-white/10 transition-colors"
    onclick={() => showPlaylistSub = !showPlaylistSub}
  >
    <span class="flex items-center gap-2.5">
      <Icon icon="lucide:list-music" width="14" class="opacity-60" />
      Ajouter à une playlist
    </span>
    <Icon icon={showPlaylistSub ? "lucide:chevron-down" : "lucide:chevron-right"} width="12" class="opacity-40" />
  </button>

  {#if showPlaylistSub}
    <div class="border-t border-white/5 bg-white/2">
      {#if playlists.length === 0}
        <div class="flex flex-col items-center py-4 px-3">
          <Icon icon="lucide:list-music" width="16" class="text-neutral-600 mb-1.5" />
          <p class="text-[11px] text-neutral-500">Aucune playlist</p>
        </div>
      {:else}
        {#each playlists as pl, i (pl.id)}
          {#if i > 0}
            <div class="h-px mx-2 bg-white/4"></div>
          {/if}
          <button
            class="w-full flex items-center gap-2.5 pl-6 pr-3 py-2 text-[12px] text-left cursor-pointer
                   text-neutral-300 hover:bg-white/8 transition-colors
                   disabled:opacity-50"
            onclick={() => handleAddToPlaylist(pl)}
            disabled={loading}
          >
            <Icon icon={pl.icon ?? "lucide:list-music"} width="13"
                  style="color: {pl.color ?? '#22c55e'};" class="opacity-70" />
            <span class="truncate">{pl.name}</span>
            <span class="ml-auto text-[10px] text-neutral-600">{pl.track_count}</span>
          </button>
        {/each}
      {/if}
    </div>
  {/if}

  {#if type === 'album' && albumId}
    <!-- Séparateur -->
    <div class="h-px mx-3 my-1 bg-white/6"></div>

    <button
      class="w-full flex items-center justify-between px-3.5 py-2 text-sm text-left cursor-pointer
             text-neutral-200 hover:bg-white/10 transition-colors"
      onclick={() => showCoverSub = !showCoverSub}
    >
      <span class="flex items-center gap-2.5">
        <Icon icon="lucide:image" width="14" class="opacity-60" />
        Changer de pochette
      </span>
      <Icon icon={showCoverSub ? "lucide:chevron-down" : "lucide:chevron-right"} width="12" class="opacity-40" />
    </button>

    {#if showCoverSub}
      <div class="border-t border-white/5 bg-white/2">
        <button
          class="w-full flex items-center gap-2.5 pl-6 pr-3 py-2 text-[12px] text-left cursor-pointer
                 text-neutral-300 hover:bg-white/8 transition-colors
                 disabled:opacity-50"
          onclick={handleFetchCover}
          disabled={loading}
        >
          <Icon icon="lucide:wand-sparkles" width="13" class="opacity-60" />
          Importer depuis Deezer
        </button>

        <div class="h-px mx-2 bg-white/4"></div>

        <button
          class="w-full flex items-center gap-2.5 pl-6 pr-3 py-2 text-[12px] text-left cursor-pointer
                 text-neutral-300 hover:bg-white/8 transition-colors"
          onclick={() => { showDeezerSearch = true; }}
        >
          <Icon icon="lucide:search" width="13" class="opacity-60" />
          Chercher sur Deezer
        </button>

        <div class="h-px mx-2 bg-white/4"></div>

        <button
          class="w-full flex items-center gap-2.5 pl-6 pr-3 py-2 text-[12px] text-left cursor-pointer
                 text-neutral-300 hover:bg-white/8 transition-colors
                 disabled:opacity-50"
          onclick={handleChooseCover}
          disabled={loading}
        >
          <Icon icon="lucide:folder-open" width="13" class="opacity-60" />
          Choisir un fichier
        </button>
      </div>
    {/if}
  {/if}
</div>
{/if}

{#if showDeezerSearch && albumId}
  <DeezerCoverSearchPopin
    albumId={albumId}
    initialQuery={artistName ? `${artistName} ${title}` : title}
    onclose={() => { showDeezerSearch = false; onclose(); }}
    {oncover}
  />
{/if}
