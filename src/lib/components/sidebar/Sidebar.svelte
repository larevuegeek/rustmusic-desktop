<script lang="ts">
import SidebarItem from "./SidebarItem.svelte";
import Icon from "@iconify/svelte";
import { t } from "$lib/i18n";
import { goto } from "$app/navigation";
import { onMount } from "svelte";
import { liked, likedCount } from "$lib/stores/playlist/like.store";
import { popinStore } from "$lib/stores/ui/popin.store";
import AddLibraryPopin from "$lib/components/library/common/popin/AddLibraryPopin.svelte";
import AddPlaylistPopin from "$lib/components/playlist/popin/AddPlaylistPopin.svelte";
import SmartPlaylistPopin from "$lib/components/playlist/smart/SmartPlaylistPopin.svelte";
import PlaylistItem from "./PlaylistItem.svelte";
import { libraryStore } from "$lib/stores/library/library.store";
import { playlistStore } from "$lib/stores/playlist/playlist.store";
import { profilSelector } from "$lib/stores/profil/profil.store";
import { page } from "$app/state";
import { handleClickOpenDirectory, handleClickOpenFile } from "$lib/actions/player/PlayerAction";
import LibrarySelector from "$lib/components/library/common/LibrarySelector.svelte";
import LogoRustMusic from "$lib/components/ui/logo/LogoRustMusic.svelte";
import { recentCount } from "$lib/stores/recent/recent.store";
import { sidebarStore } from "$lib/stores/ui/sidebar.store";

const pathname = $derived(page.url.pathname);

// Fermer la sidebar sur mobile quand on navigue
function nav(path: string) {
  goto(path);
  sidebarStore.close();
}
const profil = $derived($profilSelector.profilSelected);
const profilColor = $derived(profil?.color ?? '#22c55e');

function isActive(section: string) {
  return pathname.endsWith(`/${section}`);
}

onMount(() => {
  liked.refresh();
  playlistStore.init();
});
</script>

<aside class="w-64 shrink-0 border-r border-neutral-300/70 dark:border-white/6
              h-full flex flex-col
              bg-neutral-50 dark:bg-zinc-950 md:bg-neutral-50/90 md:dark:bg-zinc-950/60 md:backdrop-blur-sm">

  <!-- Logo + Profil -->
  <div class="py-5 px-5 shrink-0 space-y-4">
    <h1 class="flex items-center justify-center">
      <LogoRustMusic width={170} />
    </h1>

    <div class="flex items-center gap-2.5 px-2 py-2 rounded-lg
                bg-white/80 dark:bg-white/3
                border border-neutral-200/80 dark:border-white/4
                shadow-sm shadow-black/3">
      <div class="w-7 h-7 rounded-full flex items-center justify-center shrink-0 text-[11px] font-bold text-white"
           style="background: {profilColor};">
        {profil?.name?.charAt(0)?.toUpperCase() ?? '?'}
      </div>
      <div class="min-w-0 flex-1">
        <div class="text-[12px] font-medium truncate text-neutral-700 dark:text-neutral-300">
          {profil?.name ?? 'Profil'}
        </div>
      </div>
      <div class="w-1.5 h-1.5 rounded-full bg-green-500"></div>
    </div>
  </div>

  <!-- Contenu scrollable -->
  <div class="flex-1 overflow-y-auto overflow-x-hidden px-4 pb-4
              smart-scroll">

    <!-- Navigation -->
    <div class="pb-1 mb-1">
      <SidebarItem onclick={() => nav("/")} icon="mynaui:home-solid" active={pathname === '/'}> {$t('nav.home')}</SidebarItem>
    </div>

    <!-- Séparateur -->
    <div class="h-px mx-3 mb-3 bg-linear-to-r from-transparent via-neutral-200/80 dark:via-neutral-700/30 to-transparent"></div>

    <!-- Import rapide -->
    <h3 class="text-[10px] font-semibold uppercase tracking-widest text-neutral-500 dark:text-neutral-500 px-2 mb-2">
      {$t('nav.open')}
    </h3>
    <div class="flex gap-1.5 px-1 mb-3">
      <button
        class="flex-1 flex items-center justify-center gap-1.5 px-2 py-2 rounded-lg cursor-pointer
               text-[11px] font-medium
               text-neutral-600 dark:text-neutral-400
               bg-white/80 dark:bg-white/3
               border border-neutral-200/80 dark:border-white/5
               hover:bg-neutral-100 dark:hover:bg-white/6
               hover:text-neutral-800 dark:hover:text-neutral-200
               hover:border-neutral-300/80 dark:hover:border-white/10
               shadow-sm shadow-black/3
               transition-all duration-150"
        onclick={() => handleClickOpenFile()}
      >
        <Icon icon="lucide:file-audio" width="13" height="13" />
        {$t('nav.import_file')}
      </button>
      <button
        class="flex-1 flex items-center justify-center gap-1.5 px-2 py-2 rounded-lg cursor-pointer
               text-[11px] font-medium
               text-neutral-600 dark:text-neutral-400
               bg-white/80 dark:bg-white/3
               border border-neutral-200/80 dark:border-white/5
               hover:bg-neutral-100 dark:hover:bg-white/6
               hover:text-neutral-800 dark:hover:text-neutral-200
               hover:border-neutral-300/80 dark:hover:border-white/10
               shadow-sm shadow-black/3
               transition-all duration-150"
        onclick={() => handleClickOpenDirectory()}
      >
        <Icon icon="lucide:folder-plus" width="13" height="13" />
        {$t('nav.import_folder')}
      </button>
    </div>

    <!-- Séparateur gradient -->
    <div class="h-px mx-2 mb-4 bg-linear-to-r from-transparent via-neutral-300/70 dark:via-neutral-700/30 to-transparent"></div>

    <!-- Bibliothèque -->
    <div class="mb-4">
      <div class="flex justify-between items-center mb-3 px-1">
        <h3 class="text-[10px] font-semibold uppercase tracking-widest text-neutral-500 dark:text-neutral-500">
          {$t('nav.library')}
        </h3>
        <button
          class="w-5 h-5 flex items-center justify-center rounded
                 text-neutral-400 hover:text-green-500 cursor-pointer
                 transition-colors"
          onclick={() => popinStore.open($t('library.create_library'), AddLibraryPopin, {})}
          aria-label="Ajouter une bibliothèque"
        >
          <Icon icon="lucide:plus" width="13" height="13" />
        </button>
      </div>

      {#if $libraryStore.isLoading}
        <div class="px-3 py-2 text-xs text-neutral-400">
          {$t('common.loading')}
        </div>
      {:else if $libraryStore.libraries.length === 0}
        <div class="mx-1">
          <div class="flex flex-col items-center text-center rounded-xl
                      border border-dashed border-neutral-200/80 dark:border-neutral-700/40
                      bg-neutral-50/50 dark:bg-neutral-900/30
                      px-3 py-4">
            <div class="relative mb-2.5">
              <div class="absolute inset-0 rounded-xl blur-md scale-150 opacity-30"
                   style="background: {profilColor};"></div>
              <div class="relative w-8 h-8 rounded-lg
                          bg-white dark:bg-neutral-800
                          border border-neutral-200/60 dark:border-neutral-700/40
                          flex items-center justify-center"
                   style="color: {profilColor};">
                <Icon icon="lucide:library" width={14} />
              </div>
            </div>

            <p class="text-[11px] font-medium text-neutral-600 dark:text-neutral-300 mb-2.5">
              {$t('library.no_library')}
            </p>

            <button
              class="w-full px-3 py-1.5 text-[10px] font-semibold rounded-lg
                     text-white cursor-pointer
                     hover:brightness-110 active:scale-[0.96]
                     shadow-sm transition-all duration-150
                     flex items-center justify-center gap-1"
              style="background: {profilColor}; box-shadow: 0 2px 8px {profilColor}30;"
              onclick={() => popinStore.open($t('library.create_library'), AddLibraryPopin, {})}
            >
              <Icon icon="lucide:plus" width={10} />
              {$t('library.create')}
            </button>
          </div>
        </div>
      {:else}
        <LibrarySelector />

        <SidebarItem onclick={() => nav(`/library/${$libraryStore.librarySelected?.id}/tracks`)} active={isActive("tracks")} icon="mynaui:music">{$t('library.tracks')}</SidebarItem>
        <SidebarItem onclick={() => nav(`/library/${$libraryStore.librarySelected?.id}/albums`)} active={isActive("albums")} icon="lucide:disc-album">{$t('library.albums')}</SidebarItem>
        <SidebarItem onclick={() => nav(`/library/${$libraryStore.librarySelected?.id}/artists`)} active={isActive("artists")} icon="lucide:mic-2">{$t('library.artists')}</SidebarItem>
        <SidebarItem onclick={() => nav(`/library/${$libraryStore.librarySelected?.id}/genres`)} active={isActive("genres")} icon="lucide:tag">{$t('library.genres')}</SidebarItem>
        <SidebarItem onclick={() => nav(`/library/${$libraryStore.librarySelected?.id}/folders`)} active={isActive("folders")} icon="lucide:folder-open">{$t('nav.explorer')}</SidebarItem>
      {/if}
    </div>

    <!-- Séparateur gradient -->
    <div class="h-px mx-2 mb-4 bg-linear-to-r from-transparent via-neutral-300/70 dark:via-neutral-700/30 to-transparent"></div>

    <!-- Playlists -->
    <div>
      <div class="flex justify-between items-center mb-3 px-1">
        <h3 class="text-[10px] font-semibold uppercase tracking-widest text-neutral-500 dark:text-neutral-500">
          {$t('nav.playlists')}
        </h3>
        <div class="flex items-center gap-0.5">
          <!-- Playlist intelligente : un bouton distinct plutôt qu'un choix
               dans une fenêtre. Ce sont deux gestes différents — poser une
               question à la bibliothèque, ou commencer une liste vide — et les
               confondre derrière un même bouton obligerait à choisir avant de
               savoir ce qu'on veut. -->
          <button
            class="w-5 h-5 flex items-center justify-center rounded
                   text-neutral-400 hover:text-violet-500 cursor-pointer
                   transition-colors"
            onclick={() => popinStore.open(
              'Playlist intelligente',
              SmartPlaylistPopin,
              {},
              { size: 'xl', icon: 'lucide:sparkles', flush: true },
            )}
            aria-label="Nouvelle playlist intelligente"
            title="Playlist intelligente — se remplit toute seule selon des règles"
          >
            <Icon icon="lucide:sparkles" width="13" height="13" />
          </button>
          <button
            class="w-5 h-5 flex items-center justify-center rounded
                   text-neutral-400 hover:text-green-500 cursor-pointer
                   transition-colors"
            onclick={() => popinStore.open($t('nav.playlists'), AddPlaylistPopin, {})}
            aria-label="Ajouter une playlist"
            title="Nouvelle playlist"
          >
            <Icon icon="lucide:plus" width="13" height="13" />
          </button>
        </div>
      </div>

      <!-- Liked -->
      <button
        class="group flex w-full items-center gap-3 px-2 py-1.5 rounded-lg text-left cursor-pointer
               transition-all duration-150
               {pathname === '/playlist/liked'
                 ? 'bg-green-500/10'
                 : 'hover:bg-neutral-100/80 dark:hover:bg-white/4'}"
        onclick={() => nav("/playlist/liked")}
      >
        <div class="w-9 h-9 rounded-lg flex items-center justify-center shrink-0
                    bg-linear-to-br from-rose-500/15 to-pink-500/25
                    border border-rose-500/15">
          <Icon icon="mynaui:heart-solid" width="16" height="16"
                class="text-rose-400" />
        </div>
        <div class="min-w-0 flex-1">
          <div class="font-medium text-[13px] truncate text-neutral-800 dark:text-neutral-200">
            {$t('playlist_page.liked_title')}
          </div>
          <div class="text-[11px] truncate text-neutral-400 dark:text-neutral-500">{$likedCount} titre{$likedCount !== 1 ? 's' : ''}</div>
        </div>
      </button>

      <!-- Recent -->
      <button
        class="group flex w-full items-center gap-3 px-2 py-1.5 rounded-lg text-left cursor-pointer
               transition-all duration-150
               {pathname === '/playlist/recent'
                 ? 'bg-green-500/10'
                 : 'hover:bg-neutral-100/80 dark:hover:bg-white/4'}"
        onclick={() => nav("/playlist/recent")}
      >
        <div class="w-9 h-9 rounded-lg flex items-center justify-center shrink-0
                    bg-linear-to-br from-sky-500/15 to-cyan-500/25
                    border border-sky-500/15">
          <Icon icon="mynaui:clock-8" width="16" height="16"
                class="text-sky-400" />
        </div>
        <div class="min-w-0 flex-1">
          <div class="font-medium text-[13px] truncate text-neutral-800 dark:text-neutral-200">
            {$t('playlist_page.recent_title')}
          </div>
          <div class="text-[11px] truncate text-neutral-400 dark:text-neutral-500">{$recentCount} titre{$recentCount !== 1 ? 's' : ''}</div>
        </div>
      </button>

      <!-- Custom playlists -->
      {#each $playlistStore.playlists as playlist (playlist.id)}
        <PlaylistItem {playlist} />
      {/each}
    </div>
  </div>

</aside>
