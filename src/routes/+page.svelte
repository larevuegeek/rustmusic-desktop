<script lang="ts">
import { onMount } from "svelte";
import { invoke } from "@tauri-apps/api/core";
import { recent } from "$lib/stores/recent/recent.store";
import { profilSelector } from "$lib/stores/profil/profil.store";
import { libraryStore } from "$lib/stores/library/library.store";
import { handleClickOpenDirectory } from "$lib/actions/player/PlayerAction";
import { handleAddDirectory } from "$lib/actions/library/LibraryAction";
import HomeEntete from "$lib/components/home/HomeEntete.svelte";
import HomeReprise from "$lib/components/home/HomeReprise.svelte";
import HomeMixes from "$lib/components/home/HomeMixes.svelte";
import HomeGenres from "$lib/components/home/HomeGenres.svelte";
import HomeAjouts from "$lib/components/home/HomeAjouts.svelte";
import HomeRecents from "$lib/components/home/HomeRecents.svelte";
import { settingsStore } from "$lib/stores/settings/settings.store";
import type { LibraryStats } from "$lib/types/ui/library/stats/LibraryStats";
import type { GenreView } from "$lib/types/ui/library/genre/GenreView";
import type { AlbumListView } from "$lib/types/ui/library/album/AlbumListView";

const profil = $derived($profilSelector.profilSelected);
// `!== 'false'` : sans réglage en base, la section est visible.
const montrerRecents = $derived($settingsStore.show_recent_in_home !== 'false' && $recent.length > 0);
// Un identifiant, pas l'objet : l'effet ne repart que si la bibliothèque change.
const libraryId = $derived(($libraryStore.librarySelected?.id as number | undefined) ?? null);

let stats = $state<LibraryStats | null>(null);
let genres = $state<GenreView[]>([]);
let albums = $state<AlbumListView[]>([]);

$effect(() => {
  const id = libraryId;
  stats = null; genres = []; albums = [];
  if (!id) return;

  let perime = false;
  invoke<LibraryStats>('get_library_stats', { libraryId: id })
    .then((s) => { if (!perime) stats = s; }).catch(() => {});
  invoke<GenreView[]>('get_genres', { libraryId: id })
    .then((g) => { if (!perime) genres = [...g].sort((a, b) => b.total_tracks - a.total_tracks); })
    .catch(() => {});
  invoke<AlbumListView[]>('get_albums', { libraryId: id, missingCover: null })
    .then((a) => { if (!perime) albums = a; }).catch(() => {});

  return () => { perime = true; };
});

onMount(() => { recent.refreshRecent(); });

function ajouterMusique() {
  if (libraryId) handleAddDirectory(libraryId, true);
  else handleClickOpenDirectory();
}
</script>

<div class="home-maquette py-6 px-4 md:px-10 scrollbar-app overflow-y-auto" style="height: calc(100vh - 250px);">
  <div class="@container max-w-[1400px] mx-auto flex flex-col gap-10 pb-10">

    <div class="-mb-4">
      <HomeEntete nom={profil?.name} {libraryId} {stats} onajouter={ajouterMusique} />
    </div>

    <HomeReprise onajouter={ajouterMusique} {libraryId} ancreRecents={montrerRecents} />

    {#if libraryId}
      <HomeMixes {libraryId} {genres} {albums} hires={stats?.quality_hires ?? 0} />

      <HomeGenres {libraryId} {genres} />
      <HomeAjouts {libraryId} {albums} />
    {/if}

    <!-- Tout en bas : la section vers laquelle mène « Tout voir » du hero. -->
    {#if montrerRecents}
      <HomeRecents />
    {/if}
  </div>
</div>
