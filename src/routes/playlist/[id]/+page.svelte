<script lang="ts">
  // Une playlist, rangée ou auto : la page commune des playlists, avec ses actions (modifier, épingler, supprimer, retirer).
  import { page } from "$app/state";
  import { goto } from "$app/navigation";
  import { untrack } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import Icon from "@iconify/svelte";
  import { t, currentLocale } from "#lib/i18n";
  import { popinStore } from "#lib/stores/ui/popin.store";
  import { playlistStore } from "#lib/stores/playlist/playlist.store";
  import { libraryStore } from "#lib/stores/library/library.store";
  import { resetTagCache } from "#lib/config/trackColumns";
  import RoundButton from "#lib/components/ui/button/RoundButton.svelte";
  import PageState from "#lib/components/ui/layout/PageState.svelte";
  import PlaylistTracksView from "#lib/components/playlist/PlaylistTracksView.svelte";
  import EditPlaylistPopin from "#lib/components/playlist/popin/EditPlaylistPopin.svelte";
  import SmartPlaylistPopin from "#lib/components/playlist/smart/SmartPlaylistPopin.svelte";
  import { summarizeRules, type FieldOption, type Group, type Vocabulary } from "#lib/components/playlist/smart/types";
  import type { Playlist } from "#lib/types/db/playlist/Playlist";
  import type { PlaylistTrackView } from "#lib/types/ui/playlist/playlistTrackView";
  import type { TrackListView } from "#lib/types/ui/library/track/TrackListView";

  const playlistId = $derived(Number(page.params.id));

  let playlist = $state<Playlist | null>(null);
  let tracks = $state<TrackListView[]>([]);
  // Ligne de playlist de chaque morceau : c'est elle qu'on retire, pas le morceau.
  let itemIds = $state(new Map<string, number>());
  let loading = $state(true);
  let failed = $state(false);
  let rules = $state<string | null>(null);
  let run = 0;

  // La bibliothèque par défaut : ses tags et ses colonnes servent au tableau.
  const libraryId = $derived(
    $libraryStore.libraries.find((l) => l.is_default)?.id ?? $libraryStore.libraries[0]?.id ?? 0,
  );
  // Lu dans le store : un nom, une couleur ou une épingle changés ailleurs se voient ici aussi.
  const live = $derived($playlistStore.playlists.find((p) => p.id === playlistId) ?? null);
  const shown = $derived(live ?? playlist);
  const pinned = $derived(live?.pinned ?? false);

  let stamp: string | null = null;
  $effect(() => {
    const id = playlistId;
    stamp = null;
    if (id) untrack(() => load(id, ++run));
  });
  // Règles modifiées, titre ajouté depuis la bibliothèque : le store change, on recharge.
  $effect(() => {
    const next = live ? `${live.updated_at}|${live.track_count}` : null;
    if (next === null) return;
    if (stamp !== null && next !== stamp) untrack(() => load(playlistId, ++run));
    stamp = next;
  });

  async function load(id: number, n: number) {
    loading = true;
    failed = false;
    try {
      const [p, view, items] = await Promise.all([
        invoke<Playlist>("get_playlist", { playlistId: id }),
        invoke<TrackListView[]>("get_playlist_tracks_view", { playlistId: id }),
        invoke<PlaylistTrackView[]>("get_playlist_tracks", { playlistId: id }),
      ]);
      if (n !== run) return;
      resetTagCache();
      playlist = p;
      tracks = view ?? [];
      itemIds = new Map((items ?? []).map((x) => [x.library_track_id, x.playlist_item_id]));
      rules = p.is_smart ? await loadRules(id) : null;
    } catch (e) {
      if (n === run) failed = true;
      console.error("[playlist] chargement :", e);
    } finally {
      if (n === run) loading = false;
    }
  }

  // Une playlist auto affiche ses règles en clair : un nom resté d'une autre recette se voit tout de suite.
  async function loadRules(id: number): Promise<string | null> {
    try {
      const [voc, group] = await Promise.all([
        invoke<Vocabulary>("get_rule_vocabulary"),
        invoke<Group | null>("get_smart_playlist_rules", { playlistId: id }),
      ]);
      return group ? summarizeRules(group, voc, voc.fields as FieldOption[]) : null;
    } catch {
      return null;
    }
  }

  async function remove(track: TrackListView) {
    const itemId = itemIds.get(track.id);
    if (itemId == null) return;
    try {
      await invoke("remove_track_from_playlist", { playlistItemId: itemId });
      tracks = tracks.filter((x) => x.id !== track.id);
      await playlistStore.refresh();
    } catch (e) {
      console.error("[playlist] retrait :", e);
    }
  }

  function edit() {
    const p = shown;
    if (!p) return;
    if (p.is_smart) {
      popinStore.open($t("playlist.new_smart"), SmartPlaylistPopin, { playlistId: p.id }, { size: "xl", icon: "lucide:sparkles", flush: true });
    } else {
      popinStore.open($t("playlist_page.edit_playlist"), EditPlaylistPopin, { playlist: p });
    }
  }

  // ─── Suppression, après confirmation ───
  let confirmDelete = $state(false);
  const confirmParts = $derived.by(() => {
    const n = tracks.length;
    return $t(n === 1 ? "playlist_page.delete_confirm_one" : "playlist_page.delete_confirm_n")
      .replace("{n}", n.toLocaleString($currentLocale))
      .split("{name}");
  });
  async function deletePlaylist() {
    try {
      await playlistStore.removePlaylist(playlistId);
      goto("/playlists");
    } catch (e) {
      console.error("[playlist] suppression :", e);
    }
  }
</script>

{#if failed}
  <PageState icon="material-symbols:queue-music-rounded" message={$t("common.error_library")} lien={{ href: "/playlists", label: $t("playlists_view.all") }} />
{:else if !shown}
  <PageState chargement message={$t("common.loading")} />
{:else}
  <PlaylistTracksView
    title={shown.name}
    kicker={shown.is_smart ? $t("playlist.new_smart") : $t("home.playlist")}
    serverId={shown.server_id ?? null}
    subtitle={rules}
    color={shown.color}
    icon={shown.icon}
    {tracks}
    {loading}
    {libraryId}
    emptyTitle={$t("playlist_page.empty_playlist")}
    emptyDesc={$t("playlist_page.empty_playlist_desc")}
    onremove={shown.is_smart ? undefined : remove}
    removeLabel={$t("playlist_page.remove_playlist")}
  >
    {#snippet actions()}
      <!-- Une playlist auto se modifie par ses règles, pas par sa liste. -->
      <RoundButton icon={shown?.is_smart ? "lucide:sparkles" : "material-symbols:edit-outline-rounded"}
                   title={shown?.is_smart ? $t("playlist_page.rules") : $t("playlist_page.modify")} onclick={edit} />
      <RoundButton icon={pinned ? "material-symbols:keep-rounded" : "material-symbols:keep-outline-rounded"}
                   title={pinned ? $t("sidebar.unpin") : $t("sidebar.pin")}
                   onclick={() => playlistStore.setPinned(playlistId, !pinned)} />
      <RoundButton icon="material-symbols:delete-outline-rounded" title={$t("playlist_page.delete_playlist")} onclick={() => (confirmDelete = true)} />
    {/snippet}
  </PlaylistTracksView>
{/if}

{#if confirmDelete && shown}
  <div class="fixed inset-0 z-50 flex items-center justify-center">
    <button type="button" class="absolute inset-0 bg-black/60 backdrop-blur-sm cursor-default"
            onclick={() => (confirmDelete = false)} aria-label={$t("common.close")}></button>
    <div class="relative w-full max-w-sm mx-4 p-6 rounded-2xl border bg-(--rg-carte) border-(--rg-bd) shadow-2xl text-center">
      <span class="mx-auto mb-4 w-12 h-12 rounded-full bg-red-500/10 flex items-center justify-center">
        <Icon icon="material-symbols:delete-outline-rounded" width="22" class="text-red-500" />
      </span>
      <h3 class="mb-1 text-base font-bold text-(--rg-tx)">{$t("playlist_page.delete_playlist")}</h3>
      <p class="mb-6 text-sm text-(--rg-mu)">{confirmParts[0]}<strong class="text-(--rg-tx)">{shown.name}</strong>{confirmParts[1] ?? ""}</p>
      {#if shown.server_id}
        <!-- Playlist d'un serveur : la suppression part aussi là-bas. -->
        <p class="-mt-3 mb-6 flex items-center justify-center gap-1.5 text-[13px] font-medium text-sky-700 dark:text-sky-300">
          <Icon icon="material-symbols:cloud-outline-rounded" width="16" />{$t("servers.playlist_delete_note")}
        </p>
      {/if}
      <div class="flex gap-3">
        <button type="button" class="flex-1 h-10 rounded-xl text-sm font-semibold cursor-pointer text-(--rg-tx2) hover:bg-(--rg-hover)"
                onclick={() => (confirmDelete = false)}>{$t("common.cancel")}</button>
        <button type="button" class="flex-1 h-10 rounded-xl text-sm font-bold cursor-pointer bg-red-500 text-white hover:bg-red-600"
                onclick={deletePlaylist}>{$t("common.delete")}</button>
      </div>
    </div>
  </div>
{/if}
