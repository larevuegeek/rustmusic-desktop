<script lang="ts">
import { onMount } from "svelte";
import { invoke } from "@tauri-apps/api/core";
import { recent } from "$lib/stores/recent/recent.store";
import { profilSelector } from "$lib/stores/profil/profil.store";
import { libraryStore } from "$lib/stores/library/library.store";
import { importsTermines } from "$lib/stores/library/importProgress.store";
import { goto } from "$app/navigation";
import { t } from "$lib/i18n";
import { handleAddDirectory } from "$lib/actions/library/LibraryAction";
import HomeEntete from "$lib/components/home/HomeEntete.svelte";
import HomeReprise from "$lib/components/home/HomeReprise.svelte";
import HomeBienvenue from "$lib/components/home/HomeBienvenue.svelte";
import HomeMixes from "$lib/components/home/HomeMixes.svelte";
import HomeGenres from "$lib/components/home/HomeGenres.svelte";
import HomeAjouts from "$lib/components/home/HomeAjouts.svelte";
import HomeRecents from "$lib/components/home/HomeRecents.svelte";
import HomePlaylists from "$lib/components/home/HomePlaylists.svelte";
import { settingsStore } from "$lib/stores/settings/settings.store";
import type { LibraryStats } from "$lib/types/ui/library/stats/LibraryStats";
import type { GenreView } from "$lib/types/ui/library/genre/GenreView";
import type { GenreMix } from "$lib/types/ui/library/genre/GenreMix";
import type { AlbumListView } from "$lib/types/ui/library/album/AlbumListView";

const profil = $derived($profilSelector.profilSelected);
// `!== 'false'` : sans réglage en base, la section est visible.
const montrerRecents = $derived($settingsStore.show_recent_in_home !== 'false' && $recent.length > 0);
// Un identifiant, pas l'objet : l'effet ne repart que si la bibliothèque change.
const libraryId = $derived(($libraryStore.librarySelected?.id as number | undefined) ?? null);
// Aucune bibliothèque, ou une bibliothèque encore vide : accueil de bienvenue.
const accueilVide = $derived(!$libraryStore.isLoading
  && ($libraryStore.libraries.length === 0 || $libraryStore.librarySelected?.total_tracks === 0));

let stats = $state<LibraryStats | null>(null);
let genres = $state<GenreView[]>([]);
let mixGenres = $state<GenreMix[]>([]);
let albums = $state<AlbumListView[]>([]);

// Rechargé à chaque import validé, même arrêté.
let chargePour: number | null = null;
$effect(() => {
  const id = libraryId;
  void $importsTermines;
  if (id !== chargePour) { stats = null; genres = []; mixGenres = []; albums = []; }
  chargePour = id;
  if (!id) return;

  let perime = false;
  invoke<LibraryStats>('get_library_stats', { libraryId: id })
    .then((s) => { if (!perime) stats = s; }).catch(() => {});
  invoke<GenreView[]>('get_genres', { libraryId: id })
    .then((g) => { if (!perime) genres = [...g].sort((a, b) => b.total_tracks - a.total_tracks); })
    .catch(() => {});
  invoke<GenreMix[]>('get_mix_genres', { libraryId: id })
    .then((g) => { if (!perime) mixGenres = g; }).catch(() => {});
  invoke<AlbumListView[]>('get_albums', { libraryId: id, missingCover: null })
    .then((a) => { if (!perime) albums = a; }).catch(() => {});

  return () => { perime = true; };
});

onMount(() => { recent.refreshRecent(); });

function ajouterMusique() {
  if (libraryId) handleAddDirectory(libraryId, true);
  else goto('/import');
}
</script>

<div class="home-maquette py-6 px-4 md:px-10 h-full overflow-y-auto overscroll-contain scrollbar-app">
  <div class="@container max-w-[1400px] mx-auto flex flex-col gap-10 pb-10">

    <div class="-mb-4">
      <HomeEntete nom={profil?.name} {libraryId} {stats} onajouter={ajouterMusique}
                  sousTitre={accueilVide ? $t('home_welcome.subtitle') : null} bouton={!accueilVide} />
    </div>

    {#if accueilVide}
      <HomeBienvenue />
    {:else}
      <HomeReprise onajouter={ajouterMusique} {libraryId} ancreRecents={montrerRecents} />
    {/if}

    {#if libraryId && !accueilVide}
      <HomeMixes {libraryId} genres={mixGenres} {albums} hires={stats?.quality_hires ?? 0} />
    {/if}

    <!-- Les playlists sont au profil : elles s'affichent même sans bibliothèque. -->
    <HomePlaylists />

    {#if libraryId && !accueilVide}
      <HomeGenres {libraryId} {genres} />
      <HomeAjouts {libraryId} {albums} />
    {/if}

    <!-- Tout en bas : la section vers laquelle mène « Tout voir » du hero. -->
    {#if montrerRecents}
      <HomeRecents />
    {/if}
  </div>
</div>
