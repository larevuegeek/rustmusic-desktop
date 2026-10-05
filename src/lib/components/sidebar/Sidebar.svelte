<script lang="ts">
import { couverturesPlaylists } from "#lib/stores/playlist/couvertures.store";
import Icon from "@iconify/svelte";
import { t, currentLocale } from "#lib/i18n";
import { goto } from "$app/navigation";
import { page } from "$app/state";
import { onMount } from "svelte";
import { liked, likedCount } from "#lib/stores/playlist/like.store";
import { popinStore } from "#lib/stores/ui/popin.store";
import AddLibraryPopin from "#lib/components/library/common/popin/AddLibraryPopin.svelte";
import AddPlaylistPopin from "#lib/components/playlist/popin/AddPlaylistPopin.svelte";
import SmartPlaylistPopin from "#lib/components/playlist/smart/SmartPlaylistPopin.svelte";
import PlaylistZoneMenu from "#lib/components/ui/contextmenu/PlaylistZoneMenu.svelte";
import LibraryZoneMenu from "#lib/components/ui/contextmenu/LibraryZoneMenu.svelte";
import OpenZoneMenu from "#lib/components/ui/contextmenu/OpenZoneMenu.svelte";
import LibrarySelector from "#lib/components/library/common/LibrarySelector.svelte";
import { libraryStore } from "#lib/stores/library/library.store";
import { playlistStore } from "#lib/stores/playlist/playlist.store";
import { pinsStore } from "#lib/stores/library/pins.store";
import { recentCount } from "#lib/stores/recent/recent.store";
import { sidebarStore } from "#lib/stores/ui/sidebar.store";
import { settingsStore } from "#lib/stores/settings/settings.store";
import { handleClickOpenDirectory, handleClickOpenFile } from "#lib/actions/player/PlayerAction";
import PlaylistItem from "./PlaylistItem.svelte";
import SidebarBouton from "./SidebarBouton.svelte";
import SidebarEpingles from "./SidebarEpingles.svelte";
import SidebarLigne from "./SidebarLigne.svelte";
import SidebarNav from "./SidebarNav.svelte";
import SidebarPied from "./SidebarPied.svelte";
import SidebarTitre from "./SidebarTitre.svelte";
import SidebarVignette from "./SidebarVignette.svelte";

const pathname = $derived(page.url.pathname);
const replie = $derived($settingsStore.sidebar_collapsed === 'true');
const libraryId = $derived($libraryStore.librarySelected?.id ?? null);

// `!== 'false'` : sans réglage en base, le défaut est « visible ».
// Sans cœur, plus rien ne remplit les likés : la liste suit le bouton favori.
const favorisActifs = $derived($settingsStore.show_favorites !== 'false');
const montrerLikes = $derived(favorisActifs && $settingsStore.show_liked_in_playlists !== 'false');
const montrerRecents = $derived($settingsStore.show_recent_in_playlists !== 'false');
const montrerBoutonsOuvrir = $derived($settingsStore.show_open_buttons === 'true');

// Pastille sur le « … » quand un raccourci est masqué : c'est là qu'on le retrouve.
const raccourcisMasques = $derived(
  (favorisActifs && $settingsStore.show_liked_in_playlists === 'false' ? 1 : 0) +
  ($settingsStore.show_recent_in_playlists === 'false' ? 1 : 0)
);

let menuZone = $state<{ x: number; y: number } | null>(null);
let menuBibliotheque = $state<{ x: number; y: number } | null>(null);
let menuOuvrir = $state<{ x: number; y: number } | null>(null);

const sousMenu = (e: MouseEvent) => {
  const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
  return { x: r.right, y: r.bottom + 4 };
};

function compte(genre: string, n: number): string {
  if (n === 0) return `${genre} · ${$t('sidebar.empty')}`;
  const titres = n === 1 ? $t('home.track_one') : $t('home.tracks_n').replace('{n}', n.toLocaleString($currentLocale));
  return `${genre} · ${titres}`;
}

function nav(path: string) {
  goto(path);
  sidebarStore.close();
}

$effect(() => { pinsStore.load(libraryId); });

onMount(() => {
  liked.refresh();
  playlistStore.init();
});
</script>

<aside
  class="sidebar h-full shrink-0 flex flex-col gap-3 overflow-y-auto overflow-x-hidden smart-scroll
         bg-(--sb-bg) border-r border-(--sb-sep) transition-[width] duration-200
         {replie ? 'w-19 items-center pt-3 pb-3' : 'w-72 px-4 pt-4 pb-3'}"
>
  <!-- ─── Haut : ouvrir, navigation, bibliothèque. Le logo est dans la barre de titre. ─── -->
  <div class="shrink-0 flex flex-col gap-3 {replie ? 'items-center' : ''}">
    <!-- Affiché par défaut ; se masque depuis son « … » ou Réglages → Apparence. -->
    {#if montrerBoutonsOuvrir && !replie}
      <div class="flex items-center gap-2">
        {#each [
          { icone: 'material-symbols:audio-file-outline-rounded', label: $t('nav.import_file'), action: handleClickOpenFile },
          { icone: 'material-symbols:create-new-folder-outline-rounded', label: $t('nav.import_folder'), action: handleClickOpenDirectory },
        ] as bouton (bouton.icone)}
          <button
            type="button"
            class="flex-1 h-9 flex items-center justify-center gap-1.5 rounded-lg cursor-pointer transition-colors
                   text-xs font-semibold bg-(--sb-s1) border border-(--sb-bd) text-(--sb-tx2)
                   hover:border-(--sb-bd2) hover:text-(--sb-tx)"
            onclick={() => bouton.action()}
          >
            <Icon icon={bouton.icone} width="17" />
            {bouton.label}
          </button>
        {/each}
        <SidebarBouton icon="material-symbols:more-horiz" label={$t('nav.open')} onclick={(e) => menuOuvrir = sousMenu(e)} />
      </div>
      <!-- Part avec la section. -->
      <div class="h-px bg-linear-to-r from-transparent via-(--sb-mu)/40 to-transparent" role="presentation"></div>
    {/if}

    <SidebarNav {replie} />

    {#if !$libraryStore.isLoading && $libraryStore.libraries.length === 0}
      <button
        type="button"
        class="flex items-center gap-3 rounded-xl cursor-pointer transition-colors
               bg-(--sb-s1) border border-dashed border-(--sb-bd2) hover:border-(--sb-g)
               {replie ? 'p-1.75' : 'w-full py-2 pr-2.5 pl-2'}"
        onclick={() => popinStore.open($t('library.create_library'), AddLibraryPopin, {})}
        title={$t('library.create_library')}
      >
        <span class="w-9 h-9 rounded-[9px] shrink-0 flex items-center justify-center bg-(--sb-gbg) text-(--sb-g)">
          <Icon icon="material-symbols:add-rounded" width="22" />
        </span>
        {#if !replie}
          <span class="flex flex-col gap-px min-w-0 text-left">
            <b class="text-sm font-bold truncate text-(--sb-tx)">{$t('library.no_library')}</b>
            <small class="text-xs truncate text-(--sb-g) font-semibold">{$t('library.create')}</small>
          </span>
        {/if}
      </button>
    {:else}
      <div class="flex gap-2">
        <LibrarySelector {replie} />
        <!-- Ses réglages vivent dans le « … » de Fichier / Dossier ; masquée, il reprend le relais. -->
        {#if !replie && !montrerBoutonsOuvrir}
          <button
            type="button"
            class="w-9 shrink-0 rounded-xl flex items-center justify-center cursor-pointer transition-colors
                   bg-(--sb-s1) border border-(--sb-bd) text-(--sb-tx2) hover:border-(--sb-bd2) hover:text-(--sb-tx)"
            onclick={(e) => menuBibliotheque = sousMenu(e)}
            title={$t('sidebar.library_options')}
            aria-label={$t('sidebar.library_options')}
          >
            <Icon icon="material-symbols:more-vert" width="20" />
          </button>
        {/if}
      </div>
    {/if}

  </div>

  <!-- ─── Milieu : playlists et épinglés, seule zone qui défile. Repliée, sans
       barre de défilement : elle mangerait la colonne. ─── -->
  <div
    class="flex flex-col border-t border-(--sb-sep)
           {replie
             ? 'flex-1 min-h-0 w-full items-center gap-2 py-2 overflow-y-auto overflow-x-hidden [scrollbar-width:none]'
             : 'flex-[1_0_140px] min-h-35 gap-0.5 -mx-2 px-2 pb-8 overflow-y-auto overflow-x-hidden smart-scroll sb-liste-fondu'}"
  >
    {#if !replie}
      <SidebarTitre titre={$t('nav.playlists')} aide={$t('playlists_view.all')} onclick={() => nav('/playlists')}>
        {#snippet actions()}
          <SidebarBouton
            icon="material-symbols:auto-awesome-outline-rounded"
            label={$t('playlist.new_smart')}
            taille={16}
            onclick={() => popinStore.open($t('playlist.new_smart'), SmartPlaylistPopin, {},
              { size: 'xl', icon: 'lucide:sparkles', flush: true })}
          />
          <SidebarBouton
            icon="material-symbols:add-rounded"
            label={$t('playlist.new')}
            taille={16}
            onclick={() => popinStore.open($t('nav.playlists'), AddPlaylistPopin, {})}
          />
          <SidebarBouton
            icon="material-symbols:more-horiz"
            label={$t('playlist.options')}
            taille={16}
            onclick={(e) => menuZone = sousMenu(e)}
          >
            {#if raccourcisMasques > 0}
              <span class="absolute top-1 right-1 w-1.5 h-1.5 rounded-full bg-(--sb-g)"></span>
            {/if}
          </SidebarBouton>
        {/snippet}
      </SidebarTitre>
    {/if}

    {#each $playlistStore.playlists.filter((p) => p.pinned && !p.is_mix) as playlist (playlist.id)}
      <PlaylistItem {playlist} {replie} couvertures={$couverturesPlaylists[`playlist:${playlist.id}`] ?? []} />
    {/each}

    {#if montrerLikes}
      <SidebarLigne
        titre={$t('playlist_page.liked_title')}
        sousTitre={compte($t('sidebar.playlist'), $likedCount)}
        {replie}
        actif={pathname === '/playlist/liked'}
        onclick={() => nav('/playlist/liked')}
      >
        {#snippet vignette()}
          <SidebarVignette couvertures={$couverturesPlaylists.liked ?? []} couleur="#f43f5e" icone="material-symbols:favorite-rounded" />
        {/snippet}
      </SidebarLigne>
    {/if}

    {#if montrerRecents}
      <SidebarLigne
        titre={$t('playlist_page.recent_title')}
        sousTitre={compte($t('sidebar.history'), $recentCount)}
        {replie}
        actif={pathname === '/playlist/recent'}
        onclick={() => nav('/playlist/recent')}
      >
        {#snippet vignette()}
          <SidebarVignette couvertures={$couverturesPlaylists.recent ?? []} couleur="#0ea5e9" icone="material-symbols:history-rounded" />
        {/snippet}
      </SidebarLigne>
    {/if}

    {#if libraryId != null}
      <SidebarEpingles {libraryId} {replie} />
    {/if}
  </div>

  <!-- ─── Pied : état du scan et profil ─── -->
  <SidebarPied {replie} />
</aside>

{#if menuZone}
  <PlaylistZoneMenu x={menuZone.x} y={menuZone.y} onclose={() => menuZone = null} />
{/if}

{#if menuBibliotheque}
  <LibraryZoneMenu x={menuBibliotheque.x} y={menuBibliotheque.y} onclose={() => menuBibliotheque = null} />
{/if}

{#if menuOuvrir}
  <OpenZoneMenu x={menuOuvrir.x} y={menuOuvrir.y} onclose={() => menuOuvrir = null} />
{/if}
