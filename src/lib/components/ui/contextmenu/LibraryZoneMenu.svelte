<script lang="ts">
  import Icon from "@iconify/svelte";
  import { goto } from "$app/navigation";
  import { t } from "$lib/i18n";
  import { portal } from "$lib/helper/portal";
  import { settingsStore } from "$lib/stores/settings/settings.store";
  import {
    ONGLETS_BIBLIOTHEQUE,
    ONGLETS_PAR_DEFAUT,
    lireOnglets,
    lirePlacement,
    type LibraryTabKey,
    type PlacementOnglets,
  } from "$lib/config/libraryTabs";

  /** Les options de la zone Bibliothèque, sous le bouton « … ». */
  let { x, y, onclose }: { x: number; y: number; onclose: () => void } = $props();

  const placement = $derived(lirePlacement($settingsStore.library_tabs_position));
  const retenus = $derived(lireOnglets($settingsStore.library_tabs));

  const placements: { valeur: PlacementOnglets; labelKey: string; gauche: boolean; haut: boolean }[] = [
    { valeur: 'sidebar', labelKey: 'settings.library_tabs_sidebar', gauche: true,  haut: false },
    { valeur: 'top',     labelKey: 'settings.library_tabs_top',     gauche: false, haut: true  },
    { valeur: 'both',    labelKey: 'settings.library_tabs_both',    gauche: true,  haut: true  },
  ];

  /** Conserve l'ordre d'origine, et refuse de tout décocher. */
  function basculer(cle: LibraryTabKey) {
    const suivant = retenus.includes(cle)
      ? retenus.filter((c) => c !== cle)
      : ONGLETS_PAR_DEFAUT.filter((c) => c === cle || retenus.includes(c));
    if (suivant.length === 0) return;
    settingsStore.set('library_tabs', JSON.stringify(suivant));
  }

  let menuStyle = $derived.by(() => {
    const largeur = 264;
    const hauteur = 372;
    const posX = Math.max(8, Math.min(x - largeur + 16, window.innerWidth - largeur - 8));
    const posY = y + hauteur > window.innerHeight ? Math.max(8, window.innerHeight - hauteur - 8) : y;
    return `left: ${posX}px; top: ${posY}px;`;
  });

  const titre = "px-2.5 pt-1.5 pb-2 text-[10px] font-semibold uppercase tracking-widest text-neutral-400 dark:text-neutral-500";
  const ligne = "w-full flex items-center gap-2.5 px-2 py-1.5 rounded-xl cursor-pointer text-left transition-colors hover:bg-neutral-100 dark:hover:bg-white/5";
</script>

<!-- Porté vers `body` : la barre latérale porte `backdrop-blur`, qui crée un
     contexte d'empilement. Voir `PlaylistContextMenu`. -->
<div use:portal>
<!-- svelte-ignore a11y_no_static_element_interactions -->
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
  <p class={titre}>{$t('settings.library_tabs')}</p>

  <!-- Trois maquettes plutôt que trois lignes de texte : « à gauche », « en
       haut » et « les deux » se lisent d'un coup d'œil en image. -->
  <div class="flex gap-1 px-1 pb-1">
    {#each placements as p (p.valeur)}
      <button
        class="flex-1 flex flex-col items-center gap-1 py-1.5 rounded-lg cursor-pointer
               border transition-colors
               {placement === p.valeur
                 ? 'bg-emerald-500/10 border-emerald-500/30'
                 : 'border-transparent hover:bg-neutral-100 dark:hover:bg-white/5'}"
        onclick={() => settingsStore.set('library_tabs_position', p.valeur)}
        title={$t(p.labelKey)}
      >
        <span class="w-9 h-6 rounded flex gap-0.5 p-0.5
                     bg-neutral-100 dark:bg-white/5">
          <span class="w-1/4 h-full rounded-[2px]
                       {p.gauche ? 'bg-emerald-500/70' : 'bg-neutral-300 dark:bg-white/10'}"></span>
          <span class="flex-1 flex flex-col gap-0.5">
            <span class="h-1 rounded-[2px]
                         {p.haut ? 'bg-emerald-500/70' : 'bg-neutral-300 dark:bg-white/10'}"></span>
            <span class="flex-1 rounded-[2px] bg-neutral-200 dark:bg-white/4"></span>
          </span>
        </span>
        <span class="text-[10px] leading-none
                     {placement === p.valeur
                       ? 'text-emerald-600 dark:text-emerald-400'
                       : 'text-neutral-500 dark:text-neutral-400'}">
          {$t(p.labelKey)}
        </span>
      </button>
    {/each}
  </div>

  <div class="h-px mx-2 my-1.5 bg-neutral-200/80 dark:bg-white/8"></div>

  <p class={titre}>{$t('settings.library_tabs_shown')}</p>

  {#each ONGLETS_BIBLIOTHEQUE as onglet (onglet.key)}
    {@const affiche = retenus.includes(onglet.key)}
    {@const dernier = affiche && retenus.length === 1}
    <button
      class="{ligne} {dernier ? 'cursor-not-allowed' : ''}"
      disabled={dernier}
      onclick={() => basculer(onglet.key)}
      title={dernier ? $t('settings.library_tabs_last') : ''}
    >
      <span class="w-7 h-7 rounded-lg shrink-0 flex items-center justify-center border
                   bg-neutral-100 dark:bg-white/5 border-neutral-200/80 dark:border-white/8
                   transition-opacity {affiche ? '' : 'opacity-40'}">
        <Icon icon={onglet.icon} width="13"
              class={affiche ? 'text-neutral-600 dark:text-neutral-300' : 'text-neutral-400'} />
      </span>

      <span class="flex-1 min-w-0 text-[13px] truncate
                   {affiche
                     ? 'text-neutral-800 dark:text-neutral-100'
                     : 'text-neutral-400 dark:text-neutral-500'}">
        {$t(onglet.labelKey)}
      </span>

      <span class="w-8 h-4.5 rounded-full shrink-0 relative transition-colors
                   {affiche ? 'bg-emerald-500' : 'bg-neutral-300 dark:bg-neutral-700'}
                   {dernier ? 'opacity-50' : ''}">
        <span class="absolute top-0.5 left-0.5 w-3.5 h-3.5 rounded-full bg-white shadow-sm
                     transition-transform {affiche ? 'translate-x-3.5' : ''}"></span>
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
