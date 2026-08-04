<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import Icon from "@iconify/svelte";
  import {
    audioDevicesStore,
    formatSampleRate,
    maxSampleRate,
    bestFormatLabel,
    type AudioDeviceInfo,
  } from "$lib/stores/audio/audioDevices.store";
  import AudioDeviceDetailsModal from "$lib/components/settings/AudioDeviceDetailsModal.svelte";
  import { settingsStore } from "$lib/stores/settings/settings.store";
  import { detectOS } from "$lib/helper/tools/osDetection";
  import { t } from "$lib/i18n";
  import { playbackPipelineStore } from "$lib/stores/player/playbackPipeline.store";

  let isWindows = detectOS() === "windows";
  let wasapiExclusive = $derived($settingsStore.wasapi_exclusive === "true");

  // DoP actif en lecture : le volume logiciel est bit-perfect (figé). On grise
  // le slider — le volume se règle physiquement sur le DAC.
  // Un backend DoP par OS ("WASAPI DoP", "ALSA DoP", "CoreAudio DoP").
  let dopActive = $derived(
    $playbackPipelineStore?.backend?.endsWith("DoP") === true,
  );

  function toggleWasapi() {
    settingsStore.toggle("wasapi_exclusive");
  }

  let value = $state(80);
  let previousValue = $state(80);
  let isMuted = $derived(value === 0);
  let showPercent = $state(false);

  // 0 = mute, 1 = low, 2 = high
  let level = $derived(value === 0 ? 0 : value < 50 ? 1 : 2);

  // Devices (enrichis avec capacités DAC).
  // On track la sélection par displayName (unique après dédup backend) et
  // non par le nom brut CPAL qui peut être partagé entre 2 endpoints.
  let selectedDisplayName: string | null = $state(null);
  let showDevices = $state(false);
  let deviceBtnEl: HTMLElement | null = $state(null);
  let modalDevice: AudioDeviceInfo | null = $state(null);

  function openDeviceDetails(device: AudioDeviceInfo) {
    modalDevice = device;
    showDevices = false;
  }
  function closeModal() {
    modalDevice = null;
  }
  function handleModalSelect(device: AudioDeviceInfo) {
    selectedDisplayName = device.displayName;
    audioDevicesStore.setActive(device.displayName);
  }

  let devices = $derived($audioDevicesStore.devices);

  onMount(async () => {
    try {
      value = await invoke<number>('get_volume');
      previousValue = value;
    } catch (e) {
      console.error('Failed to get volume', e);
    }

    try {
      await audioDevicesStore.ensureLoaded();
      const list = $audioDevicesStore.devices;
      const def = list.find((d) => d.isDefault) ?? list[0];
      if (def) selectedDisplayName = def.displayName;
    } catch (e) {
      console.error('Failed to get devices', e);
    }
  });

  let volumeTimer: ReturnType<typeof setTimeout> | null = null;

  function handleChange() {
    // Debounce : envoyer au max toutes les 50ms au lieu de chaque pixel
    if (volumeTimer) clearTimeout(volumeTimer);
    volumeTimer = setTimeout(async () => {
      try {
        await invoke('set_volume', { volume: value });
      } catch (e) {
        console.error('Failed to set volume', e);
      }
    }, 50);
  }

  function toggleMute() {
    if (isMuted) {
      value = previousValue > 0 ? previousValue : 80;
    } else {
      previousValue = value;
      value = 0;
    }
    handleChange();
  }

  function toggleDevices() {
    showDevices = !showDevices;
  }

  async function selectDevice(device: AudioDeviceInfo) {
    selectedDisplayName = device.displayName;
    audioDevicesStore.setActive(device.displayName);
    showDevices = false;
    // Envoyer le nom brut CPAL au backend pour les prochaines lectures.
    try {
      await invoke('set_device', { deviceName: device.name });
    } catch (e) {
      console.error('Failed to set device', e);
    }
  }

  // Fermer le dropdown au clic extérieur
  $effect(() => {
    if (!showDevices) return;

    function handleClickOutside(e: MouseEvent) {
      if (deviceBtnEl && !deviceBtnEl.contains(e.target as Node)) {
        showDevices = false;
      }
    }

    const timer = setTimeout(() => {
      document.addEventListener('click', handleClickOutside);
    }, 50);

    return () => {
      clearTimeout(timer);
      document.removeEventListener('click', handleClickOutside);
    };
  });
</script>

<div
  class="flex items-center gap-2"
  onmouseenter={() => showPercent = true}
  onmouseleave={() => showPercent = false}
  role="group"
>
  <!-- Bouton device audio -->
  <div class="relative" bind:this={deviceBtnEl}>
    <button
      class="cursor-pointer shrink-0 flex items-center justify-center w-8 h-8 rounded-full
             text-neutral-500 dark:text-neutral-400 hover:text-emerald-500 dark:hover:text-emerald-400 transition-colors"
      onclick={toggleDevices}
      aria-label="Périphérique audio"
    >
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" class="w-5 h-5">
        <rect x="4" y="2" width="16" height="20" rx="2" />
        <circle cx="12" cy="14" r="4" />
        <line x1="12" y1="6" x2="12.01" y2="6" />
      </svg>
    </button>

    <!-- Dropdown devices enrichi (nom, format max, fréquence max, badges) -->
    {#if showDevices && devices.length > 0}
      <div class="absolute bottom-8 right-0 z-50 w-72
                  rounded-lg border border-white/10 bg-neutral-950/95 backdrop-blur-xl
                  shadow-2xl shadow-black/40 p-1.5
                  text-sm text-neutral-200">
        <div class="px-2.5 py-1.5 text-[10px] font-semibold uppercase tracking-widest text-neutral-500">
          Sortie audio
        </div>
        {#each devices as device (device.displayName)}
          {@const isSelected = selectedDisplayName === device.displayName}
          {@const maxRate = maxSampleRate(device.sampleRates)}
          {@const bestFmt = bestFormatLabel(device.sampleFormats)}
          <div
            class="w-full flex items-stretch rounded-md transition-colors duration-100
                   {isSelected ? 'bg-emerald-500/15' : 'hover:bg-white/8'}"
          >
            <button
              class="flex-1 min-w-0 flex items-start gap-2.5 px-2.5 py-2 rounded-l-md text-left cursor-pointer
                     {isSelected ? 'text-emerald-400' : 'text-neutral-300'}"
              onclick={() => selectDevice(device)}
              title="Sélectionner comme sortie"
            >
              <!-- Speaker icon -->
              <svg class="w-4 h-4 shrink-0 mt-0.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
                <path d="M11 5L6 9H2v6h4l5 4V5z" />
                {#if isSelected}
                  <path d="M15.54 8.46a5 5 0 0 1 0 7.07" />
                  <path d="M19.07 4.93a10 10 0 0 1 0 14.14" />
                {/if}
              </svg>
              <div class="min-w-0 flex-1">
                <div class="flex items-center gap-1.5">
                  <span class="truncate text-xs font-medium">{device.displayName}</span>
                  {#if device.isDefault}
                    <span class="shrink-0 text-[9px] px-1 py-px rounded
                                 bg-neutral-500/20 text-neutral-400 uppercase tracking-wider">
                      default
                    </span>
                  {/if}
                </div>
                <div class="mt-0.5 flex items-center gap-1.5 text-[10px] text-neutral-500">
                  {#if bestFmt}
                    <span>{bestFmt}</span>
                  {/if}
                  {#if maxRate}
                    {#if bestFmt}<span class="text-neutral-700">·</span>{/if}
                    <span>{formatSampleRate(maxRate)}</span>
                  {/if}
                  {#if device.isHires}
                    <span class="ml-auto shrink-0 text-[9px] px-1.5 py-px rounded
                                 bg-amber-500/20 text-amber-300 font-semibold tracking-wide">
                      Hi-Res
                    </span>
                  {/if}
                </div>
              </div>
            </button>

            <!-- Bouton info : ouvre le modal détaillé -->
            <button
              class="shrink-0 flex items-center justify-center w-8 rounded-r-md cursor-pointer
                     text-neutral-500 hover:text-neutral-200 transition-colors"
              onclick={() => openDeviceDetails(device)}
              title="Voir les détails du périphérique"
              aria-label="Détails du périphérique"
            >
              <Icon icon="lucide:info" width="13" />
            </button>
          </div>
        {/each}

        <!-- Toggle WASAPI exclusive (Windows uniquement) -->
        {#if isWindows}
          <div class="mt-1 pt-1.5 border-t border-white/8">
            <button
              class="w-full flex items-center gap-2.5 px-2.5 py-2 rounded-md cursor-pointer
                     transition-colors hover:bg-white/8 text-left"
              onclick={toggleWasapi}
            >
              <Icon
                icon="lucide:audio-lines"
                width="15"
                class="shrink-0 {wasapiExclusive ? 'text-amber-400' : 'text-neutral-500'}"
              />
              <div class="min-w-0 flex-1">
                <div class="text-xs font-medium text-neutral-200">
                  {$t("settings.wasapi_exclusive")}
                </div>
                <div class="text-[10px] text-neutral-500">
                  {wasapiExclusive
                    ? $t("player.wasapi_on_short")
                    : $t("player.wasapi_off_short")}
                </div>
              </div>
              <!-- Mini switch -->
              <div
                class="shrink-0 relative w-9 h-5 rounded-full transition-colors duration-200
                       {wasapiExclusive ? 'bg-amber-500' : 'bg-neutral-700'}"
              >
                <div
                  class="absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white shadow transition-transform duration-200
                         {wasapiExclusive ? 'translate-x-4' : ''}"
                ></div>
              </div>
            </button>
          </div>
        {/if}
      </div>
    {/if}
  </div>

  <!-- Icône volume (SVG inline, pas de flash). Désactivée en DoP (bit-perfect). -->
  <button
    disabled={dopActive}
    title={dopActive ? $t('player.volume_dop_locked') : ''}
    class="shrink-0 flex items-center justify-center w-8 h-8 rounded-full transition-colors
           {dopActive
             ? 'text-neutral-300 dark:text-neutral-600 cursor-not-allowed'
             : isMuted ? 'text-red-400/70 cursor-pointer' : 'text-neutral-500 dark:text-neutral-400 hover:text-emerald-500 dark:hover:text-emerald-400 cursor-pointer'}"
    onclick={toggleMute}
    aria-label={isMuted ? 'Activer le son' : 'Couper le son'}
  >
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" class="w-5 h-5">
      <path d="M11 5L6 9H2v6h4l5 4V5z" />
      {#if level === 0}
        <line x1="23" y1="9" x2="17" y2="15" />
        <line x1="17" y1="9" x2="23" y2="15" />
      {:else if level === 1}
        <path d="M15.54 8.46a5 5 0 0 1 0 7.07" />
      {:else}
        <path d="M15.54 8.46a5 5 0 0 1 0 7.07" />
        <path d="M19.07 4.93a10 10 0 0 1 0 14.14" />
      {/if}
    </svg>
  </button>

  <!-- Slider (grisé + verrouillé en DoP : volume géré par le DAC) -->
  <div
    class="relative w-24 group/vol {dopActive ? 'opacity-40' : ''}"
    role="presentation"
    title={dopActive ? $t('player.volume_dop_locked') : ''}
  >
    <div class="relative h-1.5 rounded-full bg-neutral-300 dark:bg-neutral-700/50 overflow-hidden">
      <div
        class="absolute inset-y-0 left-0 rounded-full transition-[width] duration-75
               {dopActive ? 'bg-neutral-400 dark:bg-neutral-600' : isMuted ? 'bg-red-400/50' : 'bg-emerald-500'}"
        style="width: {dopActive ? 100 : value}%"
      ></div>
    </div>

    {#if !dopActive}
      <div
        class="absolute top-1/2 -translate-y-1/2 w-3.5 h-3.5 rounded-full
               bg-white shadow-md shadow-black/20
               border-2 transition-all duration-75
               opacity-0 group-hover/vol:opacity-100
               {isMuted ? 'border-red-400' : 'border-emerald-500'}"
        style="left: calc({value}% - 7px)"
      ></div>

      <input
        type="range"
        min="0"
        max="100"
        step="1"
        bind:value={value}
        oninput={handleChange}
        class="absolute inset-0 w-full h-4 -top-1.5 cursor-pointer opacity-0"
        aria-label="Volume"
      />
    {/if}
  </div>

  <!-- Pourcentage -->
  <span
    class="text-[10px] tabular-nums font-medium w-7 text-right shrink-0 transition-opacity duration-150
           {showPercent ? 'opacity-100' : 'opacity-0'}
           {isMuted ? 'text-red-400/70' : 'text-neutral-500'}"
  >
    {value}%
  </span>
</div>

{#if modalDevice}
  {@const activeDn = $audioDevicesStore.activeDisplayName}
  <AudioDeviceDetailsModal
    device={modalDevice}
    isActive={activeDn === modalDevice.displayName}
    onClose={closeModal}
    onSelect={handleModalSelect}
  />
{/if}
