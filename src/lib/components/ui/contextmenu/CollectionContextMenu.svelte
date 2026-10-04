<script lang="ts">
  import Icon from "@iconify/svelte";
  import { queueState } from "$lib/stores/queue/queueState.store";
  import { toasts } from "$lib/stores/ui/toast.store";
  import { toQueueTracks, type TrackLike } from "$lib/helper/tools/queueTools";
  import { playlistStore } from "$lib/stores/playlist/playlist.store";
  import { playlistsRangees } from "$lib/stores/playlist/couvertures.store";
  import { invoke } from "@tauri-apps/api/core";
  import { t, currentLocale } from "$lib/i18n";
  import { open } from "@tauri-apps/plugin-dialog";
  import type { Playlist } from "$lib/types/db/playlist/Playlist";
  import { libraryContentStore } from "$lib/stores/library/libraryContent.store";
  import DeezerCoverSearchPopin from "$lib/components/library/common/popin/DeezerCoverSearchPopin.svelte";
  import { pinsStore, isPinned } from "$lib/stores/library/pins.store";

  type Props = {
    title: string;
    type: "album" | "playlist" | "genre";
    loadTracks: () => Promise<TrackLike[]>;
    x: number;
    y: number;
    onclose: () => void;
    oncover?: () => void;
    albumId?: string | null;
    artistName?: string | null;
    /** Requis pour épingler l'album dans la barre latérale. */
    libraryId?: number | null;
    /**
     * Ouvre l'atelier de tags sur cette collection.
     *
     * Fourni par l'appelant plutôt que construit ici : le menu ne sait pas
     * d'où viennent ses morceaux — un album les charge par identifiant, une
     * playlist autrement.
     */
    onedittags?: () => void;
  };

  let { title, type, loadTracks, x, y, onclose, oncover, albumId = null, artistName = null, libraryId = null, onedittags }: Props = $props();

  const epingle = $derived(isPinned($pinsStore, "album", albumId));

  // Lu avant `onclose()` : un appelant peut tirer ces props d'un état remis à `null`.
  async function handleTogglePin() {
    const m = { albumId, libraryId, title, epingle };
    onclose();
    if (!m.albumId || m.libraryId == null) return;
    try {
      await pinsStore.setPinned("album", m.libraryId, m.albumId, !m.epingle);
      toasts.push({ type: "success", title: m.title, message: $t(m.epingle ? "sidebar.unpinned" : "sidebar.pinned") });
    } catch (e) {
      toasts.push({ type: "error", title: $t("notify.error"), message: String(e) });
    }
  }

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
        toasts.push({ type: "success", title: $t("tags.cover"), message: $t("notify.cover_fetched") });
      } else {
        toasts.push({ type: "error", title: $t("tags.cover"), message: $t("notify.cover_not_found") });
      }
    } catch (e) {
      toasts.push({ type: "error", title: $t("notify.error"), message: String(e) });
    } finally {
      loading = false;
      onclose();
    }
  }

  async function handleChooseCover() {
    if (!albumId) return;
    const selected = await open({
      multiple: false,
      title: $t("actions.dialog_choose_cover"),
      filters: [{ name: $t("actions.dialog_images"), extensions: ['jpg', 'jpeg', 'png', 'webp'] }],
    });
    if (!selected) return;
    loading = true;
    try {
      await invoke('set_album_cover', { albumId, imagePath: selected });
      await libraryContentStore.refresh();
      oncover?.();
      toasts.push({ type: "success", title: $t("tags.cover"), message: $t("notify.cover_updated") });
    } catch (e) {
      toasts.push({ type: "error", title: $t("notify.error"), message: String(e) });
    } finally {
      loading = false;
      onclose();
    }
  }

  let playlists = $derived($playlistsRangees);

  let menuStyle = $derived.by(() => {
    const menuWidth = 220;
    const menuHeight = 320;
    let posX = x;
    let posY = y;

    if (posX + menuWidth > window.innerWidth) posX = window.innerWidth - menuWidth - 8;
    if (posY + menuHeight > window.innerHeight) posY = window.innerHeight - menuHeight - 8;

    return `left: ${posX}px; top: ${posY}px;`;
  });

  const playAllKey = $derived(type === "album" ? "menu.play_all_album" : type === "genre" ? "menu.play_all_genre" : "menu.play_all_playlist");

  /** « 1 morceau » / « n morceaux », nombre au format de la langue. */
  function count(n: number, one: string, many: string, extra: Record<string, string> = {}): string {
    let s = $t(n === 1 ? one : many).replace("{n}", n.toLocaleString($currentLocale));
    for (const [k, v] of Object.entries(extra)) s = s.replace(`{${k}}`, v);
    return s;
  }

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
      toasts.push({ type: "success", title: $t("notify.added_next"), message: count(tracks.length, "notify.next_one", "notify.next_n") });
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
      toasts.push({ type: "success", title: $t("notify.queued"), message: count(tracks.length, "notify.queued_one", "notify.queued_n") });
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
        const params: Record<string, unknown> = { playlistId: pl.id, path: track.path };
        if (typeof track.id === 'string' && track.id) {
          params.libraryTrackId = track.id;
        }
        try {
          await invoke('add_track_to_playlist', params);
          added++;
        } catch { /* skip duplicates */ }
      }

      await playlistStore.refresh();
      toasts.push({ type: "success", title: $t("notify.added"), message: count(added, "notify.added_to_one", "notify.added_to_n", { name: pl.name }) });
    } catch (e) {
      toasts.push({ type: "error", title: $t("notify.error"), message: String(e) });
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
  <!-- Header -->
  <div class="px-3.5 py-1.5 mb-1">
    <p class="text-[10px] uppercase tracking-widest text-(--rg-mu) truncate">{title}</p>
  </div>

  <button
    class="w-full flex items-center gap-2.5 px-3.5 py-2 text-sm text-left cursor-pointer
           text-(--rg-tx) hover:bg-green-500/15 hover:text-(--rg-gtx) transition-colors
           disabled:opacity-50"
    onclick={handlePlayAll}
    disabled={loading}
  >
    <Icon icon="lucide:play" width="14" class="opacity-60" />
    {$t(playAllKey)}
  </button>

  <button
    class="w-full flex items-center gap-2.5 px-3.5 py-2 text-sm text-left cursor-pointer
           text-(--rg-tx) hover:bg-(--rg-s2) dark:hover:bg-[#252b28] transition-colors
           disabled:opacity-50"
    onclick={handleAddAllNext}
    disabled={loading}
  >
    <Icon icon="lucide:list-start" width="14" class="opacity-60" />
    {$t("track_view.play_next")}
  </button>

  <button
    class="w-full flex items-center gap-2.5 px-3.5 py-2 text-sm text-left cursor-pointer
           text-(--rg-tx) hover:bg-(--rg-s2) dark:hover:bg-[#252b28] transition-colors
           disabled:opacity-50"
    onclick={handleAddAllToQueue}
    disabled={loading}
  >
    <Icon icon="lucide:list-end" width="14" class="opacity-60" />
    {$t("menu.enqueue_all")}
  </button>

  {#if onedittags}
    <div class="h-px mx-3 my-1 bg-(--rg-bd) dark:bg-white/8"></div>
    <button
      class="w-full flex items-center gap-2.5 px-3.5 py-2 text-sm text-left cursor-pointer
             text-(--rg-tx) hover:bg-(--rg-s2) dark:hover:bg-[#252b28] transition-colors"
      onclick={() => { onclose(); onedittags(); }}
    >
      <Icon icon="lucide:table-properties" width="14" class="opacity-60" />
      {$t('workshop.fix_tags')}
    </button>
  {/if}

  <!-- Séparateur -->
  <div class="h-px mx-3 my-1 bg-(--rg-bd) dark:bg-white/8"></div>

  <!-- Ajouter à une playlist -->
  <button
    class="w-full flex items-center justify-between px-3.5 py-2 text-sm text-left cursor-pointer
           text-(--rg-tx) hover:bg-(--rg-s2) dark:hover:bg-[#252b28] transition-colors"
    onclick={() => showPlaylistSub = !showPlaylistSub}
  >
    <span class="flex items-center gap-2.5">
      <Icon icon="lucide:list-music" width="14" class="opacity-60" />
      {$t("menu.add_to_playlist")}
    </span>
    <Icon icon={showPlaylistSub ? "lucide:chevron-down" : "lucide:chevron-right"} width="12" class="opacity-40" />
  </button>

  {#if showPlaylistSub}
    <div class="border-t border-(--rg-bd) dark:border-white/6 bg-(--rg-hover)">
      {#if playlists.length === 0}
        <div class="flex flex-col items-center py-4 px-3">
          <Icon icon="lucide:list-music" width="16" class="text-(--rg-mu2) mb-1.5" />
          <p class="text-[11px] text-(--rg-mu)">{$t("menu.no_playlist")}</p>
        </div>
      {:else}
        {#each playlists as pl, i (pl.id)}
          {#if i > 0}
            <div class="h-px mx-2 bg-(--rg-bd) dark:bg-white/8"></div>
          {/if}
          <button
            class="w-full flex items-center gap-2.5 pl-6 pr-3 py-2 text-[12px] text-left cursor-pointer
                   text-(--rg-tx2) hover:bg-(--rg-s2) dark:hover:bg-[#252b28] transition-colors
                   disabled:opacity-50"
            onclick={() => handleAddToPlaylist(pl)}
            disabled={loading}
          >
            <Icon icon={pl.icon ?? "lucide:list-music"} width="13"
                  style="color: {pl.color ?? '#22c55e'};" class="opacity-70" />
            <span class="truncate">{pl.name}</span>
            <span class="ml-auto text-[10px] text-(--rg-mu2)">{pl.track_count}</span>
          </button>
        {/each}
      {/if}
    </div>
  {/if}

  {#if type === 'album' && albumId}
    <!-- Séparateur -->
    <div class="h-px mx-3 my-1 bg-(--rg-bd) dark:bg-white/8"></div>

    {#if libraryId != null}
      <button
        class="w-full flex items-center gap-2.5 px-3.5 py-2 text-sm text-left cursor-pointer
               text-(--rg-tx) hover:bg-(--rg-s2) dark:hover:bg-[#252b28] transition-colors"
        onclick={handleTogglePin}
      >
        <Icon icon={epingle ? "material-symbols:keep-off-outline-rounded" : "material-symbols:keep-outline-rounded"} width="15" class="opacity-60" />
        {$t(epingle ? "sidebar.unpin" : "sidebar.pin")}
      </button>
    {/if}

    <button
      class="w-full flex items-center justify-between px-3.5 py-2 text-sm text-left cursor-pointer
             text-(--rg-tx) hover:bg-(--rg-s2) dark:hover:bg-[#252b28] transition-colors"
      onclick={() => showCoverSub = !showCoverSub}
    >
      <span class="flex items-center gap-2.5">
        <Icon icon="lucide:image" width="14" class="opacity-60" />
        {$t("menu.change_cover")}
      </span>
      <Icon icon={showCoverSub ? "lucide:chevron-down" : "lucide:chevron-right"} width="12" class="opacity-40" />
    </button>

    {#if showCoverSub}
      <div class="border-t border-(--rg-bd) dark:border-white/6 bg-(--rg-hover)">
        <button
          class="w-full flex items-center gap-2.5 pl-6 pr-3 py-2 text-[12px] text-left cursor-pointer
                 text-(--rg-tx2) hover:bg-(--rg-s2) dark:hover:bg-[#252b28] transition-colors
                 disabled:opacity-50"
          onclick={handleFetchCover}
          disabled={loading}
        >
          <Icon icon="lucide:wand-sparkles" width="13" class="opacity-60" />
          {$t("menu.cover_from_deezer")}
        </button>

        <div class="h-px mx-2 bg-(--rg-bd) dark:bg-white/8"></div>

        <button
          class="w-full flex items-center gap-2.5 pl-6 pr-3 py-2 text-[12px] text-left cursor-pointer
                 text-(--rg-tx2) hover:bg-(--rg-s2) dark:hover:bg-[#252b28] transition-colors"
          onclick={() => { showDeezerSearch = true; }}
        >
          <Icon icon="lucide:search" width="13" class="opacity-60" />
          {$t("menu.cover_search_deezer")}
        </button>

        <div class="h-px mx-2 bg-(--rg-bd) dark:bg-white/8"></div>

        <button
          class="w-full flex items-center gap-2.5 pl-6 pr-3 py-2 text-[12px] text-left cursor-pointer
                 text-(--rg-tx2) hover:bg-(--rg-s2) dark:hover:bg-[#252b28] transition-colors
                 disabled:opacity-50"
          onclick={handleChooseCover}
          disabled={loading}
        >
          <Icon icon="lucide:folder-open" width="13" class="opacity-60" />
          {$t("menu.cover_pick_file")}
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
