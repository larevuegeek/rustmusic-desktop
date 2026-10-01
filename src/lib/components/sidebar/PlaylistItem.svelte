<script lang="ts">
  import type { Playlist } from "$lib/types/db/playlist/Playlist";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import { t, currentLocale } from "$lib/i18n";
  import { sidebarStore } from "$lib/stores/ui/sidebar.store";
  import PlaylistContextMenu from "$lib/components/ui/contextmenu/PlaylistContextMenu.svelte";
  import SidebarLigne from "./SidebarLigne.svelte";
  import SidebarVignette from "./SidebarVignette.svelte";

  let { playlist, couvertures = [], replie = false }: {
    playlist: Playlist;
    couvertures?: string[];
    replie?: boolean;
  } = $props();

  let menu = $state<{ x: number; y: number } | null>(null);

  const sousTitre = $derived.by(() => {
    const n = playlist.track_count;
    const genre = playlist.is_smart ? $t('sidebar.playlist_auto') : $t('sidebar.playlist');
    // Une playlist auto pas encore évaluée compte 0 : ne pas la dire vide.
    if (n === 0) return playlist.is_smart ? genre : `${genre} · ${$t('sidebar.empty')}`;
    const titres = n === 1 ? $t('home.track_one') : $t('home.tracks_n').replace('{n}', n.toLocaleString($currentLocale));
    return `${genre} · ${titres}`;
  });
</script>

<SidebarLigne
  titre={playlist.name}
  {sousTitre}
  {replie}
  actif={page.url.pathname === `/playlist/${playlist.id}`}
  onclick={() => { goto(`/playlist/${playlist.id}`); sidebarStore.close(); }}
  onmenu={(x, y) => menu = { x, y }}
>
  {#snippet vignette()}
    <SidebarVignette {couvertures} couleur={playlist.color} icone={playlist.icon} />
  {/snippet}
</SidebarLigne>

{#if menu}
  <PlaylistContextMenu {playlist} x={menu.x} y={menu.y} onclose={() => menu = null} />
{/if}
