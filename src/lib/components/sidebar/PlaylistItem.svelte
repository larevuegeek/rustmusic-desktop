<script lang="ts">
  import Icon from "@iconify/svelte";
  import type { Playlist } from "$lib/types/db/playlist/Playlist";
  import { goto } from "$app/navigation";
  import { popinStore } from "$lib/stores/ui/popin.store";
  import EditPlaylistPopin from "$lib/components/playlist/popin/EditPlaylistPopin.svelte";
  import SmartPlaylistPopin from "$lib/components/playlist/smart/SmartPlaylistPopin.svelte";
  import PlaylistContextMenu from "$lib/components/ui/contextmenu/PlaylistContextMenu.svelte";

  let { playlist }: { playlist: Playlist } = $props();

  let menu = $state<{ x: number; y: number } | null>(null);

  /**
   * Le crayon n'ouvre pas la même chose selon la playlist.
   *
   * Sur une playlist intelligente, ce qu'on veut changer ce sont ses règles :
   * la fenêtre de modification ordinaire n'en montre aucune, et enregistrer
   * depuis elle aurait réécrit son identité sans toucher à ce qui la remplit.
   */
  function openEdit(e: MouseEvent) {
    e.stopPropagation();

    if (playlist.is_smart) {
      popinStore.open(
        "Playlist intelligente",
        SmartPlaylistPopin,
        { playlistId: playlist.id },
        { size: "xl", icon: "lucide:sparkles", flush: true },
      );
      return;
    }

    popinStore.open("Modifier la playlist", EditPlaylistPopin, { playlist });
  }
</script>

<div
  class="group relative"
  oncontextmenu={(e) => { e.preventDefault(); menu = { x: e.clientX, y: e.clientY }; }}
  role="presentation"
>
  <button
    class="flex w-full items-center gap-3 px-2 py-1.5 rounded-lg text-left cursor-pointer
           transition-all duration-150
           hover:bg-neutral-100/80 dark:hover:bg-white/4"
    onclick={() => goto(`/playlist/${playlist.id}`)}
  >
    <div class="relative w-9 h-9 shrink-0">
      <div class="w-full h-full rounded-lg flex items-center justify-center"
           style="background: {playlist.color}18; border: 1px solid {playlist.color}25;">
        <Icon icon={playlist.icon} width="16" height="16"
              style="color: {playlist.color};" />
      </div>

      <!-- Repère des playlists intelligentes, dans le coin de la vignette.
           Deux corrections par rapport au premier essai, qui restait illisible.
           La couleur : d'abord celle de la playlist, donc celle de la vignette
           qu'elle chevauche — le repère se fondait dans ce dont il devait se
           distinguer. Puis un vert, qui entrait en concurrence avec l'accent de
           l'application.
           Aucune teinte n'est sûre de trancher : le catalogue des couleurs de
           playlist couvre presque toute la roue, du violet à l'orange. Le
           repère est donc **neutre**, et c'est justement ce qui le rend
           toujours lisible — il ne cherche pas à rivaliser avec la couleur
           qu'on a choisie, il s'en détache par le contraste.
           Le glyphe : une baguette, dont la diagonale reste identifiable quand
           tout le reste se referme.
           L'anneau reprend le fond de la barre latérale pour détacher la
           pastille du carré qu'elle mord. -->
      {#if playlist.is_smart}
        <span
          class="absolute -bottom-1 -right-1 w-4 h-4 rounded-full
                 flex items-center justify-center
                 bg-neutral-700 dark:bg-neutral-300
                 ring-2 ring-neutral-50 dark:ring-zinc-950"
          title="Playlist intelligente — se remplit toute seule selon des règles"
        >
          <Icon
            icon="lucide:wand-2"
            width="10"
            height="10"
            class="text-white dark:text-neutral-900"
          />
        </span>
      {/if}
    </div>
    <div class="min-w-0 flex-1">
      <div class="font-medium text-[13px] truncate text-neutral-800 dark:text-neutral-200">
        {playlist.name}
      </div>
      <div class="text-[11px] truncate text-neutral-400 dark:text-neutral-500">
        {playlist.track_count} titre{playlist.track_count !== 1 ? 's' : ''}
      </div>
    </div>
  </button>

  <!-- Bouton edit au hover -->
  <button
    type="button"
    class="absolute right-2 top-1/2 -translate-y-1/2 opacity-0 group-hover:opacity-100
           w-7 h-7 rounded-md flex items-center justify-center
           text-neutral-400 hover:text-neutral-600 dark:hover:text-neutral-200
           hover:bg-neutral-300/50 dark:hover:bg-neutral-700/50
           transition-all duration-150 cursor-pointer"
    aria-label="Modifier la playlist"
    onclick={openEdit}
  >
    <Icon icon="lucide:pen-line" class="w-3.5 h-3.5" />
  </button>
</div>

{#if menu}
  <PlaylistContextMenu {playlist} x={menu.x} y={menu.y} onclose={() => menu = null} />
{/if}
