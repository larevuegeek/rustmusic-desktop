<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import Icon from "@iconify/svelte";
  import { settingsStore } from "$lib/stores/settings/settings.store";
  import { toggleMiniPlayer } from "$lib/stores/ui/miniPlayer.store";
  import { t } from "$lib/i18n";
  import { detectOS } from "$lib/helper/tools/osDetection";
  import LogoRustMusic from "$lib/components/ui/logo/LogoRustMusic.svelte";
  import SidebarBouton from "$lib/components/sidebar/SidebarBouton.svelte";
  import type { Snippet } from "svelte";

  /**
   * Barre unique : marque, puis ce que fournit la page (navigation, recherche,
   * actions), puis les contrôles de fenêtre. `avecSidebar` : le bouton pour
   * replier la barre latérale suit le logo.
   */
  let { avecSidebar = false, children }: { avecSidebar?: boolean; children?: Snippet } = $props();

  const appWindow = getCurrentWindow();

  let isMaximized = $state(false);
  let osDetected = detectOS();

  let minimizeToTray = $derived($settingsStore.minimize_to_tray === 'true');
  let replie = $derived($settingsStore.sidebar_collapsed === 'true');

  // Style résolu : 'auto' → en fonction de l'OS, sinon valeur explicite
  let effectiveStyle = $derived.by<'macos' | 'windows' | 'linux'>(() => {
    const setting = $settingsStore.window_controls_style;
    if (setting === 'macos' || setting === 'windows' || setting === 'linux') return setting;
    // 'auto' : Mac → traffic lights, Linux → boutons ronds GNOME, sinon → Windows
    if (osDetected === 'macos') return 'macos';
    if (osDetected === 'linux') return 'linux';
    return 'windows';
  });

  // Position résolue : 'right' par défaut, 'left' si explicitement demandé
  let effectivePosition = $derived<'right' | 'left'>(
    $settingsStore.window_controls_position === 'left' ? 'left' : 'right'
  );

  // Suit l'état réel : bouton, Win+↑, double clic ou fenêtre tirée vers le haut.
  $effect(() => {
    const lire = () => { appWindow.isMaximized().then((v) => { isMaximized = v; }).catch(() => {}); };
    lire();
    let arreter: (() => void) | null = null;
    let fini = false;
    appWindow.onResized(lire).then((u) => { if (fini) u(); else arreter = u; }).catch(() => {});
    return () => { fini = true; arreter?.(); };
  });

  function minimize() { appWindow.minimize(); }
  function toggleMaximize() { appWindow.toggleMaximize(); }

  // Seulement sur le vide : un double clic dans la recherche n'agrandit pas la fenêtre.
  function doubleClic(e: MouseEvent) {
    if ((e.target as HTMLElement).hasAttribute('data-tauri-drag-region')) toggleMaximize();
  }
  function close() {
    if (minimizeToTray) appWindow.hide();
    else appWindow.close();
  }
</script>

<!-- Bouton mini-lecteur : action de fenêtre (always-on-top compact), donc
     rangé avec les contrôles de fenêtre. Toujours côté "intérieur" du groupe
     (order-first quand les contrôles sont à droite, order-last à gauche)
     pour laisser les 3 contrôles standards contre le coin de la fenêtre. -->
{#snippet miniPlayerButton(style: 'macos' | 'windows' | 'linux')}
  <button
    class="cursor-pointer transition-colors
           {style === 'windows'
             ? 'flex items-center justify-center w-11 h-full text-neutral-500 dark:text-neutral-400 hover:bg-neutral-200/60 dark:hover:bg-white/8 hover:text-neutral-700 dark:hover:text-neutral-200'
             : style === 'linux'
               ? 'flex items-center justify-center w-6 h-6 rounded-full bg-neutral-200 dark:bg-white/10 text-neutral-700 dark:text-neutral-300 hover:bg-neutral-300 dark:hover:bg-white/20'
               : 'flex items-center justify-center w-5 h-5 text-neutral-400 dark:text-neutral-500 hover:text-neutral-700 dark:hover:text-neutral-200'}"
    class:order-first={effectivePosition === 'right'}
    class:order-last={effectivePosition === 'left'}
    onclick={() => toggleMiniPlayer()}
    aria-label={$t('mini.enter')}
    title={$t('mini.enter')}
  >
    <Icon icon="lucide:picture-in-picture-2" width={style === 'linux' ? 11 : 13} height={style === 'linux' ? 11 : 13} />
  </button>
{/snippet}

<div
  data-tauri-drag-region
  role="toolbar"
  tabindex="-1"
  class="relative z-30 flex items-center h-15 select-none shrink-0
         bg-(--c-barre) dark:bg-zinc-950/50 backdrop-blur-md
         border-b border-(--c-bord) dark:border-white/6"
  ondblclick={doubleClic}
>
  {#snippet separateur()}
    <span class="w-px h-6 shrink-0 bg-(--c-bord) dark:bg-white/10"></span>
  {/snippet}

  <!-- Marque. `sidebar` pour les couleurs du bouton, pas pour le fond. Le bouton
       de repli suit le logo, presque collé ; caché sur mobile, où le menu s'ouvre autrement. -->
  <div data-tauri-drag-region class="sidebar flex items-center gap-1 h-full pl-5 pr-3 shrink-0">
    <span class="flex pointer-events-none"><LogoRustMusic width={140} /></span>
    {#if avecSidebar}
      <span class="hidden md:flex">
        <SidebarBouton
          icon={replie ? 'material-symbols:left-panel-open-outline-rounded' : 'material-symbols:left-panel-close-outline-rounded'}
          label={replie ? $t('sidebar.expand') : $t('sidebar.collapse')}
          taille={18}
          onclick={() => settingsStore.set('sidebar_collapsed', replie ? 'false' : 'true')}
        />
      </span>
    {/if}
  </div>

  {#if children}
    {@render separateur()}
    <div data-tauri-drag-region class="flex-1 min-w-0 h-full flex items-center gap-2 md:gap-3 px-3 md:px-4">
      {@render children()}
    </div>
  {:else}
    <div data-tauri-drag-region class="flex-1 h-full"></div>
  {/if}

  {#if effectivePosition === 'right'}{@render separateur()}{/if}

  <!-- ─── Contrôles : style macOS (traffic lights) ─── -->
  {#if effectiveStyle === 'macos'}
    <div
      class="flex items-center gap-2 px-4 h-full"
      class:ml-auto={effectivePosition === 'right'}
      class:order-first={effectivePosition === 'left'}
    >
      {@render miniPlayerButton('macos')}
      <button
        class="w-3 h-3 rounded-full bg-[#febc2e] opacity-70 hover:opacity-100
               transition-opacity cursor-pointer"
        onclick={minimize}
        aria-label={$t("window.minimize")}
      ></button>
      <button
        class="w-3 h-3 rounded-full bg-[#28c840] opacity-70 hover:opacity-100
               transition-opacity cursor-pointer"
        onclick={toggleMaximize}
        aria-label={$t("window.maximize")}
      ></button>
      <button
        class="w-3 h-3 rounded-full bg-[#ff5f57] opacity-70 hover:opacity-100
               transition-opacity cursor-pointer"
        onclick={close}
        aria-label={$t("common.close")}
      ></button>
    </div>

  <!-- ─── Contrôles : style Linux (boutons ronds Adwaita / GNOME) ─── -->
  {:else if effectiveStyle === 'linux'}
    <div
      class="flex items-center gap-1.5 px-3 h-full"
      class:ml-auto={effectivePosition === 'right'}
      class:order-first={effectivePosition === 'left'}
    >
      {@render miniPlayerButton('linux')}
      <!-- Minimize -->
      <button
        class="flex items-center justify-center w-6 h-6 rounded-full cursor-pointer
               bg-neutral-200 dark:bg-white/10
               text-neutral-700 dark:text-neutral-300
               hover:bg-neutral-300 dark:hover:bg-white/20
               transition-colors"
        onclick={minimize}
        aria-label={$t("window.minimize")}
      >
        <svg viewBox="0 0 12 12" width="9" height="9" fill="currentColor">
          <rect x="2" y="5.5" width="8" height="1" />
        </svg>
      </button>

      <!-- Maximize / Restore -->
      <button
        class="flex items-center justify-center w-6 h-6 rounded-full cursor-pointer
               bg-neutral-200 dark:bg-white/10
               text-neutral-700 dark:text-neutral-300
               hover:bg-neutral-300 dark:hover:bg-white/20
               transition-colors"
        onclick={toggleMaximize}
        aria-label={$t("window.maximize")}
      >
        {#if isMaximized}
          <svg viewBox="0 0 12 12" width="9" height="9" fill="none" stroke="currentColor" stroke-width="1.2">
            <rect x="2.5" y="3.5" width="6" height="6" />
            <path d="M4 3.5V2.5h6V8.5H8.5" />
          </svg>
        {:else}
          <svg viewBox="0 0 12 12" width="9" height="9" fill="none" stroke="currentColor" stroke-width="1.2">
            <rect x="2.5" y="2.5" width="7" height="7" />
          </svg>
        {/if}
      </button>

      <!-- Close (rond rouge GNOME) -->
      <button
        class="flex items-center justify-center w-6 h-6 rounded-full cursor-pointer
               bg-neutral-200 dark:bg-white/10
               text-[#5e625d] dark:text-neutral-300
               hover:bg-red-500 hover:text-white
               transition-colors"
        onclick={close}
        aria-label={$t("common.close")}
      >
        <svg viewBox="0 0 12 12" width="9" height="9" fill="none" stroke="currentColor" stroke-width="1.2">
          <path d="M2.5 2.5l7 7M9.5 2.5l-7 7" />
        </svg>
      </button>
    </div>

  <!-- ─── Contrôles : style Windows (boutons rectangulaires) ─── -->
  {:else}
    <div
      class="flex items-center h-full"
      class:ml-auto={effectivePosition === 'right'}
      class:order-first={effectivePosition === 'left'}
    >
      {@render miniPlayerButton('windows')}
      <!-- Minimize -->
      <button
        class="flex items-center justify-center w-11 h-full cursor-pointer
               text-[#5e625d] dark:text-neutral-300
               hover:bg-[#e5e1d8] dark:hover:bg-white/8
               transition-colors"
        onclick={minimize}
        aria-label={$t("window.minimize")}
      >
        <svg viewBox="0 0 12 12" width="12" height="12" fill="currentColor">
          <rect x="2" y="5.5" width="8" height="1" />
        </svg>
      </button>

      <!-- Maximize / Restore -->
      <button
        class="flex items-center justify-center w-11 h-full cursor-pointer
               text-[#5e625d] dark:text-neutral-300
               hover:bg-[#e5e1d8] dark:hover:bg-white/8
               transition-colors"
        onclick={toggleMaximize}
        aria-label={$t("window.maximize")}
      >
        {#if isMaximized}
          <!-- Restore icon (2 squares) -->
          <svg viewBox="0 0 12 12" width="11" height="11" fill="none" stroke="currentColor" stroke-width="1">
            <rect x="2.5" y="3.5" width="6" height="6" />
            <path d="M4 3.5V2.5h6V8.5H8.5" />
          </svg>
        {:else}
          <!-- Maximize icon (single square) -->
          <svg viewBox="0 0 12 12" width="11" height="11" fill="none" stroke="currentColor" stroke-width="1">
            <rect x="2.5" y="2.5" width="7" height="7" />
          </svg>
        {/if}
      </button>

      <!-- Close (hover rouge spécifique Windows) -->
      <button
        class="flex items-center justify-center w-11 h-full cursor-pointer
               text-[#5e625d] dark:text-neutral-300
               hover:bg-red-500 hover:text-white
               transition-colors"
        onclick={close}
        aria-label={$t("common.close")}
      >
        <svg viewBox="0 0 12 12" width="11" height="11" fill="none" stroke="currentColor" stroke-width="1.1">
          <path d="M2.5 2.5l7 7M9.5 2.5l-7 7" />
        </svg>
      </button>
    </div>
  {/if}

  {#if effectivePosition === 'left'}<span class="order-first flex">{@render separateur()}</span>{/if}
</div>
