<script lang="ts">
  import Icon from "@iconify/svelte";
  import { selectionStore } from "$lib/stores/ui/selection.store";
  import { toQueueTracks } from "$lib/helper/tools/queueTools";
  import { queueState } from "$lib/stores/queue/queueState.store";
  import { playerService } from "$lib/services/player/player.service";
  import { playlistStore } from "$lib/stores/playlist/playlist.store";
  import { invoke } from "@tauri-apps/api/core";
  import { toasts } from "$lib/stores/ui/toast.store";
  import type { Playlist } from "$lib/types/db/playlist/Playlist";
  import { fade, fly } from "svelte/transition";
  import { popinStore } from "$lib/stores/ui/popin.store";
  import { batchStore } from "$lib/stores/ui/batch.store";
  import { canWriteTags } from "$lib/services/tags/tagEditor.service";
  import { t } from "$lib/i18n";
  import EditTagsBatchPopin from "$lib/components/library/common/popin/EditTagsBatchPopin.svelte";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import { tagWorkshop } from "$lib/stores/tags/tagWorkshop.store";

  let selection = $derived($selectionStore);
  let showPlaylistMenu = $state(false);
  let playlists = $derived($playlistStore.playlists);

  async function handlePlayAll() {
    const tracks = selectionStore.getSelectedTracks();
    if (tracks.length === 0) return;
    const queueTracks = toQueueTracks(tracks);
    await queueState.loadTracks(queueTracks);
    playerService.playFile(queueTracks[0]);
    selectionStore.stop();
  }

  /**
   * Ouvre l'éditeur sur les fichiers réinscriptibles de la sélection.
   *
   * Le tri a lieu ici plutôt que dans l'éditeur : proposer de corriger des
   * fichiers dont on sait déjà qu'ils échoueront reviendrait à peupler le
   * compte rendu d'échecs annoncés. Le backend seul sait quels conteneurs il
   * réécrit, d'où l'aller-retour.
   */
  async function handleEditTags() {
    const tracks = selectionStore.getSelectedTracks();
    const candidates = tracks.map((tk) => tk.path).filter((p): p is string => !!p);
    if (candidates.length === 0) return;

    const checks = await Promise.all(candidates.map((p) => canWriteTags(p)));
    const writable = candidates.filter((_, i) => checks[i]);

    if (writable.length === 0) {
      toasts.push({
        type: "info",
        title: $t("tags.edit"),
        message: $t("tags.none_writable"),
      });
      return;
    }
    if (writable.length < candidates.length) {
      toasts.push({
        type: "info",
        title: $t("tags.edit"),
        message: `${candidates.length - writable.length} ${$t("tags.skipped_unsupported")}`,
      });
    }

    selectionStore.stop();
    popinStore.open(
      $t("tags.edit"),
      EditTagsBatchPopin,
      { paths: writable },
      { size: "xl", flush: true, icon: "lucide:tags" },
    );
  }

  /**
   * Envoie la sélection dans l'atelier.
   *
   * Le jeu de travail passe par le magasin et non par l'adresse : une liste de
   * cinquante chemins ne tient pas dans une URL. La page n'est donc pas
   * rechargeable dans ce cas — contrairement à l'entrée par album, qui elle
   * porte son identifiant.
   */
  async function handleOpenWorkshop() {
    const tracks = selectionStore.getSelectedTracks();
    const candidates = tracks.map((tk) => tk.path).filter((p): p is string => !!p);
    if (candidates.length === 0) return;

    const checks = await Promise.all(candidates.map((p) => canWriteTags(p)));
    const writable = candidates.filter((_, i) => checks[i]);

    if (writable.length === 0) {
      toasts.push({
        type: "info",
        title: $t("workshop.title"),
        message: $t("tags.none_writable"),
      });
      return;
    }

    const libraryId = page.params.library_id;
    selectionStore.stop();
    await tagWorkshop.load(writable, $t("workshop.from_selection"), page.url.pathname);
    await goto(`/library/${libraryId}/tags`);
  }

  async function handleAddToQueue() {
    const tracks = selectionStore.getSelectedTracks();
    if (tracks.length === 0) return;
    const queueTracks = toQueueTracks(tracks);
    for (const t of queueTracks) {
      queueState.enqueue(t);
    }
    toasts.push({ type: "success", title: "Ajouté à la file", message: `${tracks.length} morceau(x) ajoutés` });
    selectionStore.stop();
  }

  async function handleAddToPlaylist(pl: Playlist) {
    const tracks = selectionStore.getSelectedTracks();
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
    showPlaylistMenu = false;
    selectionStore.stop();
  }
</script>

{#if selection.active && selection.count > 0}
  <div
    class="fixed bottom-24 left-1/2 -translate-x-1/2 z-50"
    transition:fly={{ y: 20, duration: 200 }}
  >
    <div class="flex items-center gap-2 px-4 py-2.5 rounded-2xl
                bg-white/95 dark:bg-neutral-950/95 backdrop-blur-xl
                border border-neutral-200 dark:border-white/10
                shadow-[0_8px_32px_rgba(0,0,0,0.4)]">

      <!-- Count -->
      <span class="text-xs font-semibold text-emerald-600 dark:text-emerald-400 tabular-nums px-2">
        {selection.count} sélectionné{selection.count > 1 ? 's' : ''}
      </span>

      <div class="w-px h-5 bg-neutral-200 dark:bg-white/10"></div>

      <!-- Play -->
      <button
        type="button"
        class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium cursor-pointer
               text-emerald-700 dark:text-white bg-emerald-500/10 dark:bg-emerald-500/15 hover:bg-emerald-500/20 dark:hover:bg-emerald-500/25
               transition-colors"
        onclick={handlePlayAll}
      >
        <Icon icon="lucide:play" width={13} />
        Lire
      </button>

      <!-- Modifier les tags -->
      <button
        type="button"
        class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium cursor-pointer
               text-neutral-600 dark:text-neutral-300 hover:text-neutral-900 dark:hover:text-white hover:bg-neutral-100 dark:hover:bg-white/8
               transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
        onclick={handleEditTags}
        disabled={$batchStore.running}
        title={$batchStore.running ? $t("tags.batch_already_running") : $t("tags.edit")}
      >
        <Icon icon="lucide:tags" width={13} />
        {$t("tags.edit_short")}
      </button>

      <!-- Atelier -->
      <button
        type="button"
        class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium cursor-pointer
               text-neutral-600 dark:text-neutral-300 hover:text-neutral-900 dark:hover:text-white hover:bg-neutral-100 dark:hover:bg-white/8
               transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
        onclick={handleOpenWorkshop}
        disabled={$batchStore.running || !page.params.library_id}
        title={$t("workshop.title")}
      >
        <Icon icon="lucide:table-properties" width={13} />
        {$t("workshop.short")}
      </button>

      <!-- Add to queue -->
      <button
        type="button"
        class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium cursor-pointer
               text-neutral-600 dark:text-neutral-300 hover:text-neutral-900 dark:hover:text-white hover:bg-neutral-100 dark:hover:bg-white/8
               transition-colors"
        onclick={handleAddToQueue}
      >
        <Icon icon="lucide:list-end" width={13} />
        File
      </button>

      <!-- Add to playlist -->
      <div class="relative">
        <button
          type="button"
          class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium cursor-pointer
                 text-neutral-600 dark:text-neutral-300 hover:text-neutral-900 dark:hover:text-white hover:bg-neutral-100 dark:hover:bg-white/8
                 transition-colors"
          onclick={() => showPlaylistMenu = !showPlaylistMenu}
        >
          <Icon icon="lucide:list-music" width={13} />
          Playlist
        </button>

        {#if showPlaylistMenu}
          <button type="button" class="fixed inset-0 z-10 cursor-default"
                  onclick={() => showPlaylistMenu = false} aria-label="Fermer"></button>
          <div
            class="absolute bottom-full left-0 mb-2 z-20 w-48 py-1
                   bg-white/95 dark:bg-neutral-950/95 backdrop-blur-xl
                   border border-neutral-200 dark:border-white/10
                   rounded-xl shadow-2xl shadow-black/30
                   max-h-48 overflow-y-auto scrollbar-app"
            transition:fly={{ y: 8, duration: 150 }}
          >
            {#if playlists.length === 0}
              <p class="text-xs text-neutral-500 text-center py-3">Aucune playlist</p>
            {:else}
              {#each playlists as pl (pl.id)}
                <button
                  type="button"
                  class="w-full flex items-center gap-2.5 px-3 py-2 text-xs text-left cursor-pointer
                         text-neutral-600 dark:text-neutral-300 hover:bg-neutral-100 dark:hover:bg-white/8 transition-colors"
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
      </div>

      <div class="w-px h-5 bg-neutral-200 dark:bg-white/10"></div>

      <!-- Deselect all -->
      <button
        type="button"
        class="flex items-center gap-1.5 px-2 py-1.5 rounded-lg text-xs cursor-pointer
               text-neutral-500 hover:text-neutral-800 dark:hover:text-neutral-300 hover:bg-neutral-100 dark:hover:bg-white/5
               transition-colors"
        onclick={() => selectionStore.deselectAll()}
      >
        Tout désélectionner
      </button>

      <!-- Close -->
      <button
        type="button"
        class="flex items-center justify-center w-7 h-7 rounded-lg cursor-pointer
               text-neutral-500 hover:text-neutral-900 dark:hover:text-white hover:bg-neutral-100 dark:hover:bg-white/8
               transition-colors"
        onclick={() => selectionStore.stop()}
        aria-label="Quitter la sélection"
      >
        <Icon icon="lucide:x" width={14} />
      </button>
    </div>
  </div>
{/if}
