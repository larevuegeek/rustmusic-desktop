<script lang="ts">
  // Titres likés : la page commune des playlists ; « Retirer des likes » dans le menu des morceaux.
  import { invoke } from "@tauri-apps/api/core";
  import { sourceFilter } from "#lib/stores/source/sourceFilter.store";
  import { untrack } from "svelte";
  import { t } from "#lib/i18n";
  import { liked } from "#lib/stores/playlist/like.store";
  import { profilSelector } from "#lib/stores/profil/profil.store";
  import { libraryStore } from "#lib/stores/library/library.store";
  import { resetTagCache } from "#lib/config/trackColumns";
  import PlaylistTracksView from "#lib/components/playlist/PlaylistTracksView.svelte";
  import { mapTrackOutsideLibrary } from "#lib/mapper/library/mapTrackOutsideLibrary";
  import type { TrackLikedView } from "#lib/types/ui/like/trackLikedView";
  import type { TrackListView } from "#lib/types/ui/library/track/TrackListView";

  let tracks = $state<TrackListView[]>([]);
  let loading = $state(true);
  let run = 0;

  const libraryId = $derived(
    $libraryStore.libraries.find((l) => l.is_default)?.id ?? $libraryStore.libraries[0]?.id ?? 0,
  );
  const profilId = $derived($profilSelector.profilSelected?.id ?? null);

  $effect(() => {
    const id = profilId;
    void $sourceFilter; // relu quand la source change
    if (id != null) untrack(() => load(id, ++run));
  });

  async function load(id: number, n: number) {
    loading = true;
    try {
      const likes = await invoke<TrackLikedView[]>("get_tracks_liked", { profilId: id, sourceId: $sourceFilter });
      const paths = [...new Set(likes.map((x) => x.path).filter(Boolean))];
      const view = paths.length ? await invoke<TrackListView[]>("get_tracks_view_by_paths", { paths }) : [];
      if (n !== run) return;
      resetTagCache();
      // Dans l'ordre des likes, le plus récent d'abord.
      const byPath = new Map(view.map((x) => [x.path, x]));
      // Hors bibliothèque, le titre aimé reste là.
      const entries = new Map(likes.map((l) => [l.path, l]));
      tracks = paths.map((p) => byPath.get(p) ?? mapTrackOutsideLibrary(entries.get(p) ?? { path: p }));
    } catch (e) {
      console.error("[likés] chargement :", e);
    } finally {
      if (n === run) loading = false;
    }
  }

  function remove(track: TrackListView) {
    liked.unlike(track.path);
    tracks = tracks.filter((x) => x.id !== track.id);
  }
</script>

<PlaylistTracksView
  title={$t("playlist_page.liked_title")}
  kicker={$t("playlist_page.collection")}
  color="#f43f5e"
  icon="material-symbols:favorite-rounded"
  {tracks}
  {loading}
  {libraryId}
  emptyTitle={$t("playlist_page.empty_liked")}
  emptyDesc={$t("playlist_page.empty_liked_desc")}
  onremove={remove}
  removeLabel={$t("playlist_page.remove_liked")}
/>
