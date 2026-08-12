<script lang="ts">
  import Icon from "@iconify/svelte";
  import { goto } from "$app/navigation";
  import { handlePlayTrack } from "$lib/actions/player/PlayerAction";
  import { queueState } from "$lib/stores/queue/queueState.store";
  import { liked } from "$lib/stores/playlist/like.store";
  import { toasts } from "$lib/stores/ui/toast.store";
  import { toQueueTrack, type TrackLike } from "$lib/helper/tools/queueTools";
  import { playlistStore } from "$lib/stores/playlist/playlist.store";
  import { invoke } from "@tauri-apps/api/core";
  import type { Playlist } from "$lib/types/db/playlist/Playlist";
  import { t } from "$lib/i18n";
  import { popinStore } from "$lib/stores/ui/popin.store";
  import { canWriteTags } from "$lib/services/tags/tagEditor.service";
  import EditTagsPopin from "$lib/components/library/common/popin/EditTagsPopin.svelte";

  type Props = {
    track: TrackLike & { id?: string | number | null; artist_id?: string | null; library_artist_id?: string | null; album_id?: string | null };
    x: number;
    y: number;
    libraryId?: number | null;
    onclose: () => void;
    showNavigation?: boolean;
    showDelete?: boolean;
    deleteLabel?: string;
    showAddToPlaylist?: boolean;
    ondelete?: () => void;
  };

  let {
    track,
    x,
    y,
    libraryId = null,
    onclose,
    showNavigation = false,
    showDelete = false,
    deleteLabel = 'Supprimer',
    showAddToPlaylist = true,
    ondelete,
  }: Props = $props();

  let showPlaylistSub = $state(false);
  let isLiked = $derived(track.path ? $liked.paths.has(track.path) : false);

  // Le backend seul sait quels conteneurs il sait réécrire : on lui demande
  // plutôt que de dupliquer une liste d'extensions qui divergerait.
  let canEditTags = $state(false);
  $effect(() => {
    const p = track.path;
    if (!p) {
      canEditTags = false;
      return;
    }
    canWriteTags(p).then((ok) => (canEditTags = ok));
  });

  function handleEditTags() {
    if (!track.path) return;
    // `flush` : l'éditeur gère lui-même ses deux colonnes défilantes et son
    // pied de page fixe, ce que la mise en page par défaut de la popin
    // (un unique conteneur défilant rembourré) ne permet pas.
    popinStore.open(
      $t('tags.edit'),
      EditTagsPopin,
      {
        path: track.path,
        onsaved: () =>
          toasts.push({ type: "success", title: $t('tags.saved'), message: "" }),
      },
      { size: "xl", flush: true, icon: "lucide:tags" },
    );
    onclose();
  }
  let playlists = $derived($playlistStore.playlists);

  // Position ajustée pour ne pas sortir de l'écran
  let menuStyle = $derived.by(() => {
    const menuWidth = 220;
    const menuHeight = 350;
    let posX = x;
    let posY = y;

    if (posX + menuWidth > window.innerWidth) posX = window.innerWidth - menuWidth - 8;
    if (posY + menuHeight > window.innerHeight) posY = window.innerHeight - menuHeight - 8;

    return `left: ${posX}px; top: ${posY}px;`;
  });

  function handlePlay() {
    if (!track.path) return;
    handlePlayTrack(track.path);
    onclose();
  }

  function handleAddNext() {
    queueState.addTrack(toQueueTrack(track));
    toasts.push({ type: "success", title: "Ajouté en priorité", message: "Le morceau sera lu juste après" });
    onclose();
  }

  function handleAddToQueue() {
    queueState.enqueue(toQueueTrack(track));
    toasts.push({ type: "success", title: "Ajouté à la file", message: "Le morceau a été ajouté à la suite" });
    onclose();
  }

  function handleToggleLike() {
    if (!track.path) return;
    liked.toggle(track.path);
    onclose();
  }

  async function handleAddToPlaylist(pl: Playlist) {
    try {
      // Construire les params selon ce qu'on a :
      // - library_track_id (string UUID) → tracks de la bibliothèque
      // - path → fichiers liked, recent, explorateur
      const params: Record<string, any> = { playlistId: pl.id };

      // Si on a un ID string (UUID des library_tracks)
      if (track.id && typeof track.id === 'string') {
        params.libraryTrackId = track.id;
      }
      // Sinon on cherche par path
      else if (track.path) {
        params.path = track.path;
      }
      // Sinon on ne peut rien faire
      else {
        toasts.push({ type: "error", title: "Erreur", message: "Impossible d'identifier le morceau" });
        onclose();
        return;
      }

      await invoke('add_track_to_playlist', params);

      // Rafraîchir le store pour mettre à jour les compteurs dans la sidebar
      await playlistStore.refresh();

      toasts.push({ type: "success", title: "Ajouté", message: `Ajouté à ${pl.name}` });
    } catch (e) {
      toasts.push({ type: "error", title: "Erreur", message: String(e) });
    }
    onclose();
  }

  function handleViewTrack() {
    if (libraryId && track.id) goto(`/library/${libraryId}/tracks/${track.id}`);
    onclose();
  }

  function handleViewAlbum() {
    if (libraryId && track.album_id) goto(`/library/${libraryId}/albums/${track.album_id}`);
    onclose();
  }

  function handleViewArtist() {
    if (libraryId && (track.library_artist_id || track.artist_id)) {
      goto(`/library/${libraryId}/artists/${track.library_artist_id ?? track.artist_id}`);
    }
    onclose();
  }

  function handleDelete() {
    ondelete?.();
    onclose();
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- Backdrop invisible pour fermer -->
<button type="button" class="fixed inset-0 z-9998 cursor-default" onclick={onclose} aria-label="Fermer le menu"></button>

<!-- Menu contextuel -->
<div
  class="fixed z-[9999] w-55 py-1.5
         bg-neutral-950/95 backdrop-blur-xl
         border border-white/10
         rounded-xl shadow-2xl shadow-black/30
         overflow-hidden"
  style={menuStyle}
>
  <!-- Lire -->
  <button
    class="w-full flex items-center gap-2.5 px-3.5 py-2 text-sm text-left cursor-pointer
           text-neutral-200 hover:bg-green-500/15 hover:text-green-400 transition-colors"
    onclick={handlePlay}
  >
    <Icon icon="lucide:play" width="14" class="opacity-60" />
    Lire maintenant
  </button>

  <button
    class="w-full flex items-center gap-2.5 px-3.5 py-2 text-sm text-left cursor-pointer
           text-neutral-200 hover:bg-white/10 transition-colors"
    onclick={handleAddNext}
  >
    <Icon icon="lucide:list-start" width="14" class="opacity-60" />
    Lire ensuite
  </button>

  <button
    class="w-full flex items-center gap-2.5 px-3.5 py-2 text-sm text-left cursor-pointer
           text-neutral-200 hover:bg-white/10 transition-colors"
    onclick={handleAddToQueue}
  >
    <Icon icon="lucide:list-end" width="14" class="opacity-60" />
    Ajouter à la file
  </button>

  <div class="h-px mx-2 my-1 bg-white/8"></div>

  <!-- Liker -->
  <button
    class="w-full flex items-center gap-2.5 px-3.5 py-2 text-sm text-left cursor-pointer
           {isLiked
             ? 'text-pink-400 hover:bg-pink-500/15'
             : 'text-neutral-200 hover:bg-pink-500/15 hover:text-pink-400'}
           transition-colors"
    onclick={handleToggleLike}
  >
    <Icon icon={isLiked ? "lucide:heart-off" : "lucide:heart"} width="14" class={isLiked ? '' : 'opacity-60'} />
    {isLiked ? 'Retirer des favoris' : 'Ajouter aux favoris'}
  </button>

  <!-- Ajouter à une playlist -->
  {#if showAddToPlaylist}
  <button
    class="w-full flex items-center justify-between px-3.5 py-2 text-sm text-left cursor-pointer
           text-neutral-200 hover:bg-white/10 transition-colors"
    onclick={() => showPlaylistSub = !showPlaylistSub}
  >
    <span class="flex items-center gap-2.5">
      <Icon icon="lucide:list-plus" width="14" class="opacity-60" />
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
            class="w-full flex items-center gap-2.5 px-3 py-2 text-[12px] text-left cursor-pointer
                   text-neutral-300 hover:bg-white/8 transition-colors"
            onclick={() => handleAddToPlaylist(pl)}
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
  {/if}

  <!-- Navigation (optionnel) -->
  {#if showNavigation && libraryId}
    <div class="h-px mx-2 my-1 bg-white/8"></div>

    {#if track.id}
      <button
        class="w-full flex items-center gap-2.5 px-3.5 py-2 text-sm text-left cursor-pointer
               text-neutral-200 hover:bg-white/10 transition-colors"
        onclick={handleViewTrack}
      >
        <Icon icon="lucide:music-4" width="14" class="opacity-60" />
        Voir le morceau
      </button>
    {/if}

    {#if track.album_id}
      <button
        class="w-full flex items-center gap-2.5 px-3.5 py-2 text-sm text-left cursor-pointer
               text-neutral-200 hover:bg-white/10 transition-colors"
        onclick={handleViewAlbum}
      >
        <Icon icon="lucide:disc-album" width="14" class="opacity-60" />
        Voir l'album
      </button>
    {/if}

    {#if track.library_artist_id || track.artist_id}
      <button
        class="w-full flex items-center gap-2.5 px-3.5 py-2 text-sm text-left cursor-pointer
               text-neutral-200 hover:bg-white/10 transition-colors"
        onclick={handleViewArtist}
      >
        <Icon icon="lucide:mic-2" width="14" class="opacity-60" />
        Voir l'artiste
      </button>
    {/if}
  {/if}

  <!-- Modifier les tags — masqué si le format n'est pas réinscriptible -->
  {#if canEditTags}
    <div class="h-px mx-2 my-1 bg-white/8"></div>
    <button
      class="w-full flex items-center gap-2.5 px-3.5 py-2 text-sm text-left cursor-pointer
             text-neutral-200 hover:bg-white/10 transition-colors"
      onclick={handleEditTags}
    >
      <Icon icon="lucide:tag" width="14" class="opacity-60" />
      {$t('tags.edit')}
    </button>
  {/if}

  <!-- Supprimer (optionnel) -->
  {#if showDelete}
    <div class="h-px mx-2 my-1 bg-white/8"></div>
    <button
      class="w-full flex items-center gap-2.5 px-3.5 py-2 text-sm text-left cursor-pointer
             text-red-400 hover:bg-red-500/15 hover:text-red-300 transition-colors"
      onclick={handleDelete}
    >
      <Icon icon="lucide:trash-2" width="14" class="opacity-60" />
      {deleteLabel}
    </button>
  {/if}
</div>
