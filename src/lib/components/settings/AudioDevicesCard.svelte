<script lang="ts">
  // Sélecteur de sortie audio : une liste de radios façon réglages son de
  // l'OS, mais avec la matière visuelle de l'app (tuile d'icône, liseré
  // d'accent, halo émeraude sur la sortie active). Chaque ligne garde son
  // propre accès aux capacités détaillées du DAC.
  import Icon from "@iconify/svelte";
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import {
    audioDevicesStore,
    formatSampleRate,
    maxSampleRate,
    type AudioDeviceInfo,
  } from "$lib/stores/audio/audioDevices.store";
  import { t } from "$lib/i18n";
  import AudioDeviceDetailsModal from "./AudioDeviceDetailsModal.svelte";

  let detailsOf = $state<AudioDeviceInfo | null>(null);

  onMount(() => {
    audioDevicesStore.ensureLoaded();
  });

  async function select(device: AudioDeviceInfo) {
    if (device.displayName === $audioDevicesStore.activeDisplayName) return;
    try {
      await invoke("set_device", { deviceName: device.name });
      audioDevicesStore.setActive(device.displayName);
    } catch (err) {
      console.error("Failed to set device", err);
    }
  }
</script>

<div class="mb-2">
  <div class="flex items-center justify-between mb-2.5 px-1">
    <div class="flex items-center gap-3">
      <Icon icon="lucide:speaker" width="18" class="text-neutral-400" />
      <div>
        <p class="text-sm font-medium text-neutral-800 dark:text-neutral-200">
          {$t("settings.audio_devices")}
        </p>
        <p class="text-[11px] text-neutral-400 dark:text-neutral-500">
          {$t("settings.audio_devices_desc")}
        </p>
      </div>
    </div>
    <button
      class="cursor-pointer p-1.5 rounded-md text-neutral-500 hover:text-neutral-800
             dark:hover:text-neutral-200 hover:bg-neutral-100 dark:hover:bg-white/5
             transition-colors"
      onclick={() => audioDevicesStore.refresh()}
      aria-label={$t("settings.audio_devices_refresh")}
      title={$t("settings.audio_devices_refresh")}
    >
      <Icon
        icon="lucide:refresh-cw"
        width="14"
        class={$audioDevicesStore.loading ? "animate-spin" : ""}
      />
    </button>
  </div>

  {#if $audioDevicesStore.error}
    <div class="p-3 rounded-xl bg-red-500/10 border border-red-500/20 text-[11px] text-red-400">
      {$audioDevicesStore.error}
    </div>
  {:else if !$audioDevicesStore.loaded && $audioDevicesStore.loading}
    <div class="p-4 rounded-2xl bg-neutral-50/60 dark:bg-white/2 border border-neutral-200/70 dark:border-white/8
                text-[11px] text-neutral-500 flex items-center gap-2">
      <Icon icon="lucide:loader-2" width="14" class="animate-spin" />
      {$t("settings.audio_devices_loading")}
    </div>
  {:else if $audioDevicesStore.devices.length === 0}
    <div class="p-4 rounded-2xl bg-neutral-50/60 dark:bg-white/2 border border-neutral-200/70 dark:border-white/8
                text-[11px] text-neutral-500">
      {$t("settings.audio_devices_empty")}
    </div>
  {:else}
    <div class="rounded-2xl overflow-hidden
                border border-neutral-200/70 dark:border-white/8
                bg-white/70 dark:bg-white/2 backdrop-blur-sm
                shadow-sm shadow-black/4 dark:shadow-[0_18px_40px_-28px_rgba(0,0,0,0.9)]
                divide-y divide-neutral-200/60 dark:divide-white/5">
      {#each $audioDevicesStore.devices as device (device.displayName)}
        {@const maxRate = maxSampleRate(device.sampleRates)}
        {@const isActive = $audioDevicesStore.activeDisplayName === device.displayName}
        <div
          class="group relative flex items-stretch transition-colors
                 {isActive
                   ? 'bg-linear-to-r from-emerald-500/12 via-emerald-500/6 to-transparent'
                   : 'hover:bg-neutral-100/70 dark:hover:bg-white/4'}"
        >
          <!-- Liseré d'accent sur la sortie active -->
          {#if isActive}
            <span
              class="absolute left-0 top-1/2 -translate-y-1/2 h-8 w-[3px] rounded-r-full
                     bg-emerald-500 shadow-[0_0_12px_rgba(16,185,129,0.55)]"
            ></span>
          {/if}

          <!-- Zone principale : choisir cette sortie -->
          <button
            class="flex-1 min-w-0 flex items-center gap-3 pl-4 pr-3 py-3 text-left cursor-pointer"
            onclick={() => select(device)}
            title={$t("settings.audio_devices_select_hint")}
          >
            <!-- Radio -->
            <span
              class="shrink-0 w-4 h-4 rounded-full border-2 flex items-center justify-center
                     transition-all
                     {isActive
                       ? 'border-emerald-500'
                       : 'border-neutral-300 dark:border-neutral-600 group-hover:border-neutral-400 dark:group-hover:border-neutral-500'}"
            >
              {#if isActive}
                <span class="w-2 h-2 rounded-full bg-emerald-500 shadow-[0_0_8px_rgba(16,185,129,0.8)]"></span>
              {/if}
            </span>

            <!-- Tuile d'icône -->
            <span
              class="shrink-0 w-9 h-9 rounded-xl flex items-center justify-center transition-all
                     {isActive
                       ? 'bg-emerald-500/15 text-emerald-600 dark:text-emerald-400 ring-1 ring-emerald-500/25 shadow-[0_0_18px_-4px_rgba(16,185,129,0.5)]'
                       : 'bg-neutral-200/60 dark:bg-white/5 text-neutral-500 dark:text-neutral-400 ring-1 ring-transparent'}"
            >
              <Icon icon={device.isHires ? "lucide:zap" : "lucide:speaker"} width="17" />
            </span>

            <!-- Nom + qualificatifs -->
            <span class="flex-1 min-w-0 flex items-center gap-2">
              <span
                class="truncate text-sm
                       {isActive
                         ? 'font-medium text-neutral-900 dark:text-neutral-100'
                         : 'text-neutral-700 dark:text-neutral-300'}"
              >
                {device.displayName}
              </span>
              {#if device.isHires}
                <span class="shrink-0 text-[9px] px-1.5 py-0.5 rounded-md font-semibold uppercase tracking-wider
                             text-amber-600 dark:text-amber-300/90 ring-1 ring-amber-500/25 bg-amber-500/8">
                  Hi-Res
                </span>
              {/if}
              {#if device.isDefault && !isActive}
                <span class="shrink-0 text-[9px] px-1.5 py-0.5 rounded-md font-semibold uppercase tracking-wider
                             text-neutral-500 dark:text-neutral-400 ring-1 ring-neutral-300/60 dark:ring-white/10">
                  {$t("settings.audio_devices_default_badge")}
                </span>
              {/if}
            </span>

            <!-- Capacités -->
            <span class="shrink-0 flex items-center gap-1.5 text-[11px] tabular-nums
                         text-neutral-400 dark:text-neutral-500">
              {#if maxRate}
                <span>{formatSampleRate(maxRate)}</span>
                <span class="text-neutral-300 dark:text-neutral-700">·</span>
              {/if}
              <span>{device.maxChannels} ch</span>
            </span>
          </button>

          <!-- Détails de CE périphérique -->
          <button
            class="shrink-0 flex items-center justify-center pl-2 pr-3.5 cursor-pointer
                   text-neutral-400/70 dark:text-neutral-500/70
                   hover:text-neutral-800 dark:hover:text-neutral-100
                   group-hover:text-neutral-500 dark:group-hover:text-neutral-400
                   transition-colors"
            onclick={() => (detailsOf = device)}
            aria-label={$t("settings.audio_devices_details_action")}
            title={$t("settings.audio_devices_details_action")}
          >
            <Icon icon="lucide:sliders-horizontal" width="15" />
          </button>
        </div>
      {/each}
    </div>
  {/if}
</div>

{#if detailsOf}
  <AudioDeviceDetailsModal
    device={detailsOf}
    isActive={$audioDevicesStore.activeDisplayName === detailsOf.displayName}
    onClose={() => (detailsOf = null)}
    onSelect={(d) => select(d)}
  />
{/if}
