<script lang="ts">
  import { t } from "#lib/i18n";
import "../app.css";
import "#lib/icons/preload";
import Sidebar from "#lib/components/sidebar/Sidebar.svelte";
import SwitchTheme from "#lib/components/ui/input/SwitchTheme.svelte";
import Player from "#lib/components/player/Player.svelte";
import QueuePanel from "#lib/components/queue/QueuePanel.svelte";
import Icon from "@iconify/svelte";
import SearchAutocomplete from "#lib/components/search/SearchAutocomplete.svelte";
import Toast from "#lib/components/ui/toast/Toast.svelte";
import UpdateBanner from "#lib/components/updater/UpdateBanner.svelte";
import { onMount } from "svelte";
import { get } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import { profilSelector } from "#lib/stores/profil/profil.store";
import { player } from "#lib/stores/player/player.store";
import ProfilSelectorInput from "#lib/components/header/ProfilSelectorInput.svelte"
import ProfilSelectorPopin from "#lib/components/header/ProfilSelectorPopin.svelte";
import Titlebar from "#lib/components/ui/titlebar/Titlebar.svelte";
import Popin from "#lib/components/ui/popin/Popin.svelte";
import { libraryStore } from "#lib/stores/library/library.store";
import { queueState } from "#lib/stores/queue/queueState.store";
import { playerService } from "#lib/services/player/player.service";
import { importProgressStore } from "#lib/stores/library/importProgress.store";
import { settingsStore } from "#lib/stores/settings/settings.store";
import { sidebarStore } from "#lib/stores/ui/sidebar.store";
import LibraryTabs from "#lib/components/library/common/LibraryTabs.svelte";
import LibraryViewControls from "#lib/components/library/common/LibraryViewControls.svelte";
import { lirePlacement } from "#lib/config/libraryTabs";
import { taskProgressStore } from "#lib/stores/ui/taskProgress.store";
import { artistImageReadyStore } from "#lib/stores/library/artistImageReady.store";
import { refreshDlnaStatus } from "#lib/stores/dlna/dlna.store";
import { initPlaybackPipelineListener } from "#lib/stores/player/playbackPipeline.store";
import { initBatchListeners } from "#lib/stores/ui/batch.store";
import BatchPanel from "#lib/components/ui/batch/BatchPanel.svelte";
import { trackNotificationService } from "#lib/services/notification/trackNotification.service";
import { mediaControlsService } from "#lib/services/mediaControls/mediaControls.service";
import { onDestroy } from "svelte";
import { page } from "$app/state";
import SelectionBar from "#lib/components/ui/selection/SelectionBar.svelte";
import MiniPlayer from "#lib/components/player/MiniPlayer.svelte";
import { miniPlayerActive, assurerTailleNormale } from "#lib/stores/ui/miniPlayer.store";
import SleepTimerButton from "#lib/components/player/SleepTimerButton.svelte";
import { fade } from "svelte/transition";
import { ouvrirLiensExternes } from "#lib/helper/tools/liensExternes";
import { lecture } from "#lib/stores/player/lecture.store";
import { listen } from "@tauri-apps/api/event";
import { empecherSurlignageMaj } from "#lib/helper/tools/surlignageMaj";
import { volume, chargerVolume, reglerVolume } from "#lib/stores/player/volume.store";

// Affichage du bouton sleep timer dans le header (désactivable dans les réglages).
let showSleepTimer = $derived($settingsStore.show_sleep_timer !== 'false');
import type { Snippet } from "svelte";

let { children }: { children: Snippet } = $props();

// Sur /settings : mode pleine page — on cache la sidebar app + la barre de
// recherche pour laisser Paramètres prendre toute la largeur. Le Player en
// bas reste visible pour continuer à contrôler la lecture.
let isFullPageRoute = $derived(page.url.pathname.startsWith('/settings'));

// « En haut » comme « Les deux » : la rangée suit partout. Seul « À gauche »
// s'en passe, la barre latérale portant alors les sections.
const tabsEnHaut = $derived(
  !isFullPageRoute && lirePlacement($settingsStore.library_tabs_position) !== 'sidebar'
);
const dansLaBibliotheque = $derived(page.url.pathname.startsWith('/library/'));
// Les réglages gèrent eux-mêmes le fondu entre sections : leur navigation reste en place.
// Dans une bibliothèque, son layout reste monté : le fondu des pages se fait dans ce layout.
const cleTransition = $derived(isFullPageRoute ? '/settings' : dansLaBibliotheque ? `/library/${page.params.library_id}` : page.url.pathname);

// L'écran de chargement d'app.html s'efface en fondu une fois le profil prêt.
$effect(() => {
  if (!$profilSelector.initialized) return;
  const ecran = document.getElementById("demarrage");
  if (!ecran) return;
  ecran.classList.add("fini");
  setTimeout(() => ecran.remove(), 250);
});

onMount(ouvrirLiensExternes);
onMount(() => void chargerVolume());

onMount(empecherSurlignageMaj);

// Pas de veille automatique tant que la musique joue (réglage) ; l'écran peut s'éteindre.
$effect(() => {
  const bloquer = $lecture.status === "playing" && $settingsStore.prevent_sleep !== "false";
  invoke("set_veille_bloquee", { bloquer }).catch(() => {});
});

// Mise en veille annoncée (Linux) : pause, pour ne pas laisser la sortie audio ouverte. Au réveil, on reste en pause.
onMount(() => {
  const fin = listen<boolean>("systeme-veille", (e) => {
    if (e.payload && get(player).status === "playing") playerService.pauseFile();
  });
  return () => { fin.then((f) => f()); };
});

onMount(async () => {
  // Fermée en mode mini, la fenêtre rouvrirait toute petite : plancher et taille d'origine.
  void assurerTailleNormale();

  // Sentinelle anti-crash GPU (Linux) : on ne confirme le boot qu'après deux
  // frames réellement peintes. Si le compositing WebKit est cassé (fenêtre
  // blanche), requestAnimationFrame ne fire jamais → la sentinelle reste
  // armée et le prochain lancement basculera en rendu logiciel.
  requestAnimationFrame(() => requestAnimationFrame(() => {
    invoke('notify_ui_ready').catch(() => {});
  }));

  // Les réglages d'abord : ils décident ce que la barre latérale affiche, et
  // l'écran d'attente ne se lève qu'au profil. Chargés après, les raccourcis
  // masqués apparaissaient deux secondes avant de disparaître.
  await settingsStore.init();

  await profilSelector.init();
  await libraryStore.init();
  await queueState.init();
  playerService.init();
  importProgressStore.init();
  taskProgressStore.init();
  artistImageReadyStore.init();

  // Refresh DLNA status (server may have auto-started in Rust setup)
  refreshDlnaStatus();

  // Subscribe to the backend playback-pipeline event so the player status
  // bar can show "source → output" when a conversion happens.
  initPlaybackPipelineListener();
  initBatchListeners();

  // Notif OS sur changement de morceau (cover + titre + artiste).
  // Skip la 1re émission (restore de la queue au démarrage).
  trackNotificationService.init();

  // SMTC / MPRIS / Now Playing — intégration aux contrôles média OS.
  // sync() lit le réglage `system_media_controls` et active/désactive en
  // conséquence. Appel après settingsStore.init() ci-dessus pour avoir la
  // valeur persistée. Erreurs silencieuses (logged) pour ne pas bloquer
  // le démarrage si l'OS refuse (Linux sans D-Bus, etc.).
  mediaControlsService.sync().catch((e) => console.warn('[smtc] sync init failed:', e));

  // WASAPI exclusive — pousser la valeur persistée vers l'atomic global Rust
  // dès le boot pour que le 1er morceau lu utilise déjà la bonne sortie.
  try {
    const wasapiOn = settingsStore.get('wasapi_exclusive') === 'true';
    await invoke('set_wasapi_exclusive_preference', { enabled: wasapiOn });
    const dopOn = settingsStore.get('dsd_dop') === 'true';
    await invoke('set_dop_preference', { enabled: dopOn });
  } catch (e) {
    console.warn('[settings] WASAPI/DoP preference init failed:', e);
  }

  // Le `playback-preparing` event est désormais écouté par taskProgressStore
  // (déjà init plus haut), pas besoin d'un listener dédié.

  // Scan automatique au démarrage si activé
  if (settingsStore.get('scan_on_startup') === 'true') {
    const lib = get(libraryStore).librarySelected;
    if (lib?.id) {
      invoke('rescan_library', { libraryId: lib.id }).catch(e =>
        console.warn('[startup] Scan auto échoué:', e)
      );
    }
  }

  // Auto-update : vérifie en silence 5s après le démarrage (anti cold-start
  // slow). Si une update est dispo, le store updaterState passe en
  // "available" et UpdateBanner s'affiche en bas à droite.
  const { initUpdaterAutoCheck } = await import('#lib/services/updater/updater.service');
  initUpdaterAutoCheck(5000);
});

onDestroy(() => {
  playerService.destroy();
  importProgressStore.destroy();
  taskProgressStore.destroy();
  trackNotificationService.destroy();
  mediaControlsService.disable().catch(() => {});
});

function goBack() { history.back(); }
function goForward() { history.forward(); }

// Raccourcis clavier globaux
function handleKeydown(e: KeyboardEvent) {
  // Ignorer si on est dans un input/textarea
  const tag = (e.target as HTMLElement)?.tagName;
  if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return;

  switch (e.code) {
    case 'Space':
      e.preventDefault();
      playerService.handleTogglePlay();
      break;
    case 'ArrowRight':
      if (e.ctrlKey || e.metaKey) {
        playerService.nextTrack();
      } else {
        playerService.seekTo((get(player).jsPosition ?? 0) + 10);
      }
      break;
    case 'ArrowLeft':
      if (e.ctrlKey || e.metaKey) {
        playerService.prevTrack();
      } else {
        playerService.seekTo(Math.max(0, (get(player).jsPosition ?? 0) - 10));
      }
      break;
    case 'ArrowUp':
      e.preventDefault();
      reglerVolume(get(volume) + 5);
      break;
    case 'ArrowDown':
      e.preventDefault();
      reglerVolume(get(volume) - 5);
      break;
    case 'KeyM':
      invoke('mute').then(chargerVolume);
      break;
    case 'KeyF':
    case 'KeyK':
      if (e.ctrlKey || e.metaKey) {
        e.preventDefault();
        const searchInput = document.querySelector('input[type="search"]') as HTMLInputElement;
        searchInput?.focus();
      }
      break;
  }
}
</script>

<svelte:window onkeydown={handleKeydown} />


{#if !$profilSelector.initialized}
  <!-- L'écran de chargement d'app.html reste affiché. -->
{:else if $miniPlayerActive}
  <!-- Mode mini-player : la fenêtre est réduite et always-on-top -->
  <MiniPlayer />
{:else}
<main class="w-screen h-screen flex flex-col bg-(--c-fond) dark:bg-zinc-950 text-gray-900 dark:text-gray-100 overflow-hidden">
  <!-- Barre unique : la barre de titre porte la navigation, la recherche et les
       actions. Les pages plein écran (réglages) n'en gardent que la marque. -->
  <Titlebar avecSidebar={!isFullPageRoute}>
    {#if !isFullPageRoute}
      <!-- Burger (mobile) -->
      <button
        onclick={() => sidebarStore.toggle()}
        class="flex md:hidden items-center justify-center w-9 h-9 rounded-lg cursor-pointer transition-colors
               text-neutral-600 dark:text-neutral-300 hover:bg-neutral-200/60 dark:hover:bg-white/8"
        aria-label={$t("common.menu")}
      >
        <Icon icon={$sidebarStore.open ? "lucide:x" : "lucide:menu"} class="h-5 w-5" />
      </button>

      <div class="hidden sm:flex items-center gap-0.5">
        {#each [
          { action: goBack, icone: "heroicons:chevron-left", label: "Retour" },
          { action: goForward, icone: "heroicons:chevron-right", label: "Suivant" },
        ] as nav (nav.label)}
          <button
            onclick={nav.action}
            class="flex items-center justify-center w-8 h-8 rounded-lg cursor-pointer transition-colors
                   text-[#1a1c1a] dark:text-neutral-400 hover:text-black dark:hover:text-white
                   hover:bg-[#e5e1d8] dark:hover:bg-white/8"
            aria-label={nav.label}
            title={nav.label}
          >
            <Icon icon={nav.icone} class="h-5 w-5" />
          </button>
        {/each}
      </div>

      <!-- La recherche passe avant le reste : nom du profil et « Ctrl K » cèdent d'abord. -->
      <div class="flex-1 min-w-40 max-w-140">
        <SearchAutocomplete />
      </div>

      <div class="ml-auto shrink-0 flex items-center gap-1 md:gap-2">
        {#if showSleepTimer}
          <SleepTimerButton />
        {/if}
        <SwitchTheme />
        <ProfilSelectorInput />
      </div>
    {/if}
  </Titlebar>
  <div class="flex grow overflow-hidden relative">

    <!-- ═══ SIDEBAR : fixe sur desktop, overlay sur mobile ═══ -->

    <!-- Backdrop mobile (ferme la sidebar au clic) -->
    {#if $sidebarStore.open && !isFullPageRoute}
      <button
        type="button"
        class="fixed inset-0 z-30 bg-black/50 backdrop-blur-sm cursor-default
               md:hidden"
        onclick={() => sidebarStore.close()}
        aria-label={$t("menu.close_menu")}
      ></button>
    {/if}

    <!-- Sidebar app (cachée en mode pleine page /settings) -->
    {#if !isFullPageRoute}
      <div class="
        fixed md:relative z-40 md:z-auto
        h-full
        transition-transform duration-300 ease-out
        {$sidebarStore.open ? 'translate-x-0' : '-translate-x-full md:translate-x-0'}
      ">
        <Sidebar />
      </div>
    {/if}

    <!-- ═══ CONTENU PRINCIPAL ═══ -->
    <div class="grow overflow-hidden flex flex-col min-w-0">

      <!-- Accueil et sections en haut (« en haut » ou « les deux ») ; commandes de vue dans la bibliothèque seulement. -->
      {#if tabsEnHaut}
        <div class="shrink-0 px-3 md:px-6 py-2
                    bg-neutral-100/50 dark:bg-white/2
                    border-y border-neutral-200/60 dark:border-white/6">
          <div class="flex items-center justify-between gap-2">
            <LibraryTabs libraryId={($libraryStore.librarySelected?.id as number | undefined) ?? null} />
            {#if dansLaBibliotheque}
              <LibraryViewControls />
            {/if}
          </div>
        </div>
      {/if}

      <!-- Contenu scrollable avec transition -->
      <div class="flex-1 overflow-hidden">
        {#key cleTransition}
          <div class="h-full" in:fade={{ duration: 120, delay: 60 }}>
            {@render children()}
          </div>
        {/key}
      </div>
    </div>
  </div>

  <Player />
  <QueuePanel />
  <SelectionBar />
  <Toast />
  <UpdateBanner />
  <Popin />
  <!-- Panneau flottant : un lot peut durer des minutes, l'application doit
       rester utilisable pendant ce temps. -->
  <BatchPanel />
  <ProfilSelectorPopin />
</main>
{/if}
