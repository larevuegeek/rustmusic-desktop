<script lang="ts">
  import Icon from "@iconify/svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { openPath } from "@tauri-apps/plugin-opener";
  import { appDataDir } from "@tauri-apps/api/path";
  import { settingsStore } from "$lib/stores/settings/settings.store";
  import { libraryStore } from "$lib/stores/library/library.store";
  import { t } from "$lib/i18n";
  import { detectOS } from "$lib/helper/tools/osDetection";
  import ResetConfirmPopin from "$lib/components/settings/ResetConfirmPopin.svelte";
  import AppearanceCard from "$lib/components/settings/AppearanceCard.svelte";
  import AudioDevicesCard from "$lib/components/settings/AudioDevicesCard.svelte";
  import AboutContent from "$lib/components/settings/AboutContent.svelte";
  import {
    dlnaGetSettings,
    dlnaStart,
    dlnaStop,
    dlnaUpdateSettings,
    type DlnaSettings,
  } from "$lib/services/dlna/dlna.service";
  import { dlnaStatusStore, refreshDlnaStatus } from "$lib/stores/dlna/dlna.store";
  import {
    getAudioQualityStatus,
    setAudioQualitySetting,
    type AudioQualityStatus,
    type AudioQualitySetting,
  } from "$lib/services/audio/audioQuality.service";
  import {
    getRenderMode,
    setRenderMode,
    type RenderModeStatus,
    type RenderMode,
  } from "$lib/services/system/renderMode.service";
  import { onMount } from "svelte";

  let showResetDialog = $state(false);
  let rescanning = $state(false);
  let fetchingImages = $state(false);
  let fetchingCovers = $state(false);

  // ─── DLNA section state ───
  let dlnaSettings = $state<DlnaSettings | null>(null);
  let dlnaToggling = $state(false);
  let dlnaSavingName = $state(false);
  let dlnaSavingPort = $state(false);
  let dlnaCopied = $state(false);

  // ─── Audio quality state ───
  let audioQuality = $state<AudioQualityStatus | null>(null);
  let audioQualitySaving = $state(false);

  // ─── Render mode state (Linux WebKit env vars) ───
  let renderMode = $state<RenderModeStatus | null>(null);
  let renderModeSaving = $state(false);
  let renderModeChanged = $state(false); // for "restart required" hint
  let renderModeInitial: RenderMode | null = null;

  // Load settings + status when entering the page
  onMount(async () => {
    try {
      dlnaSettings = await dlnaGetSettings();
      await refreshDlnaStatus();
    } catch (e) {
      console.error("[dlna] init failed:", e);
    }
    try {
      audioQuality = await getAudioQualityStatus();
    } catch (e) {
      console.error("[audio quality] init failed:", e);
    }
    try {
      renderMode = await getRenderMode();
      renderModeInitial = renderMode.mode;
    } catch (e) {
      console.error("[render mode] init failed:", e);
    }
  });

  async function handleAudioQualityChange(value: AudioQualitySetting) {
    if (audioQualitySaving) return;
    audioQualitySaving = true;
    try {
      audioQuality = await setAudioQualitySetting(value);
    } catch (e) {
      console.error("[audio quality] save failed:", e);
    } finally {
      audioQualitySaving = false;
    }
  }

  async function handleRenderModeChange(value: RenderMode) {
    if (renderModeSaving) return;
    renderModeSaving = true;
    try {
      renderMode = await setRenderMode(value);
      // Flag "restart required" if the user changed the value from what was
      // active at app boot (env vars are baked in at startup).
      renderModeChanged = renderModeInitial !== null && renderModeInitial !== value;
    } catch (e) {
      console.error("[render mode] save failed:", e);
    } finally {
      renderModeSaving = false;
    }
  }

  async function handleDlnaToggle() {
    if (dlnaToggling) return;
    dlnaToggling = true;
    try {
      const status = $dlnaStatusStore?.running ? await dlnaStop() : await dlnaStart();
      dlnaStatusStore.set(status);
      if (dlnaSettings) dlnaSettings = { ...dlnaSettings, enabled: status.running };
    } catch (e) {
      console.error("[dlna] toggle failed:", e);
    } finally {
      dlnaToggling = false;
    }
  }

  async function handleDlnaNameSave(value: string) {
    if (!dlnaSettings || dlnaSavingName) return;
    const trimmed = value.trim();
    if (!trimmed || trimmed === dlnaSettings.friendly_name) return;
    dlnaSavingName = true;
    try {
      const status = await dlnaUpdateSettings(trimmed, undefined);
      dlnaStatusStore.set(status);
      dlnaSettings = { ...dlnaSettings, friendly_name: status.friendly_name };
    } catch (e) {
      console.error("[dlna] update name failed:", e);
    } finally {
      dlnaSavingName = false;
    }
  }

  async function handleDlnaPortSave(value: number) {
    if (!dlnaSettings || dlnaSavingPort) return;
    if (!Number.isFinite(value) || value < 1 || value > 65535) return;
    if (value === dlnaSettings.port) return;
    dlnaSavingPort = true;
    try {
      const status = await dlnaUpdateSettings(undefined, value);
      dlnaStatusStore.set(status);
      dlnaSettings = { ...dlnaSettings, port: status.port };
    } catch (e) {
      console.error("[dlna] update port failed:", e);
    } finally {
      dlnaSavingPort = false;
    }
  }

  async function handleDlnaCopyUrl() {
    const url = $dlnaStatusStore?.url;
    if (!url) return;
    try {
      await navigator.clipboard.writeText(url);
      dlnaCopied = true;
      setTimeout(() => (dlnaCopied = false), 1500);
    } catch (e) {
      console.error("[dlna] clipboard failed:", e);
    }
  }

  async function handleFetchArtistImages() {
    fetchingImages = true;
    try {
      await invoke('fetch_all_artist_images', { force: true });
    } catch (e) {
      console.error('Failed to fetch artist images:', e);
    } finally {
      fetchingImages = false;
    }
  }

  async function handleFetchAlbumCovers() {
    fetchingCovers = true;
    try {
      const libraries = $libraryStore.libraries;
      for (const lib of libraries) {
        await invoke('fetch_all_album_covers', { libraryId: lib.id });
      }
    } catch (e) {
      console.error('Failed to fetch album covers:', e);
    } finally {
      fetchingCovers = false;
    }
  }

  async function handleRescan() {
    rescanning = true;
    try {
      const libraries = $libraryStore.libraries;
      for (const lib of libraries) {
        await invoke('rescan_library', { libraryId: lib.id });
      }
    } catch (e) {
      console.error('Rescan failed:', e);
    } finally {
      rescanning = false;
    }
  }

  // Derived depuis le store (réactif)
  let language = $derived($settingsStore.language);
  let autoStart = $derived($settingsStore.auto_start === 'true');
  let minimizeToTray = $derived($settingsStore.minimize_to_tray === 'true');
  let scanOnStartup = $derived($settingsStore.scan_on_startup === 'true');
  let singleClickPlay = $derived($settingsStore.single_click_play === 'true');
  let showSleepTimer = $derived($settingsStore.show_sleep_timer !== 'false');
  let notifications = $derived($settingsStore.show_notifications === 'true');
  let systemMediaControls = $derived($settingsStore.system_media_controls === 'true');
  let wasapiExclusive = $derived($settingsStore.wasapi_exclusive === 'true');
  let dsdDop = $derived($settingsStore.dsd_dop === 'true');
  let isWindows = $derived(detectOS() === 'windows');
  let isLinux = $derived(detectOS() === 'linux');
  let isMac = $derived(detectOS() === 'macos');

  // ─── Test WASAPI ──────────────────────────────────────────────────────
  // Bouton "Tester" qui exécute la cascade de format negotiation pour les
  // rates audio courants. Permet de vérifier si le DAC supporte WASAPI
  // exclusive AVANT d'activer le toggle pour la lecture.
  type WasapiTestRow = { rate: number; status: "ok" | "fail"; message: string };
  let wasapiTesting = $state(false);
  let wasapiDeviceName = $state<string | null>(null);
  let wasapiResults = $state<WasapiTestRow[] | null>(null);

  async function runWasapiTest() {
    wasapiTesting = true;
    wasapiResults = null;
    wasapiDeviceName = null;
    try {
      wasapiDeviceName = await invoke<string>("wasapi_default_device_name");
    } catch (e) {
      wasapiDeviceName = `Erreur device : ${e}`;
    }

    const rates = [44100, 48000, 88200, 96000, 176400, 192000];
    const rows: WasapiTestRow[] = [];
    for (const rate of rates) {
      try {
        const r = await invoke<{ sample_rate: number; bits_per_sample: number; channels: number }>(
          "wasapi_test_format_negotiation",
          { sourceRate: rate, channels: 2 },
        );
        rows.push({
          rate,
          status: "ok",
          message: `${r.sample_rate} Hz · ${r.bits_per_sample}-bit · ${r.channels} ch`,
        });
      } catch (e) {
        rows.push({ rate, status: "fail", message: String(e) });
      }
    }
    wasapiResults = rows;
    wasapiTesting = false;
  }
  let theme = $derived($settingsStore.theme);
  let windowControlsStyle = $derived($settingsStore.window_controls_style);
  let windowControlsPosition = $derived($settingsStore.window_controls_position);

  const languages = [
    { value: 'fr', label: 'Français' },
    { value: 'en', label: 'English' },
    { value: 'es', label: 'Español' },
    { value: 'de', label: 'Deutsch' },
    { value: 'it', label: 'Italiano' },
  ];

  // ─── Navigation par sections (sidebar left) ───
  type SectionId = 'general' | 'appearance' | 'audio' | 'network' | 'storage' | 'about';
  type SectionMeta = {
    id: SectionId;
    icon: string;
    labelKey: string;
    descKey: string;
    group: 'preferences' | 'app';
  };
  const sections: SectionMeta[] = [
    { id: 'general',    icon: 'lucide:settings',   labelKey: 'settings.general',    descKey: 'settings.general_desc',    group: 'preferences' },
    { id: 'appearance', icon: 'lucide:paintbrush', labelKey: 'settings.appearance', descKey: 'settings.appearance_desc', group: 'preferences' },
    { id: 'audio',      icon: 'lucide:speaker',    labelKey: 'settings.audio',      descKey: 'settings.audio_desc',      group: 'preferences' },
    { id: 'network',    icon: 'lucide:network',    labelKey: 'settings.network',    descKey: 'settings.network_desc',    group: 'preferences' },
    { id: 'storage',    icon: 'lucide:hard-drive', labelKey: 'settings.storage',    descKey: 'settings.storage_desc',    group: 'app' },
    { id: 'about',      icon: 'lucide:info',       labelKey: 'settings.about',      descKey: 'settings.about_desc',      group: 'app' },
  ];
  let activeSection: SectionId = $state('general');
  let activeSectionMeta = $derived(sections.find((s) => s.id === activeSection)!);

</script>

<div class="flex h-full">

  <!-- ═══ SIDEBAR ═══ -->
  <aside class="shrink-0 w-64 border-r border-neutral-200/70 dark:border-white/8
                bg-linear-to-b from-neutral-50/80 to-neutral-100/40
                dark:from-white/2 dark:to-transparent
                overflow-y-auto scrollbar-app flex flex-col">
    <!-- Header sidebar -->
    <div class="px-5 pt-7 pb-6 flex items-center gap-3">
      <button
        class="shrink-0 w-9 h-9 flex items-center justify-center rounded-lg cursor-pointer
               text-neutral-500 hover:text-neutral-800 dark:text-neutral-400 dark:hover:text-neutral-100
               bg-white/60 dark:bg-white/5 border border-neutral-200/70 dark:border-white/10
               hover:bg-white dark:hover:bg-white/10
               shadow-sm shadow-black/5 transition-all"
        onclick={() => history.back()}
        aria-label="Retour"
        title="Retour"
      >
        <Icon icon="lucide:arrow-left" width="15" />
      </button>
      <div class="min-w-0">
        <h1 class="text-[15px] font-bold tracking-tight text-neutral-900 dark:text-neutral-50 leading-tight">
          {$t('settings.title')}
        </h1>
        <p class="text-[10px] text-neutral-400 dark:text-neutral-500 mt-0.5 tracking-wide">
          RustMusic
        </p>
      </div>
    </div>

    <!-- Nav sections -->
    <nav class="flex-1 px-3 pb-5 space-y-6">
      <!-- Groupe : Préférences -->
      <div class="space-y-0.5">
        <p class="px-3 mb-2 text-[9px] font-semibold uppercase tracking-widest text-neutral-400 dark:text-neutral-500">
          {$t('settings.group_preferences')}
        </p>
        {#each sections.filter((s) => s.group === 'preferences') as section}
          {@const isActive = activeSection === section.id}
          <button
            class="relative w-full flex items-center gap-3 pl-3.5 pr-3 py-2.5 rounded-lg text-sm cursor-pointer
                   transition-all text-left
                   {isActive
                     ? 'bg-emerald-500/10 text-emerald-700 dark:text-emerald-400 font-medium shadow-sm shadow-emerald-500/5'
                     : 'text-neutral-600 dark:text-neutral-300 hover:bg-white dark:hover:bg-white/5 hover:text-neutral-900 dark:hover:text-neutral-100'}"
            onclick={() => (activeSection = section.id)}
          >
            {#if isActive}
              <span class="absolute left-0 top-1/2 -translate-y-1/2 w-1 h-5 rounded-r-full bg-emerald-500"></span>
            {/if}
            <Icon
              icon={section.icon}
              width="16"
              class={isActive ? '' : 'text-neutral-400 dark:text-neutral-500'}
            />
            <span>{$t(section.labelKey)}</span>
          </button>
        {/each}
      </div>

      <!-- Groupe : Application -->
      <div class="space-y-0.5">
        <p class="px-3 mb-2 text-[9px] font-semibold uppercase tracking-widest text-neutral-400 dark:text-neutral-500">
          {$t('settings.group_app')}
        </p>
        {#each sections.filter((s) => s.group === 'app') as section}
          {@const isActive = activeSection === section.id}
          <button
            class="relative w-full flex items-center gap-3 pl-3.5 pr-3 py-2.5 rounded-lg text-sm cursor-pointer
                   transition-all text-left
                   {isActive
                     ? 'bg-emerald-500/10 text-emerald-700 dark:text-emerald-400 font-medium shadow-sm shadow-emerald-500/5'
                     : 'text-neutral-600 dark:text-neutral-300 hover:bg-white dark:hover:bg-white/5 hover:text-neutral-900 dark:hover:text-neutral-100'}"
            onclick={() => (activeSection = section.id)}
          >
            {#if isActive}
              <span class="absolute left-0 top-1/2 -translate-y-1/2 w-1 h-5 rounded-r-full bg-emerald-500"></span>
            {/if}
            <Icon
              icon={section.icon}
              width="16"
              class={isActive ? '' : 'text-neutral-400 dark:text-neutral-500'}
            />
            <span>{$t(section.labelKey)}</span>
          </button>
        {/each}
      </div>
    </nav>

    <!-- Footer sidebar : version -->
    <div class="px-5 py-4 border-t border-neutral-200/60 dark:border-white/5
                text-[10px] text-neutral-400 dark:text-neutral-500 flex items-center gap-1.5">
      <Icon icon="lucide:tag" width="10" />
      <span>v0.1.7</span>
    </div>
  </aside>

  <!-- ═══ CONTENU ═══ -->
  <div class="flex-1 overflow-y-auto scrollbar-app">

  <!-- Page header sticky -->
  <div class="sticky top-0 z-10 backdrop-blur-md
              bg-white/70 dark:bg-neutral-950/70
              border-b border-neutral-200/60 dark:border-white/5
              px-8 md:px-12 py-6">
    <div class="max-w-3xl">
      <h1 class="text-2xl font-bold tracking-tight text-neutral-900 dark:text-neutral-50">
        {$t(activeSectionMeta.labelKey)}
      </h1>
      <p class="text-[13px] text-neutral-500 dark:text-neutral-400 mt-1">
        {$t(activeSectionMeta.descKey)}
      </p>
    </div>
  </div>

  <div class="px-8 md:px-12 py-8 max-w-3xl">

    <!-- ═══ GÉNÉRAL ═══ -->
    {#if activeSection === 'general'}
    <section>
      <div class="space-y-1">
        <!-- Langue -->
        <div class="flex items-center justify-between px-4 py-3 rounded-xl
                    hover:bg-neutral-50 dark:hover:bg-white/2 transition-colors">
          <div class="flex items-center gap-3">
            <Icon icon="lucide:languages" width="18" class="text-neutral-400" />
            <div>
              <p class="text-sm font-medium text-neutral-800 dark:text-neutral-200">{$t('settings.language')}</p>
              <p class="text-[11px] text-neutral-400 dark:text-neutral-500">{$t('settings.language_desc')}</p>
            </div>
          </div>
          <select
            value={language}
            onchange={(e) => settingsStore.set('language', (e.target as HTMLSelectElement).value)}
            class="text-sm appearance-none
                   bg-neutral-100 dark:bg-neutral-800/80
                   border border-neutral-200/80 dark:border-neutral-700/60
                   rounded-xl px-4 py-2 pr-8
                   text-neutral-700 dark:text-neutral-200
                   shadow-sm
                   hover:border-neutral-300 dark:hover:border-neutral-600
                   focus:outline-none focus:ring-2 focus:ring-green-500/30 focus:border-green-500/40
                   cursor-pointer transition-all duration-150
                   bg-[url('data:image/svg+xml;charset=UTF-8,%3csvg%20xmlns%3d%22http%3a//www.w3.org/2000/svg%22%20width%3d%2212%22%20height%3d%2212%22%20viewBox%3d%220%200%2024%2024%22%20fill%3d%22none%22%20stroke%3d%22%239ca3af%22%20stroke-width%3d%222%22%3e%3cpath%20d%3d%22m6%209%206%206%206-6%22/%3e%3c/svg%3e')]
                   bg-no-repeat bg-position-[right_0.75rem_center]"
          >
            {#each languages as lang}
              <option value={lang.value}>{lang.label}</option>
            {/each}
          </select>
        </div>

        <!-- Exécution automatique -->
        <div class="flex items-center justify-between px-4 py-3 rounded-xl
                    hover:bg-neutral-50 dark:hover:bg-white/2 transition-colors">
          <div class="flex items-center gap-3">
            <Icon icon="lucide:power" width="18" class="text-neutral-400" />
            <div>
              <p class="text-sm font-medium text-neutral-800 dark:text-neutral-200">{$t('settings.autostart')}</p>
              <p class="text-[11px] text-neutral-400 dark:text-neutral-500">{$t('settings.autostart_desc')}</p>
            </div>
          </div>
          <button
            class="relative w-10 h-6 rounded-full cursor-pointer transition-colors duration-200
                   {autoStart ? 'bg-green-500' : 'bg-neutral-300 dark:bg-neutral-700'}"
            aria-label="Lancement au démarrage"
            onclick={() => settingsStore.toggle('auto_start')}
          >
            <div class="absolute top-0.5 left-0.5 w-5 h-5 rounded-full bg-white shadow transition-transform duration-200
                        {autoStart ? 'translate-x-4' : ''}"></div>
          </button>
        </div>

        <!-- Minimiser dans le tray -->
        <div class="flex items-center justify-between px-4 py-3 rounded-xl
                    hover:bg-neutral-50 dark:hover:bg-white/2 transition-colors">
          <div class="flex items-center gap-3">
            <Icon icon="lucide:minimize-2" width="18" class="text-neutral-400" />
            <div>
              <p class="text-sm font-medium text-neutral-800 dark:text-neutral-200">{$t('settings.tray')}</p>
              <p class="text-[11px] text-neutral-400 dark:text-neutral-500">{$t('settings.tray_desc')}</p>
            </div>
          </div>
          <button
            class="relative w-10 h-6 rounded-full cursor-pointer transition-colors duration-200
                   {minimizeToTray ? 'bg-green-500' : 'bg-neutral-300 dark:bg-neutral-700'}"
            aria-label="Minimiser dans la barre système"
            onclick={() => settingsStore.toggle('minimize_to_tray')}
          >
            <div class="absolute top-0.5 left-0.5 w-5 h-5 rounded-full bg-white shadow transition-transform duration-200
                        {minimizeToTray ? 'translate-x-4' : ''}"></div>
          </button>
        </div>

        <!-- Scan au démarrage -->
        <div class="flex items-center justify-between px-4 py-3 rounded-xl
                    hover:bg-neutral-50 dark:hover:bg-white/2 transition-colors">
          <div class="flex items-center gap-3">
            <Icon icon="lucide:refresh-cw" width="18" class="text-neutral-400" />
            <div>
              <p class="text-sm font-medium text-neutral-800 dark:text-neutral-200">{$t('settings.scan_on_startup')}</p>
              <p class="text-[11px] text-neutral-400 dark:text-neutral-500">{$t('settings.scan_on_startup_desc')}</p>
            </div>
          </div>
          <button
            class="relative w-10 h-6 rounded-full cursor-pointer transition-colors duration-200
                   {scanOnStartup ? 'bg-green-500' : 'bg-neutral-300 dark:bg-neutral-700'}"
            aria-label="Scan au démarrage"
            onclick={() => settingsStore.toggle('scan_on_startup')}
          >
            <div class="absolute top-0.5 left-0.5 w-5 h-5 rounded-full bg-white shadow transition-transform duration-200
                        {scanOnStartup ? 'translate-x-4' : ''}"></div>
          </button>
        </div>

        <!-- Simple clic = lecture -->
        <div class="flex items-center justify-between px-4 py-3 rounded-xl
                    hover:bg-neutral-50 dark:hover:bg-white/2 transition-colors">
          <div class="flex items-center gap-3">
            <Icon icon="lucide:mouse-pointer-click" width="18" class="text-neutral-400" />
            <div>
              <p class="text-sm font-medium text-neutral-800 dark:text-neutral-200">{$t('settings.single_click_play')}</p>
              <p class="text-[11px] text-neutral-400 dark:text-neutral-500">{$t('settings.single_click_play_desc')}</p>
            </div>
          </div>
          <button
            class="relative w-10 h-6 rounded-full cursor-pointer transition-colors duration-200
                   {singleClickPlay ? 'bg-green-500' : 'bg-neutral-300 dark:bg-neutral-700'}"
            aria-label="Simple clic = lecture"
            onclick={() => settingsStore.toggle('single_click_play')}
          >
            <div class="absolute top-0.5 left-0.5 w-5 h-5 rounded-full bg-white shadow transition-transform duration-200
                        {singleClickPlay ? 'translate-x-4' : ''}"></div>
          </button>
        </div>

        <!-- Bouton minuteur de veille dans le header -->
        <div class="flex items-center justify-between px-4 py-3 rounded-xl
                    hover:bg-neutral-50 dark:hover:bg-white/2 transition-colors">
          <div class="flex items-center gap-3">
            <Icon icon="tabler:alarm-snooze" width="18" class="text-neutral-400" />
            <div>
              <p class="text-sm font-medium text-neutral-800 dark:text-neutral-200">{$t('settings.show_sleep_timer')}</p>
              <p class="text-[11px] text-neutral-400 dark:text-neutral-500">{$t('settings.show_sleep_timer_desc')}</p>
            </div>
          </div>
          <button
            class="relative w-10 h-6 rounded-full cursor-pointer transition-colors duration-200
                   {showSleepTimer ? 'bg-green-500' : 'bg-neutral-300 dark:bg-neutral-700'}"
            aria-label="Minuteur de veille"
            onclick={() => settingsStore.toggle('show_sleep_timer')}
          >
            <div class="absolute top-0.5 left-0.5 w-5 h-5 rounded-full bg-white shadow transition-transform duration-200
                        {showSleepTimer ? 'translate-x-4' : ''}"></div>
          </button>
        </div>

        <!-- Notifications OS -->
        <div class="flex items-center justify-between px-4 py-3 rounded-xl
                    hover:bg-neutral-50 dark:hover:bg-white/2 transition-colors">
          <div class="flex items-center gap-3">
            <Icon icon="lucide:bell" width="18" class="text-neutral-400" />
            <div>
              <p class="text-sm font-medium text-neutral-800 dark:text-neutral-200">{$t('settings.notifications')}</p>
              <p class="text-[11px] text-neutral-400 dark:text-neutral-500">{$t('settings.notifications_desc')}</p>
            </div>
          </div>
          <button
            class="relative w-10 h-6 rounded-full cursor-pointer transition-colors duration-200
                   {notifications ? 'bg-green-500' : 'bg-neutral-300 dark:bg-neutral-700'}"
            aria-label="Notifications"
            onclick={() => settingsStore.toggle('show_notifications')}
          >
            <div class="absolute top-0.5 left-0.5 w-5 h-5 rounded-full bg-white shadow transition-transform duration-200
                        {notifications ? 'translate-x-4' : ''}"></div>
          </button>
        </div>

        <!-- Contrôles média système (SMTC/MPRIS/NowPlaying) -->
        <div class="flex items-center justify-between px-4 py-3 rounded-xl
                    hover:bg-neutral-50 dark:hover:bg-white/2 transition-colors">
          <div class="flex items-center gap-3">
            <Icon icon="lucide:radio" width="18" class="text-neutral-400" />
            <div>
              <p class="text-sm font-medium text-neutral-800 dark:text-neutral-200">{$t('settings.system_media_controls')}</p>
              <p class="text-[11px] text-neutral-400 dark:text-neutral-500">{$t('settings.system_media_controls_desc')}</p>
            </div>
          </div>
          <button
            class="relative w-10 h-6 rounded-full cursor-pointer transition-colors duration-200
                   {systemMediaControls ? 'bg-green-500' : 'bg-neutral-300 dark:bg-neutral-700'}"
            aria-label="System media controls"
            onclick={() => settingsStore.toggle('system_media_controls')}
          >
            <div class="absolute top-0.5 left-0.5 w-5 h-5 rounded-full bg-white shadow transition-transform duration-200
                        {systemMediaControls ? 'translate-x-4' : ''}"></div>
          </button>
        </div>

      </div>
    </section>

    {/if}

    <!-- ═══ APPARENCE ═══ -->
    {#if activeSection === 'appearance'}
    <section>
      <!-- ─── Thème ─── -->
      <div class="mb-5">
        <div class="flex items-center gap-3 mb-2.5 px-1">
          <Icon icon="lucide:palette" width="18" class="text-neutral-400" />
          <div>
            <p class="text-sm font-medium text-neutral-800 dark:text-neutral-200">{$t('settings.theme')}</p>
            <p class="text-[11px] text-neutral-400 dark:text-neutral-500">{$t('settings.theme_desc')}</p>
          </div>
        </div>

        <div class="grid grid-cols-3 gap-2 max-w-2xl">
          <AppearanceCard
            label={$t('settings.theme_auto')}
            selected={theme === 'auto'}
            onclick={() => settingsStore.set('theme', 'auto')}
          >
            <div class="w-full h-full flex">
              <div class="flex-1 bg-white relative overflow-hidden">
                <div class="absolute top-2 left-2 w-1/2 h-1 rounded-full bg-neutral-300"></div>
                <div class="absolute top-4 left-2 w-1/3 h-1 rounded-full bg-neutral-200"></div>
              </div>
              <div class="flex-1 bg-neutral-900 relative overflow-hidden">
                <div class="absolute top-2 left-2 w-1/2 h-1 rounded-full bg-neutral-500"></div>
                <div class="absolute top-4 left-2 w-1/3 h-1 rounded-full bg-neutral-700"></div>
              </div>
            </div>
          </AppearanceCard>

          <AppearanceCard
            label={$t('settings.theme_light')}
            selected={theme === 'light'}
            onclick={() => settingsStore.set('theme', 'light')}
          >
            <div class="w-full h-full bg-white relative">
              <div class="absolute top-2 left-2 w-1/2 h-1 rounded-full bg-neutral-300"></div>
              <div class="absolute top-4 left-2 w-1/3 h-1 rounded-full bg-neutral-200"></div>
              <div class="absolute bottom-2 right-2 w-3 h-3 rounded-full bg-green-500"></div>
            </div>
          </AppearanceCard>

          <AppearanceCard
            label={$t('settings.theme_dark')}
            selected={theme === 'dark'}
            onclick={() => settingsStore.set('theme', 'dark')}
          >
            <div class="w-full h-full bg-neutral-900 relative">
              <div class="absolute top-2 left-2 w-1/2 h-1 rounded-full bg-neutral-500"></div>
              <div class="absolute top-4 left-2 w-1/3 h-1 rounded-full bg-neutral-700"></div>
              <div class="absolute bottom-2 right-2 w-3 h-3 rounded-full bg-green-500"></div>
            </div>
          </AppearanceCard>
        </div>
      </div>

      <!-- ─── Style des contrôles fenêtre ─── -->
      <div class="mb-5">
        <div class="flex items-center gap-3 mb-2.5 px-1">
          <Icon icon="lucide:app-window" width="18" class="text-neutral-400" />
          <div>
            <p class="text-sm font-medium text-neutral-800 dark:text-neutral-200">{$t('settings.window_controls_style')}</p>
            <p class="text-[11px] text-neutral-400 dark:text-neutral-500">{$t('settings.window_controls_style_desc')}</p>
          </div>
        </div>

        <div class="grid grid-cols-4 gap-2 max-w-2xl">
          <AppearanceCard
            label={$t('settings.auto')}
            selected={windowControlsStyle === 'auto'}
            onclick={() => settingsStore.set('window_controls_style', 'auto')}
          >
            <div class="flex items-center gap-1">
              <Icon icon="lucide:cpu" width={20} class="text-neutral-400" />
              <span class="text-[10px] text-neutral-400">OS</span>
            </div>
          </AppearanceCard>

          <AppearanceCard
            label="macOS"
            selected={windowControlsStyle === 'macos'}
            onclick={() => settingsStore.set('window_controls_style', 'macos')}
          >
            <div class="flex items-center gap-1.5">
              <div class="w-2.5 h-2.5 rounded-full bg-[#ff5f57]"></div>
              <div class="w-2.5 h-2.5 rounded-full bg-[#febc2e]"></div>
              <div class="w-2.5 h-2.5 rounded-full bg-[#28c840]"></div>
            </div>
          </AppearanceCard>

          <AppearanceCard
            label="Windows"
            selected={windowControlsStyle === 'windows'}
            onclick={() => settingsStore.set('window_controls_style', 'windows')}
          >
            <div class="flex items-center gap-2 text-neutral-500 dark:text-neutral-300">
              <svg viewBox="0 0 12 12" width="11" height="11" fill="currentColor"><rect x="2" y="5.5" width="8" height="1"/></svg>
              <svg viewBox="0 0 12 12" width="10" height="10" fill="none" stroke="currentColor" stroke-width="1"><rect x="2.5" y="2.5" width="7" height="7"/></svg>
              <svg viewBox="0 0 12 12" width="10" height="10" fill="none" stroke="currentColor" stroke-width="1.1"><path d="M2.5 2.5l7 7M9.5 2.5l-7 7"/></svg>
            </div>
          </AppearanceCard>

          <AppearanceCard
            label="Linux"
            selected={windowControlsStyle === 'linux'}
            onclick={() => settingsStore.set('window_controls_style', 'linux')}
          >
            <div class="flex items-center gap-1.5">
              <div class="w-4 h-4 rounded-full bg-neutral-200 dark:bg-white/10 flex items-center justify-center">
                <svg viewBox="0 0 12 12" width="7" height="7" fill="currentColor" class="text-neutral-500 dark:text-neutral-300"><rect x="2" y="5.5" width="8" height="1"/></svg>
              </div>
              <div class="w-4 h-4 rounded-full bg-neutral-200 dark:bg-white/10 flex items-center justify-center">
                <svg viewBox="0 0 12 12" width="7" height="7" fill="none" stroke="currentColor" stroke-width="1.4" class="text-neutral-500 dark:text-neutral-300"><rect x="2.5" y="2.5" width="7" height="7"/></svg>
              </div>
              <div class="w-4 h-4 rounded-full bg-neutral-200 dark:bg-white/10 flex items-center justify-center">
                <svg viewBox="0 0 12 12" width="7" height="7" fill="none" stroke="currentColor" stroke-width="1.4" class="text-neutral-500 dark:text-neutral-300"><path d="M2.5 2.5l7 7M9.5 2.5l-7 7"/></svg>
              </div>
            </div>
          </AppearanceCard>
        </div>
      </div>

      <!-- ─── Position des contrôles fenêtre (segmented control) ─── -->
      <div class="flex items-center justify-between px-1">
        <div class="flex items-center gap-3">
          <Icon icon="lucide:arrow-left-right" width="18" class="text-neutral-400" />
          <div>
            <p class="text-sm font-medium text-neutral-800 dark:text-neutral-200">{$t('settings.window_controls_position')}</p>
            <p class="text-[11px] text-neutral-400 dark:text-neutral-500">{$t('settings.window_controls_position_desc')}</p>
          </div>
        </div>

        <div class="inline-flex p-0.5 rounded-lg bg-neutral-100 dark:bg-white/5 border border-neutral-200/60 dark:border-white/8">
          <button
            type="button"
            class="px-3 py-1.5 text-xs font-medium rounded-md cursor-pointer transition-all
                   {windowControlsPosition === 'left'
                     ? 'bg-white dark:bg-neutral-800 text-neutral-900 dark:text-white shadow-sm'
                     : 'text-neutral-500 dark:text-neutral-400 hover:text-neutral-800 dark:hover:text-neutral-200'}"
            onclick={() => settingsStore.set('window_controls_position', 'left')}
          >
            {$t('settings.left')}
          </button>
          <button
            type="button"
            class="px-3 py-1.5 text-xs font-medium rounded-md cursor-pointer transition-all
                   {windowControlsPosition === 'right'
                     ? 'bg-white dark:bg-neutral-800 text-neutral-900 dark:text-white shadow-sm'
                     : 'text-neutral-500 dark:text-neutral-400 hover:text-neutral-800 dark:hover:text-neutral-200'}"
            onclick={() => settingsStore.set('window_controls_position', 'right')}
          >
            {$t('settings.right')}
          </button>
        </div>
      </div>

      <!-- ─── Mode de rendu (Linux seulement — vide ailleurs) ─── -->
      {#if renderMode}
        <div class="mt-5">
          <div class="flex items-center gap-3 mb-2.5 px-1">
            <Icon icon="lucide:monitor-cog" width="18" class="text-neutral-400" />
            <div>
              <p class="text-sm font-medium text-neutral-800 dark:text-neutral-200">{$t('settings.render_mode')}</p>
              <p class="text-[11px] text-neutral-400 dark:text-neutral-500">{$t('settings.render_mode_desc')}</p>
            </div>
          </div>

          <div class="grid grid-cols-3 gap-2 max-w-2xl">
            <AppearanceCard
              label={$t('settings.render_mode_auto')}
              selected={renderMode.mode === 'auto'}
              onclick={() => handleRenderModeChange('auto')}
            >
              <div class="flex items-center justify-center gap-1.5 text-neutral-400">
                <Icon icon="lucide:cpu" width={18} />
                <span class="text-[10px]">{$t('settings.audio_quality_auto_hint')}</span>
              </div>
            </AppearanceCard>

            <AppearanceCard
              label={$t('settings.render_mode_gpu')}
              selected={renderMode.mode === 'force-gpu'}
              onclick={() => handleRenderModeChange('force-gpu')}
            >
              <div class="flex items-center justify-center gap-1.5 text-emerald-500">
                <Icon icon="lucide:zap" width={18} />
                <span class="text-[10px]">GPU</span>
              </div>
            </AppearanceCard>

            <AppearanceCard
              label={$t('settings.render_mode_software')}
              selected={renderMode.mode === 'force-software'}
              onclick={() => handleRenderModeChange('force-software')}
            >
              <div class="flex items-center justify-center gap-1.5 text-sky-500">
                <Icon icon="lucide:shield-check" width={18} />
                <span class="text-[10px]">SW</span>
              </div>
            </AppearanceCard>
          </div>

          <!-- Détection environnement + alerte "restart required" -->
          {#if renderMode.virt_kind}
            <div class="mt-3 px-3 py-2 rounded-lg text-[11px] text-neutral-500 dark:text-neutral-400
                        bg-neutral-50 dark:bg-white/2 border border-neutral-200/60 dark:border-white/5
                        flex items-start gap-2 max-w-2xl">
              <Icon icon="lucide:info" width={13} class="mt-0.5 shrink-0 text-neutral-400" />
              <span>{$t('settings.render_mode_vm_detected').replace('{kind}', renderMode.virt_kind)}</span>
            </div>
          {/if}

          {#if renderModeChanged}
            <div class="mt-2 px-3 py-2 rounded-lg text-[11px]
                        bg-amber-500/10 border border-amber-500/30 text-amber-700 dark:text-amber-300
                        flex items-start gap-2 max-w-2xl">
              <Icon icon="lucide:rotate-ccw" width={13} class="mt-0.5 shrink-0" />
              <span>{$t('settings.render_mode_restart_required')}</span>
            </div>
          {/if}
        </div>
      {/if}
    </section>

    {/if}

    <!-- ═══ AUDIO (qualité de décodage) ═══ -->
    {#if activeSection === 'audio'}
    <section>
      <!-- Périphériques de sortie détectés (fréquences + formats supportés) -->
      <AudioDevicesCard />

      <!-- ── Sortie bit-perfect (WASAPI + DoP) — Windows uniquement ── -->
      {#if isWindows}
        <div class="mt-8 mb-2 rounded-2xl border border-neutral-200/70 dark:border-white/8
                    bg-neutral-50/40 dark:bg-white/2 overflow-hidden">

          <!-- Ligne principale : WASAPI exclusive -->
          <div class="flex items-center gap-3.5 px-4 py-3.5">
            <div class="shrink-0 w-9 h-9 rounded-lg flex items-center justify-center
                        {wasapiExclusive ? 'bg-amber-500/15 text-amber-500 dark:text-amber-400' : 'bg-neutral-200/60 dark:bg-white/5 text-neutral-400'}">
              <Icon icon="lucide:audio-lines" width="18" />
            </div>
            <div class="flex-1 min-w-0">
              <p class="text-sm font-medium text-neutral-800 dark:text-neutral-200 flex items-center gap-1.5">
                {$t('settings.wasapi_exclusive')}
                <span class="text-[9px] font-bold px-1.5 py-0.5 rounded bg-amber-500/15 text-amber-500 border border-amber-500/20 uppercase tracking-wider">Beta</span>
              </p>
              <p class="text-[11px] text-neutral-400 dark:text-neutral-500 mt-0.5">
                {wasapiExclusive
                  ? $t('settings.wasapi_exclusive_on_hint')
                  : $t('settings.wasapi_exclusive_desc')}
              </p>
            </div>
            <button
              class="relative w-10 h-6 rounded-full cursor-pointer transition-colors duration-200 shrink-0
                     {wasapiExclusive ? 'bg-amber-500' : 'bg-neutral-300 dark:bg-neutral-700'}"
              aria-label="WASAPI exclusive"
              onclick={() => settingsStore.toggle('wasapi_exclusive')}
            >
              <div class="absolute top-0.5 left-0.5 w-5 h-5 rounded-full bg-white shadow transition-transform duration-200
                          {wasapiExclusive ? 'translate-x-4' : ''}"></div>
            </button>
          </div>

          {#if wasapiExclusive}
            <!-- Note exclusive (discrète) -->
            <div class="px-4 pb-3 -mt-1">
              <p class="flex items-start gap-1.5 text-[10.5px] text-amber-600/90 dark:text-amber-300/70 leading-relaxed">
                <Icon icon="lucide:info" width="12" class="shrink-0 mt-0.5" />
                <span>{$t('settings.wasapi_exclusive_warning')}</span>
              </p>
            </div>

            <!-- Sous-item NESTED : DSD natif (DoP) — enfant de WASAPI -->
            <div class="border-t border-neutral-200/60 dark:border-white/5
                        bg-neutral-100/40 dark:bg-black/15">
              <div class="flex items-center gap-3.5 pl-8 pr-4 py-3.5 relative">
                <!-- Trait de hiérarchie -->
                <span class="absolute left-4 top-0 bottom-0 w-px bg-purple-400/40"></span>
                <div class="shrink-0 w-8 h-8 rounded-lg flex items-center justify-center
                            {dsdDop ? 'bg-purple-500/15 text-purple-500 dark:text-purple-400' : 'bg-neutral-200/60 dark:bg-white/5 text-neutral-400'}">
                  <Icon icon="lucide:badge-check" width="16" />
                </div>
                <div class="flex-1 min-w-0">
                  <p class="text-sm font-medium text-neutral-800 dark:text-neutral-200">
                    {$t('settings.dsd_dop')}
                  </p>
                  <p class="text-[11px] text-neutral-400 dark:text-neutral-500 mt-0.5">
                    {$t('settings.dsd_dop_desc')}
                  </p>
                </div>
                <button
                  class="relative w-10 h-6 rounded-full cursor-pointer transition-colors duration-200 shrink-0
                         {dsdDop ? 'bg-purple-500' : 'bg-neutral-300 dark:bg-neutral-700'}"
                  aria-label="DSD natif DoP"
                  onclick={() => settingsStore.toggle('dsd_dop')}
                >
                  <div class="absolute top-0.5 left-0.5 w-5 h-5 rounded-full bg-white shadow transition-transform duration-200
                              {dsdDop ? 'translate-x-4' : ''}"></div>
                </button>
              </div>

              {#if dsdDop}
                <div class="pl-8 pr-4 pb-3.5 -mt-1 relative">
                  <span class="absolute left-4 top-0 bottom-0 w-px bg-purple-400/40"></span>
                  <p class="flex items-start gap-1.5 text-[10.5px] text-purple-600/90 dark:text-purple-300/75 leading-relaxed">
                    <Icon icon="lucide:info" width="12" class="shrink-0 mt-0.5" />
                    <span>{$t('settings.dsd_dop_warning')}</span>
                  </p>
                </div>
              {/if}
            </div>

            <!-- Panel de test WASAPI (diagnostic DAC) -->
            <div class="border-t border-neutral-200/60 dark:border-white/5 px-4 py-3">
              <div class="flex items-center justify-between gap-3 mb-2">
                <div class="flex items-center gap-2 text-[11px] font-medium text-neutral-600 dark:text-neutral-300">
                  <Icon icon="lucide:flask-conical" width="13" />
                  Test compatibilité DAC
                </div>
                <button
                  onclick={runWasapiTest}
                  disabled={wasapiTesting}
                  class="px-3 py-1 rounded-md text-[11px] font-medium
                         bg-neutral-900/8 hover:bg-neutral-900/12 dark:bg-white/5 dark:hover:bg-white/10
                         text-neutral-800 dark:text-neutral-200
                         disabled:opacity-50 disabled:cursor-not-allowed cursor-pointer transition-colors"
                >
                  {wasapiTesting ? '…' : 'Tester'}
                </button>
              </div>

              {#if wasapiDeviceName}
                <p class="text-[10.5px] text-neutral-600 dark:text-neutral-400 mb-2 font-mono truncate">
                  {wasapiDeviceName}
                </p>
              {/if}

              {#if wasapiResults}
                <div class="grid grid-cols-2 gap-1">
                  {#each wasapiResults as row}
                    <div class="flex items-center gap-1.5 text-[10.5px] font-mono">
                      {#if row.status === 'ok'}
                        <Icon icon="lucide:check-circle-2" width={11} class="text-emerald-500 shrink-0" />
                        <span class="text-neutral-700 dark:text-neutral-300">{row.rate} Hz</span>
                        <span class="text-emerald-600 dark:text-emerald-400 truncate">→ {row.message}</span>
                      {:else}
                        <Icon icon="lucide:x-circle" width={11} class="text-rose-500 shrink-0" />
                        <span class="text-neutral-500 dark:text-neutral-500">{row.rate} Hz</span>
                        <span class="text-rose-500/80 truncate" title={row.message}>rejeté</span>
                      {/if}
                    </div>
                  {/each}
                </div>
              {:else if !wasapiTesting}
                <p class="text-[10.5px] text-neutral-500 leading-relaxed">
                  Vérifie quels sample rates ton DAC supporte en mode exclusive.
                </p>
              {/if}
            </div>
          {/if}
        </div>
      {/if}

      <!-- ── DSD natif (DoP) — Linux (ALSA hw exclusif) & macOS (CoreAudio
           hog mode). Pas de réglage parent à cocher d'abord, contrairement à
           Windows où le DoP dépend du toggle WASAPI exclusive. ── -->
      {#if isLinux || isMac}
        <div class="mt-8 mb-2 rounded-2xl border border-neutral-200/70 dark:border-white/8
                    bg-neutral-50/40 dark:bg-white/2 overflow-hidden">
          <div class="flex items-center gap-3.5 px-4 py-3.5">
            <div class="shrink-0 w-9 h-9 rounded-lg flex items-center justify-center
                        {dsdDop ? 'bg-purple-500/15 text-purple-500 dark:text-purple-400' : 'bg-neutral-200/60 dark:bg-white/5 text-neutral-400'}">
              <Icon icon="lucide:badge-check" width="18" />
            </div>
            <div class="flex-1 min-w-0">
              <p class="text-sm font-medium text-neutral-800 dark:text-neutral-200 flex items-center gap-1.5">
                {$t('settings.dsd_dop')}
                <span class="text-[9px] font-bold px-1.5 py-0.5 rounded bg-amber-500/15 text-amber-500 border border-amber-500/20 uppercase tracking-wider">Beta</span>
              </p>
              <p class="text-[11px] text-neutral-400 dark:text-neutral-500 mt-0.5">
                {$t('settings.dsd_dop_desc')}
              </p>
            </div>
            <button
              class="relative w-10 h-6 rounded-full cursor-pointer transition-colors duration-200 shrink-0
                     {dsdDop ? 'bg-purple-500' : 'bg-neutral-300 dark:bg-neutral-700'}"
              aria-label="DSD natif DoP"
              onclick={() => settingsStore.toggle('dsd_dop')}
            >
              <div class="absolute top-0.5 left-0.5 w-5 h-5 rounded-full bg-white shadow transition-transform duration-200
                          {dsdDop ? 'translate-x-4' : ''}"></div>
            </button>
          </div>

          {#if dsdDop}
            <div class="px-4 pb-3.5 -mt-1">
              <p class="flex items-start gap-1.5 text-[10.5px] text-purple-600/90 dark:text-purple-300/75 leading-relaxed">
                <Icon icon="lucide:info" width="12" class="shrink-0 mt-0.5" />
                <span>{$t('settings.dsd_dop_warning')}</span>
              </p>
            </div>
          {/if}
        </div>
      {/if}

      <div class="mb-2 mt-8">
        <div class="flex items-center gap-3 mb-2.5 px-1">
          <Icon icon="lucide:gauge" width="18" class="text-neutral-400" />
          <div>
            <p class="text-sm font-medium text-neutral-800 dark:text-neutral-200">{$t('settings.audio_quality')}</p>
            <p class="text-[11px] text-neutral-400 dark:text-neutral-500">{$t('settings.audio_quality_desc')}</p>
          </div>
        </div>

        <div class="grid grid-cols-2 md:grid-cols-5 gap-2 max-w-3xl">
          <!-- Auto -->
          <AppearanceCard
            label={$t('settings.audio_quality_auto')}
            selected={audioQuality?.setting === 'auto'}
            onclick={() => handleAudioQualityChange('auto')}
          >
            <div class="flex items-center justify-center gap-1.5 text-neutral-400">
              <Icon icon="lucide:cpu" width={18} />
              <span class="text-[10px]">{$t('settings.audio_quality_auto_hint')}</span>
            </div>
          </AppearanceCard>

          <!-- High -->
          <AppearanceCard
            label={$t('settings.audio_quality_high')}
            selected={audioQuality?.setting === 'high'}
            onclick={() => handleAudioQualityChange('high')}
          >
            <div class="flex items-end gap-0.5 h-6">
              <div class="w-1.5 h-2 rounded-sm bg-green-500"></div>
              <div class="w-1.5 h-3 rounded-sm bg-green-500"></div>
              <div class="w-1.5 h-4 rounded-sm bg-green-500"></div>
              <div class="w-1.5 h-5 rounded-sm bg-green-500"></div>
              <div class="w-1.5 h-6 rounded-sm bg-green-500"></div>
            </div>
          </AppearanceCard>

          <!-- Medium -->
          <AppearanceCard
            label={$t('settings.audio_quality_medium')}
            selected={audioQuality?.setting === 'medium'}
            onclick={() => handleAudioQualityChange('medium')}
          >
            <div class="flex items-end gap-0.5 h-6">
              <div class="w-1.5 h-2 rounded-sm bg-amber-500"></div>
              <div class="w-1.5 h-3 rounded-sm bg-amber-500"></div>
              <div class="w-1.5 h-4 rounded-sm bg-amber-500"></div>
              <div class="w-1.5 h-2 rounded-sm bg-neutral-300 dark:bg-neutral-700"></div>
              <div class="w-1.5 h-2 rounded-sm bg-neutral-300 dark:bg-neutral-700"></div>
            </div>
          </AppearanceCard>

          <!-- Low -->
          <AppearanceCard
            label={$t('settings.audio_quality_low')}
            selected={audioQuality?.setting === 'low'}
            onclick={() => handleAudioQualityChange('low')}
          >
            <div class="flex items-end gap-0.5 h-6">
              <div class="w-1.5 h-2 rounded-sm bg-sky-500"></div>
              <div class="w-1.5 h-3 rounded-sm bg-sky-500"></div>
              <div class="w-1.5 h-2 rounded-sm bg-neutral-300 dark:bg-neutral-700"></div>
              <div class="w-1.5 h-2 rounded-sm bg-neutral-300 dark:bg-neutral-700"></div>
              <div class="w-1.5 h-2 rounded-sm bg-neutral-300 dark:bg-neutral-700"></div>
            </div>
          </AppearanceCard>

          <!-- Minimal -->
          <AppearanceCard
            label={$t('settings.audio_quality_minimal')}
            selected={audioQuality?.setting === 'minimal'}
            onclick={() => handleAudioQualityChange('minimal')}
          >
            <div class="flex items-end gap-0.5 h-6">
              <div class="w-1.5 h-2 rounded-sm bg-rose-500"></div>
              <div class="w-1.5 h-2 rounded-sm bg-neutral-300 dark:bg-neutral-700"></div>
              <div class="w-1.5 h-2 rounded-sm bg-neutral-300 dark:bg-neutral-700"></div>
              <div class="w-1.5 h-2 rounded-sm bg-neutral-300 dark:bg-neutral-700"></div>
              <div class="w-1.5 h-2 rounded-sm bg-neutral-300 dark:bg-neutral-700"></div>
            </div>
          </AppearanceCard>
        </div>

        <!-- Info sur le profil actuellement résolu + host -->
        {#if audioQuality}
          <div class="mt-3 px-3 py-2 rounded-lg text-[11px] text-neutral-500 dark:text-neutral-400
                      bg-neutral-50 dark:bg-white/2 border border-neutral-200/60 dark:border-white/5
                      flex items-start gap-2 max-w-2xl">
            <Icon icon="lucide:info" width={13} class="mt-0.5 shrink-0 text-neutral-400" />
            <div>
              {#if audioQuality.setting === 'auto'}
                {$t('settings.audio_quality_auto_resolved')
                  .replace('{profile}', $t(`settings.audio_quality_${audioQuality.resolved}`))}
              {:else}
                {$t(`settings.audio_quality_${audioQuality.setting}_desc`)}
              {/if}
              <span class="block mt-0.5 text-neutral-400 dark:text-neutral-500">
                {#if audioQuality.virt_kind}
                  {$t('settings.audio_quality_host_vm').replace('{kind}', audioQuality.virt_kind)}
                {:else}
                  {$t('settings.audio_quality_host_native')}
                {/if}
                · {$t('settings.audio_quality_host_cores').replace('{n}', String(audioQuality.cpu_cores))}
              </span>
            </div>
          </div>
        {/if}
      </div>
    </section>

    {/if}

    <!-- ═══ RÉSEAU / DLNA ═══ -->
    {#if activeSection === 'network'}
    <section>
      <div class="space-y-1">
        <!-- Toggle ON/OFF -->
        <div class="flex items-center justify-between px-4 py-3 rounded-xl
                    hover:bg-neutral-50 dark:hover:bg-white/2 transition-colors">
          <div class="flex items-center gap-3 min-w-0">
            <Icon icon="lucide:radio-tower" width="18" class="text-neutral-400 shrink-0" />
            <div class="min-w-0">
              <p class="text-sm font-medium text-neutral-800 dark:text-neutral-200">{$t('settings.dlna_enabled')}</p>
              <p class="text-[11px] text-neutral-400 dark:text-neutral-500">{$t('settings.dlna_enabled_desc')}</p>
            </div>
          </div>
          <button
            type="button"
            class="relative inline-flex h-6 w-11 items-center rounded-full transition-colors cursor-pointer
                   {$dlnaStatusStore?.running ? 'bg-emerald-500' : 'bg-neutral-300 dark:bg-neutral-700'}
                   {dlnaToggling ? 'opacity-50 cursor-wait' : ''}"
            onclick={handleDlnaToggle}
            disabled={dlnaToggling}
            aria-label={$dlnaStatusStore?.running ? 'Désactiver DLNA' : 'Activer DLNA'}
          >
            <span
              class="inline-block h-4 w-4 transform rounded-full bg-white shadow-sm transition-transform
                     {$dlnaStatusStore?.running ? 'translate-x-6' : 'translate-x-1'}"
            ></span>
          </button>
        </div>

        <!-- URL active (visible seulement quand le serveur tourne) -->
        {#if $dlnaStatusStore?.running && $dlnaStatusStore.url}
          <div class="flex items-center justify-between px-4 py-3 rounded-xl
                      bg-emerald-50 dark:bg-emerald-500/8 border border-emerald-200/60 dark:border-emerald-500/20">
            <div class="flex items-center gap-3 min-w-0">
              <div class="relative flex h-2.5 w-2.5 items-center justify-center shrink-0">
                <span class="absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-50 animate-ping"></span>
                <span class="relative inline-flex h-2 w-2 rounded-full bg-emerald-500 shadow-[0_0_6px_rgba(16,185,129,0.6)]"></span>
              </div>
              <div class="min-w-0">
                <p class="text-[11px] font-semibold uppercase tracking-wider text-emerald-700 dark:text-emerald-400">{$t('settings.dlna_active')}</p>
                <p class="text-sm font-mono text-neutral-800 dark:text-neutral-200 truncate">{$dlnaStatusStore.url}</p>
              </div>
            </div>
            <button
              type="button"
              class="text-[11px] px-3 py-1.5 rounded-md cursor-pointer
                     bg-white dark:bg-white/10 border border-neutral-200 dark:border-white/15
                     hover:bg-neutral-50 dark:hover:bg-white/15 text-neutral-700 dark:text-neutral-200
                     transition-colors flex items-center gap-1.5 shrink-0"
              onclick={handleDlnaCopyUrl}
              aria-label="Copier l'URL"
            >
              <Icon icon={dlnaCopied ? 'lucide:check' : 'lucide:copy'} width={12} />
              {dlnaCopied ? $t('settings.dlna_copied') : $t('settings.dlna_copy_url')}
            </button>
          </div>
        {/if}

        <!-- Friendly name -->
        {#if dlnaSettings}
          <div class="flex items-center justify-between px-4 py-3 rounded-xl
                      hover:bg-neutral-50 dark:hover:bg-white/2 transition-colors gap-4">
            <div class="flex items-center gap-3 min-w-0">
              <Icon icon="lucide:tag" width="18" class="text-neutral-400 shrink-0" />
              <div class="min-w-0">
                <p class="text-sm font-medium text-neutral-800 dark:text-neutral-200">{$t('settings.dlna_friendly_name')}</p>
                <p class="text-[11px] text-neutral-400 dark:text-neutral-500">{$t('settings.dlna_friendly_name_desc')}</p>
              </div>
            </div>
            <input
              type="text"
              value={dlnaSettings.friendly_name}
              onblur={(e) => handleDlnaNameSave(e.currentTarget.value)}
              maxlength="64"
              class="text-sm w-56 px-3 py-1.5 rounded-md
                     bg-white dark:bg-white/5 border border-neutral-200 dark:border-white/10
                     text-neutral-800 dark:text-neutral-200
                     focus:outline-none focus:border-emerald-400 dark:focus:border-emerald-500"
            />
          </div>

          <!-- Port -->
          <div class="flex items-center justify-between px-4 py-3 rounded-xl
                      hover:bg-neutral-50 dark:hover:bg-white/2 transition-colors gap-4">
            <div class="flex items-center gap-3 min-w-0">
              <Icon icon="lucide:plug" width="18" class="text-neutral-400 shrink-0" />
              <div class="min-w-0">
                <p class="text-sm font-medium text-neutral-800 dark:text-neutral-200">{$t('settings.dlna_port')}</p>
                <p class="text-[11px] text-neutral-400 dark:text-neutral-500">{$t('settings.dlna_port_desc')}</p>
              </div>
            </div>
            <input
              type="number"
              min="1"
              max="65535"
              value={dlnaSettings.port}
              onblur={(e) => handleDlnaPortSave(parseInt(e.currentTarget.value, 10))}
              class="text-sm w-24 px-3 py-1.5 rounded-md tabular-nums text-right
                     bg-white dark:bg-white/5 border border-neutral-200 dark:border-white/10
                     text-neutral-800 dark:text-neutral-200
                     focus:outline-none focus:border-emerald-400 dark:focus:border-emerald-500"
            />
          </div>
        {/if}
      </div>
    </section>

    {/if}

    <!-- ═══ STOCKAGE ═══ -->
    {#if activeSection === 'storage'}
    <section>
      <div class="space-y-1">
        <div class="flex items-center justify-between px-4 py-3 rounded-xl
                    hover:bg-neutral-50 dark:hover:bg-white/2 transition-colors">
          <div class="flex items-center gap-3">
            <Icon icon="lucide:refresh-cw" width="18" class="text-neutral-400" />
            <div>
              <p class="text-sm font-medium text-neutral-800 dark:text-neutral-200">{$t('settings.rescan_library')}</p>
              <p class="text-[11px] text-neutral-400 dark:text-neutral-500">{$t('settings.rescan_library_desc')}</p>
            </div>
          </div>
          <button
            class="px-3 py-1.5 rounded-lg text-xs font-medium cursor-pointer
                   border border-neutral-200 dark:border-neutral-700
                   text-neutral-600 dark:text-neutral-400
                   hover:bg-neutral-100 dark:hover:bg-neutral-800
                   disabled:opacity-50
                   transition-colors"
            disabled={rescanning}
            onclick={handleRescan}
          >
            <Icon icon={rescanning ? "lucide:loader-2" : "lucide:refresh-cw"} width="14" class="inline mr-1 {rescanning ? 'animate-spin' : ''}" />
            {$t('settings.rescan_btn')}
          </button>
        </div>

        <div class="flex items-center justify-between px-4 py-3 rounded-xl
                    hover:bg-neutral-50 dark:hover:bg-white/2 transition-colors">
          <div class="flex items-center gap-3">
            <Icon icon="lucide:image-down" width="18" class="text-neutral-400" />
            <div>
              <p class="text-sm font-medium text-neutral-800 dark:text-neutral-200">{$t('common.artist_images')}</p>
              <p class="text-[11px] text-neutral-400 dark:text-neutral-500">{$t('common.artist_images_desc')}</p>
            </div>
          </div>
          <button
            class="px-3 py-1.5 rounded-lg text-xs font-medium cursor-pointer
                   border border-neutral-200 dark:border-neutral-700
                   text-neutral-600 dark:text-neutral-400
                   hover:bg-neutral-100 dark:hover:bg-neutral-800
                   disabled:opacity-50
                   transition-colors"
            disabled={fetchingImages}
            onclick={handleFetchArtistImages}
          >
            <Icon icon={fetchingImages ? "lucide:loader-2" : "lucide:download"} width="14" class="inline mr-1 {fetchingImages ? 'animate-spin' : ''}" />
            {fetchingImages ? $t('common.fetching') : $t('common.fetch')}
          </button>
        </div>

        <div class="flex items-center justify-between px-4 py-3 rounded-xl
                    hover:bg-neutral-50 dark:hover:bg-white/2 transition-colors">
          <div class="flex items-center gap-3">
            <Icon icon="lucide:disc-album" width="18" class="text-neutral-400" />
            <div>
              <p class="text-sm font-medium text-neutral-800 dark:text-neutral-200">{$t('settings.album_covers')}</p>
              <p class="text-[11px] text-neutral-400 dark:text-neutral-500">{$t('settings.album_covers_desc')}</p>
            </div>
          </div>
          <button
            class="px-3 py-1.5 rounded-lg text-xs font-medium cursor-pointer
                   border border-neutral-200 dark:border-neutral-700
                   text-neutral-600 dark:text-neutral-400
                   hover:bg-neutral-100 dark:hover:bg-neutral-800
                   disabled:opacity-50
                   transition-colors"
            disabled={fetchingCovers}
            onclick={handleFetchAlbumCovers}
          >
            <Icon icon={fetchingCovers ? "lucide:loader-2" : "lucide:download"} width="14" class="inline mr-1 {fetchingCovers ? 'animate-spin' : ''}" />
            {fetchingCovers ? $t('common.fetching') : $t('common.fetch')}
          </button>
        </div>

        <!-- Ouvrir le dossier de données -->
        <div class="flex items-center justify-between px-4 py-3 rounded-xl
                    hover:bg-neutral-50 dark:hover:bg-white/2 transition-colors">
          <div class="flex items-center gap-3">
            <Icon icon="lucide:folder-open" width="18" class="text-neutral-400" />
            <div>
              <p class="text-sm font-medium text-neutral-800 dark:text-neutral-200">{$t('settings.open_data_folder')}</p>
              <p class="text-[11px] text-neutral-400 dark:text-neutral-500">{$t('settings.open_data_folder_desc')}</p>
            </div>
          </div>
          <button
            class="px-3 py-1.5 rounded-lg text-xs font-medium cursor-pointer
                   border border-neutral-200 dark:border-neutral-700
                   text-neutral-600 dark:text-neutral-400
                   hover:bg-neutral-100 dark:hover:bg-neutral-800
                   transition-colors"
            onclick={async () => { const dir = await appDataDir(); await openPath(dir); }}
          >
            <Icon icon="lucide:external-link" width="14" class="inline mr-1" />
            {$t('settings.open_btn')}
          </button>
        </div>

        <div class="flex items-center justify-between px-4 py-3 rounded-xl
                    hover:bg-neutral-50 dark:hover:bg-white/2 transition-colors">
          <div class="flex items-center gap-3">
            <Icon icon="lucide:rotate-ccw" width="18" class="text-red-400/60" />
            <div>
              <p class="text-sm font-medium text-neutral-800 dark:text-neutral-200">{$t('settings.reset')}</p>
              <p class="text-[11px] text-neutral-400 dark:text-neutral-500">{$t('settings.reset_desc')}</p>
            </div>
          </div>
          <button
            class="px-3 py-1.5 rounded-lg text-xs font-medium cursor-pointer
                   border border-red-200 dark:border-red-800/50
                   text-red-500 dark:text-red-400
                   hover:bg-red-50 dark:hover:bg-red-900/20
                   transition-colors"
            onclick={() => showResetDialog = true}
          >
            <Icon icon="lucide:trash-2" width="14" class="inline mr-1" />
            {$t('settings.reset_btn')}
          </button>
        </div>
      </div>
    </section>

    {/if}

    <!-- ═══ À PROPOS ═══ -->
    {#if activeSection === 'about'}
    <section>
      <AboutContent />
    </section>
    {/if}
  </div>
  </div>
</div>

{#if showResetDialog}
  <ResetConfirmPopin
    bind:open={showResetDialog}
  />
{/if}
