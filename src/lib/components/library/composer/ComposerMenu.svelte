<script lang="ts">
  // Menu « … » d'un compositeur (carte, ligne) : ses morceaux chargés à la demande.
  import { invoke } from "@tauri-apps/api/core";
  import CollectionContextMenu from "#lib/components/ui/contextmenu/CollectionContextMenu.svelte";
  import { handleComposerPlay } from "#lib/actions/queue/QueueAction";
  import type { TrackListView } from "#lib/types/ui/library/track/TrackListView";

  let { libraryId, name, x, y, onclose }: { libraryId: number; name: string; x: number; y: number; onclose: () => void } = $props();
</script>

<CollectionContextMenu title={name} type="composer" {libraryId} {x} {y} {onclose}
                       loadTracks={() => invoke<TrackListView[]>("get_tracks_by_composer", { libraryId, name })}
                       onshuffle={() => void handleComposerPlay(libraryId, name, true)} />
