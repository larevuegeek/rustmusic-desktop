<script lang="ts">
  import Icon from "@iconify/svelte";
  import { goto } from "$app/navigation";
  import { t } from "$lib/i18n";
  import { portal } from "$lib/helper/portal";
  import { settingsStore } from "$lib/stores/settings/settings.store";

  /** Les options de la zone des playlists, sous le bouton « … ». */
  let { x, y, onclose }: { x: number; y: number; onclose: () => void } = $props();

  /**
   * Les raccourcis bâtis dans l'application.
   *
   * Pastilles et teintes reprises de la barre latérale : le menu désigne ces
   * deux lignes-là, les redessiner autrement obligerait à faire le lien.
   */
  const raccourcis = $derived([
    {
      cle: 'show_liked_in_playlists' as const,
      titre: $t('playlist_page.liked_title'),
      icone: 'mynaui:heart-solid',
      pastille: 'bg-linear-to-br from-rose-500/15 to-pink-500/25 border-rose-500/20',
      teinte: 'text-rose-400',
      visible: $settingsStore.show_liked_in_playlists !== 'false',
    },
    {
      cle: 'show_recent_in_playlists' as const,
      titre: $t('playlist_page.recent_title'),
      icone: 'mynaui:clock-8',
      pastille: 'bg-linear-to-br from-sky-500/15 to-cyan-500/25 border-sky-500/20',
      teinte: 'text-sky-400',
      visible: $settingsStore.show_recent_in_playlists !== 'false',
    },
  ]);

  // Aligné à droite sur le bouton : le menu est plus large que lui, et la barre
  // latérale est contre le bord gauche.
  let menuStyle = $derived.by(() => {
    const largeur = 264;
    const hauteur = 196;
    const posX = Math.max(8, Math.min(x - largeur + 16, window.innerWidth - largeur - 8));
    const posY = y + hauteur > window.innerHeight ? Math.max(8, y - hauteur) : y;
    return `left: ${posX}px; top: ${posY}px;`;
  });

  const ligne = "w-full flex items-center gap-2.5 px-2 py-2 rounded-xl cursor-pointer text-left transition-colors hover:bg-neutral-100 dark:hover:bg-white/5";
</script>

<!-- Porté vers `body` : la barre latérale porte `backdrop-blur`, qui crée un
     contexte d'empilement. Voir `PlaylistContextMenu`. -->
<div use:portal>
<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- Le clic droit est avalé ici aussi : sans ça, un second clic droit pendant
     que le menu est ouvert laissait passer celui du navigateur. -->
<button
  type="button"
  class="fixed inset-0 z-9998 cursor-default"
  onclick={onclose}
  oncontextmenu={(e) => { e.preventDefault(); onclose(); }}
  aria-label="Fermer le menu"
></button>

<div
  role="menu"
  tabindex="-1"
  oncontextmenu={(e) => e.preventDefault()}
  class="fixed z-[9999] w-66 p-1.5
         bg-white/95 dark:bg-neutral-950/95 backdrop-blur-xl
         border border-neutral-200/80 dark:border-white/10
         rounded-2xl overflow-hidden
         shadow-xl shadow-black/10 dark:shadow-2xl dark:shadow-black/50"
  style={menuStyle}
>
  <p class="px-2.5 pt-1.5 pb-2 text-[10px] font-semibold uppercase tracking-widest
            text-neutral-400 dark:text-neutral-500">
    {$t('playlist.builtin')}
  </p>

  {#each raccourcis as r (r.cle)}
    <button class={ligne} onclick={() => settingsStore.toggle(r.cle)}>
      <span class="w-7 h-7 rounded-lg shrink-0 flex items-center justify-center border {r.pastille}
                   transition-opacity {r.visible ? '' : 'opacity-40'}">
        <Icon icon={r.icone} width="14" class={r.teinte} />
      </span>

      <span class="flex-1 min-w-0 text-[13px] truncate
                   {r.visible
                     ? 'text-neutral-800 dark:text-neutral-100'
                     : 'text-neutral-400 dark:text-neutral-500'}">
        {r.titre}
      </span>

      <!-- Interrupteur dessiné, pas un composant : la ligne entière est déjà
           le bouton, en imbriquer un second serait invalide. -->
      <span class="w-8 h-4.5 rounded-full shrink-0 relative transition-colors
                   {r.visible ? 'bg-emerald-500' : 'bg-neutral-300 dark:bg-neutral-700'}">
        <span class="absolute top-0.5 left-0.5 w-3.5 h-3.5 rounded-full bg-white shadow-sm
                     transition-transform {r.visible ? 'translate-x-3.5' : ''}"></span>
      </span>
    </button>
  {/each}

  <div class="h-px mx-2 my-1.5 bg-neutral-200/80 dark:bg-white/8"></div>

  <button class={ligne} onclick={() => { onclose(); goto('/settings/appearance'); }}>
    <span class="w-7 h-7 rounded-lg shrink-0 flex items-center justify-center
                 bg-neutral-100 dark:bg-white/5 border border-neutral-200/80 dark:border-white/8">
      <Icon icon="lucide:sliders-horizontal" width="13" class="text-neutral-500 dark:text-neutral-400" />
    </span>
    <span class="flex-1 text-[13px] text-neutral-700 dark:text-neutral-200">
      {$t('playlist.display_settings')}
    </span>
    <Icon icon="lucide:chevron-right" width="13" class="text-neutral-300 dark:text-neutral-600 shrink-0" />
  </button>
</div>
</div>
