<script lang="ts">
import RecentTrackItem from "$lib/components/recent/RecentTrackItem.svelte";
import Icon from "@iconify/svelte";
import { recent, recentCount } from "$lib/stores/recent/recent.store";
import { queueState } from "$lib/stores/queue/queueState.store";
import { profilSelector } from "$lib/stores/profil/profil.store";
import { libraryStore } from "$lib/stores/library/library.store";
import { playlistStore } from "$lib/stores/playlist/playlist.store";
import { liked, likedCount } from "$lib/stores/playlist/like.store";
import { playerService } from "$lib/services/player/player.service";
import { player } from "$lib/stores/player/player.store";
import openAudioFile, { handleClickOpenDirectory } from "$lib/actions/player/PlayerAction";
import { handleAddFiles, handleAddDirectory } from "$lib/actions/library/LibraryAction";
import { goto } from "$app/navigation";
import { t } from "$lib/i18n";
import { settingsStore } from "$lib/stores/settings/settings.store";

const selectedLibrary = $derived($libraryStore.librarySelected);

// `!== 'false'` : sans réglage en base, le défaut est « visible ».
const montrerLikes = $derived($settingsStore.show_liked_in_home !== 'false');
const montrerRecents = $derived($settingsStore.show_recent_in_home !== 'false');

// La grille se redécoupe selon le nombre de cartes restantes. Classes en
// toutes lettres : Tailwind ne sait pas fabriquer `grid-cols-${n}`.
const nbCartes = $derived(2 + (montrerLikes ? 1 : 0) + (montrerRecents ? 1 : 0));
const colonnesStats = $derived(
  nbCartes === 4 ? 'grid-cols-2 md:grid-cols-4'
  : nbCartes === 3 ? 'grid-cols-2 md:grid-cols-3'
  : 'grid-cols-2'
);

const profil = $derived($profilSelector.profilSelected);
const profilColor = $derived(profil?.color ?? '#22c55e');
const queueTracks = $derived($queueState.tracks);
const isPlaying = $derived($player?.status === "playing");

function getGreeting(): string {
  const h = new Date().getHours();
  if (h < 6) return $t('home.greeting_night');
  if (h < 12) return $t('home.greeting_morning');
  if (h < 18) return $t('home.greeting_afternoon');
  return $t('home.greeting_evening');
}

const handleClickOpenFile = async () => {
  await openAudioFile();
};
</script>

<div class="py-5 px-4 md:px-10 scrollbar-app overflow-y-auto" style="height: calc(100vh - 250px);">

  <!-- HERO -->
  <div class="relative overflow-hidden rounded-2xl px-5 md:px-8 py-8 md:py-10 mb-8"
       style="background: linear-gradient(135deg, {profilColor}08 0%, {profilColor}15 50%, transparent 100%);">

    <!-- Orbes décoratifs -->
    <div class="absolute -top-16 -right-16 w-64 h-64 rounded-full blur-[80px] opacity-20 pointer-events-none"
         style="background: {profilColor};"></div>
    <div class="absolute -bottom-12 left-1/4 w-48 h-48 rounded-full blur-[60px] opacity-10 pointer-events-none"
         style="background: {profilColor};"></div>

    <div class="relative">
      <!-- Badge -->
      <div class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full mb-5
                  border text-[11px] font-medium"
           style="background: {profilColor}15; border-color: {profilColor}25; color: {profilColor};">
        <span class="w-1.5 h-1.5 rounded-full animate-pulse" style="background: {profilColor};"></span>
        {$t('home.ready')}
      </div>

      <h1 class="text-3xl font-bold tracking-tight text-neutral-800 dark:text-neutral-50 mb-1.5">
        {getGreeting()}, {profil?.name ?? 'Utilisateur'}
      </h1>

      <p class="text-sm text-neutral-500 dark:text-neutral-400 mb-7 max-w-md leading-relaxed">
        {$t('home.tagline')}
      </p>

      <!-- Actions -->
      <div class="flex items-center gap-1.5 flex-wrap">
        {#if queueTracks.length > 0}
          <button
            class="flex items-center gap-2 px-5 py-2 rounded-full text-sm font-semibold
                   text-white shadow-lg
                   hover:brightness-110 hover:shadow-xl
                   active:scale-[0.97]
                   transition-all duration-150 cursor-pointer"
            style="background: linear-gradient(135deg, {profilColor}, {profilColor}cc);
                   box-shadow: 0 4px 20px {profilColor}30;"
            onclick={() => playerService.handleTogglePlay()}
          >
            {#if isPlaying}
              <svg class="w-3.5 h-3.5" fill="currentColor" viewBox="0 0 24 24">
                <rect x="6" y="4" width="4" height="16" rx="1"/><rect x="14" y="4" width="4" height="16" rx="1"/>
              </svg>
              {$t('home.pause')}
            {:else}
              <svg class="w-3.5 h-3.5" fill="currentColor" viewBox="0 0 24 24"><path d="M8 5v14l11-7z"/></svg>
              {$t('home.resume')}
            {/if}
          </button>
        {/if}

        <button
          onclick={() => handleClickOpenFile()}
          class="flex items-center gap-1.5 px-3 py-1.5 rounded-full text-[11px] font-medium cursor-pointer
                 text-neutral-400/80 dark:text-neutral-500
                 hover:text-neutral-300 hover:bg-white/8
                 active:scale-[0.97] transition-all duration-150"
        >
          <Icon icon="lucide:file-audio" width={12} />
          {$t('home.open_file')}
        </button>

        <button
          onclick={() => handleClickOpenDirectory()}
          class="flex items-center gap-1.5 px-3 py-1.5 rounded-full text-[11px] font-medium cursor-pointer
                 text-neutral-400/80 dark:text-neutral-500
                 hover:text-neutral-300 hover:bg-white/8
                 active:scale-[0.97] transition-all duration-150"
        >
          <Icon icon="lucide:folder-open" width={12} />
          {$t('home.open_folder')}
        </button>

        {#if selectedLibrary?.id}
          <div class="w-px h-4 bg-white/10 mx-1"></div>

          <button
            onclick={() => selectedLibrary && handleAddFiles(selectedLibrary.id as number, true)}
            class="flex items-center gap-1.5 px-3 py-1.5 rounded-full text-[11px] font-medium cursor-pointer
                   text-green-400/70
                   hover:text-green-400 hover:bg-green-500/10
                   active:scale-[0.97] transition-all duration-150"
          >
            <Icon icon="lucide:upload" width={12} />
            {$t('home.import_files')}
          </button>

          <button
            onclick={() => selectedLibrary && handleAddDirectory(selectedLibrary.id as number, true)}
            class="flex items-center gap-1.5 px-3 py-1.5 rounded-full text-[11px] font-medium cursor-pointer
                   text-green-400/70
                   hover:text-green-400 hover:bg-green-500/10
                   active:scale-[0.97] transition-all duration-150"
          >
            <Icon icon="mynaui:folder-plus" width={12} />
            {$t('home.import_folder')}
          </button>
        {/if}
      </div>
    </div>
  </div>

  <!-- STATS RAPIDES -->
  <div class="grid {colonnesStats} gap-3 mb-8">
    <button
      class="group flex items-center gap-3 p-4 rounded-xl cursor-pointer
             bg-neutral-50 dark:bg-neutral-900/50 border border-neutral-200/60 dark:border-neutral-800/60
             hover:border-neutral-300 dark:hover:border-neutral-700
             transition-all duration-200"
      onclick={() => { if ($libraryStore.librarySelected) goto(`/library/${$libraryStore.librarySelected.id}/tracks`); }}
    >
      <div class="w-10 h-10 rounded-lg flex items-center justify-center
                  bg-emerald-500/10 text-emerald-500">
        <Icon icon="lucide:library" width={18} />
      </div>
      <div class="text-left">
        <p class="text-lg font-bold text-neutral-900 dark:text-neutral-100">
          {$libraryStore.libraries.length}
        </p>
        <p class="text-[11px] text-neutral-500 dark:text-neutral-400">
          Bibliothèque{$libraryStore.libraries.length !== 1 ? 's' : ''}
        </p>
      </div>
    </button>

    {#if montrerLikes}
    <button
      class="group flex items-center gap-3 p-4 rounded-xl cursor-pointer
             bg-neutral-50 dark:bg-neutral-900/50 border border-neutral-200/60 dark:border-neutral-800/60
             hover:border-neutral-300 dark:hover:border-neutral-700
             transition-all duration-200"
      onclick={() => goto('/playlist/liked')}
    >
      <div class="w-10 h-10 rounded-lg flex items-center justify-center
                  bg-rose-500/10 text-rose-500">
        <Icon icon="mynaui:heart-solid" width={18} />
      </div>
      <div class="text-left">
        <p class="text-lg font-bold text-neutral-900 dark:text-neutral-100">
          {$likedCount}
        </p>
        <p class="text-[11px] text-neutral-500 dark:text-neutral-400">
          Titre{$likedCount !== 1 ? 's' : ''} liké{$likedCount !== 1 ? 's' : ''}
        </p>
      </div>
    </button>
    {/if}

    {#if montrerRecents}
    <button
      class="group flex items-center gap-3 p-4 rounded-xl cursor-pointer
             bg-neutral-50 dark:bg-neutral-900/50 border border-neutral-200/60 dark:border-neutral-800/60
             hover:border-neutral-300 dark:hover:border-neutral-700
             transition-all duration-200"
      onclick={() => goto('/playlist/recent')}
    >
      <div class="w-10 h-10 rounded-lg flex items-center justify-center
                  bg-sky-500/10 text-sky-500">
        <Icon icon="lucide:clock" width={18} />
      </div>
      <div class="text-left">
        <p class="text-lg font-bold text-neutral-900 dark:text-neutral-100">
          {$recentCount}
        </p>
        <p class="text-[11px] text-neutral-500 dark:text-neutral-400">
          Joué{$recentCount !== 1 ? 's' : ''} récemment
        </p>
      </div>
    </button>
    {/if}

    <div class="flex items-center gap-3 p-4 rounded-xl
                bg-neutral-50 dark:bg-neutral-900/50 border border-neutral-200/60 dark:border-neutral-800/60">
      <div class="w-10 h-10 rounded-lg flex items-center justify-center
                  bg-violet-500/10 text-violet-500">
        <Icon icon="lucide:list-music" width={18} />
      </div>
      <div class="text-left">
        <p class="text-lg font-bold text-neutral-900 dark:text-neutral-100">
          {$playlistStore.playlists.length}
        </p>
        <p class="text-[11px] text-neutral-500 dark:text-neutral-400">
          Playlist{$playlistStore.playlists.length !== 1 ? 's' : ''}
        </p>
      </div>
    </div>
  </div>

  <!-- RÉCEMMENT JOUÉS -->
  <!-- Même raccourci que le compteur du haut : un seul réglage pour les deux. -->
  {#if montrerRecents}
  <div class="flex items-center justify-between mb-4">
    <div>
      <h2 class="text-xl font-bold tracking-tight text-neutral-900 dark:text-neutral-100">
        {$t('home.recently_played')}
      </h2>
      <div class="mt-1.5 h-0.5 w-10 rounded-full" style="background: {profilColor};"></div>
    </div>

    {#if $recent.length > 0}
      <button
        onclick={() => recent.clearRecent()}
        class="cursor-pointer text-xs text-neutral-400 hover:text-neutral-600 dark:hover:text-neutral-300
               flex items-center gap-1.5 px-3 py-1.5 rounded-lg
               hover:bg-neutral-100 dark:hover:bg-neutral-800
               transition-all duration-150"
      >
        <Icon icon="mynaui:trash" width="14" height="14" />
        {$t('home.clear_history')}
      </button>
    {/if}
  </div>

  <RecentTrackItem />
  {/if}
</div>
