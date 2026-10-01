<script lang="ts">
  import Icon from "@iconify/svelte";
  import { fade, scale } from "svelte/transition";
  import { invoke } from "@tauri-apps/api/core";
  import { t, currentLocale } from "$lib/i18n";
  import { portal } from "$lib/helper/portal";
  import {
    formatSampleRate,
    type AudioDeviceInfo,
    type WasapiDeviceCapabilities,
  } from "$lib/stores/audio/audioDevices.store";
  import { onMount } from "svelte";

  let {
    device,
    isActive = false,
    onClose,
    onSelect,
  }: {
    device: AudioDeviceInfo;
    isActive?: boolean;
    onClose: () => void;
    onSelect?: (device: AudioDeviceInfo) => void;
  } = $props();

  let selecting = $state(false);
  let selectError = $state<string | null>(null);

  // ── Probing WASAPI par device (Windows uniquement) ──
  // Sur Windows, CPAL expose la table shared-mode uniforme pour tous les
  // endpoints ; le probe WASAPI direct renvoie les rates réellement acceptés
  // par le driver du DAC en mode Exclusive.
  let probing = $state(false);
  let probeError = $state<string | null>(null);
  let caps = $state<WasapiDeviceCapabilities | null>(null);

  onMount(async () => {
    if (!device.wasapiId) return;
    probing = true;
    try {
      caps = await invoke<WasapiDeviceCapabilities>(
        "wasapi_probe_device_capabilities",
        { deviceId: device.wasapiId },
      );
    } catch (e) {
      probeError = e instanceof Error ? e.message : String(e);
    } finally {
      probing = false;
    }
  });

  async function selectAsOutput() {
    selecting = true;
    selectError = null;
    try {
      await invoke("set_device", { deviceName: device.name });
      onSelect?.(device);
      onClose();
    } catch (e) {
      selectError = e instanceof Error ? e.message : String(e);
    } finally {
      selecting = false;
    }
  }

  function bitDepthTone(fmt: string): string {
    if (fmt.includes("32-bit float")) return "text-emerald-500 dark:text-emerald-400";
    if (fmt.includes("32-bit") || fmt.includes("24-bit"))
      return "text-emerald-500 dark:text-emerald-400";
    if (fmt.includes("16-bit")) return "text-sky-500 dark:text-sky-400";
    return "text-neutral-500 dark:text-neutral-400";
  }

  function rateTone(hz: number): string {
    if (hz >= 352_800) return "text-purple-500 dark:text-purple-400";
    if (hz >= 88_200) return "text-amber-500 dark:text-amber-400";
    return "text-neutral-500 dark:text-neutral-400";
  }

  function rateCategory(hz: number): string {
    if (hz >= 352_800) return $t("settings.audio_devices_rate_dsd_pcm");
    if (hz >= 88_200) return $t("settings.audio_devices_rate_hires");
    return $t("settings.audio_devices_rate_cd");
  }

  // Source de vérité : probing WASAPI si dispo et non-vide, sinon CPAL.
  // `usingProbe` = true quand on affiche des données WASAPI réelles.
  let usingProbe = $derived(
    caps !== null &&
      ((caps.exclusiveRates?.length ?? 0) > 0 ||
        (caps.exclusiveBitDepths?.length ?? 0) > 0),
  );
  let effectiveRates = $derived(
    usingProbe && caps ? caps.exclusiveRates : device.sampleRates,
  );
  let effectiveBitDepths = $derived(
    usingProbe && caps
      ? caps.exclusiveBitDepths.map((b) => `${b}-bit int`)
      : device.sampleFormats,
  );
  let ratesByCategory = $derived.by(() => {
    const groups = new Map<string, number[]>();
    for (const hz of effectiveRates) {
      const cat = rateCategory(hz);
      const arr = groups.get(cat) ?? [];
      arr.push(hz);
      groups.set(cat, arr);
    }
    return Array.from(groups.entries());
  });

  function handleKey(e: KeyboardEvent) {
    if (e.key === "Escape") onClose();
  }
</script>

<svelte:window onkeydown={handleKey} />

<div use:portal>
<div
  role="presentation"
  class="fixed inset-0 z-60 flex items-center justify-center
         bg-black/60 backdrop-blur-sm p-4"
  transition:fade={{ duration: 150 }}
  onclick={onClose}
>
  <div
    role="dialog"
    aria-labelledby="device-modal-title"
    aria-modal="true"
    tabindex="-1"
    class="w-full max-w-2xl max-h-[85vh] overflow-hidden flex flex-col
           rounded-2xl border shadow-2xl
           bg-white border-neutral-200
           dark:bg-neutral-900 dark:border-white/10"
    transition:scale={{ duration: 180, start: 0.96 }}
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.stopPropagation()}
  >
    <!-- Header -->
    <div class="px-6 pt-5 pb-4 border-b border-neutral-100 dark:border-white/5">
      <div class="flex items-start gap-3">
        <div
          class="shrink-0 w-11 h-11 rounded-xl flex items-center justify-center
                 {device.isDefault
                   ? 'bg-emerald-500/15 text-emerald-500 dark:text-emerald-400'
                   : 'bg-neutral-100 dark:bg-white/5 text-neutral-500 dark:text-neutral-400'}"
        >
          <Icon icon={device.isHires ? "lucide:zap" : "lucide:speaker"} width="22" />
        </div>
        <div class="flex-1 min-w-0">
          <h3
            id="device-modal-title"
            class="text-base font-semibold text-neutral-900 dark:text-white truncate"
          >
            {device.displayName}
          </h3>
          <div class="mt-1 flex items-center gap-1.5 flex-wrap">
            {#if device.isDefault}
              <span class="text-[9px] px-1.5 py-0.5 rounded font-semibold
                           bg-emerald-500/15 text-emerald-600 dark:text-emerald-400
                           uppercase tracking-wider">
                {$t("settings.audio_devices_default_badge")}
              </span>
            {/if}
            {#if device.isHires}
              <span class="text-[9px] px-1.5 py-0.5 rounded font-semibold
                           bg-amber-500/15 text-amber-600 dark:text-amber-300
                           uppercase tracking-wider">
                {$t("settings.audio_devices_rate_hires")}
              </span>
            {/if}
            {#if device.manufacturer}
              <span class="text-[11px] text-neutral-500 dark:text-neutral-400">
                {device.manufacturer}
              </span>
            {/if}
          </div>
        </div>
        <button
          class="shrink-0 p-1.5 rounded-lg cursor-pointer text-neutral-400 hover:text-neutral-800
                 dark:hover:text-neutral-200 hover:bg-neutral-100 dark:hover:bg-white/5 transition-colors"
          onclick={onClose}
          aria-label={$t("common.close")}
        >
          <Icon icon="lucide:x" width="16" />
        </button>
      </div>
    </div>

    <!-- Scroll body -->
    <div class="overflow-y-auto scrollbar-app px-6 py-5 space-y-6">
      <!-- Identité -->
      <section>
        <p class="text-[10px] font-semibold uppercase tracking-widest text-neutral-400 dark:text-neutral-500 mb-2">
          {$t("settings.audio_devices_identity")}
        </p>
        <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
          <div class="p-3 rounded-lg bg-neutral-50 dark:bg-white/3 border border-neutral-200/70 dark:border-white/5">
            <p class="text-[10px] uppercase tracking-wider text-neutral-400 dark:text-neutral-500 font-semibold mb-1">
              {$t("settings.audio_devices_raw_name")}
            </p>
            <p class="text-xs font-mono text-neutral-700 dark:text-neutral-300 break-all">
              {device.name}
            </p>
          </div>
          {#if device.manufacturer}
            <div class="p-3 rounded-lg bg-neutral-50 dark:bg-white/3 border border-neutral-200/70 dark:border-white/5">
              <p class="text-[10px] uppercase tracking-wider text-neutral-400 dark:text-neutral-500 font-semibold mb-1">
                {$t("settings.audio_devices_manufacturer")}
              </p>
              <p class="text-xs text-neutral-700 dark:text-neutral-300">{device.manufacturer}</p>
            </div>
          {/if}
          {#if device.driver}
            <div class="p-3 rounded-lg bg-neutral-50 dark:bg-white/3 border border-neutral-200/70 dark:border-white/5">
              <p class="text-[10px] uppercase tracking-wider text-neutral-400 dark:text-neutral-500 font-semibold mb-1">
                {$t("settings.audio_devices_driver")}
              </p>
              <p class="text-xs text-neutral-700 dark:text-neutral-300 truncate" title={device.driver}>
                {device.driver}
              </p>
            </div>
          {/if}
          <div class="p-3 rounded-lg bg-neutral-50 dark:bg-white/3 border border-neutral-200/70 dark:border-white/5">
            <p class="text-[10px] uppercase tracking-wider text-neutral-400 dark:text-neutral-500 font-semibold mb-1">
              {$t("settings.audio_devices_channels")}
            </p>
            <p class="text-xs text-neutral-700 dark:text-neutral-300">
              {device.maxChannels} ch{device.maxChannels >= 2 ? " (max)" : ""}
            </p>
          </div>
        </div>
      </section>

      <!-- Bannière source des données -->
      {#if probing}
        <div class="flex items-center gap-2 px-3 py-2 rounded-lg bg-sky-500/8 border border-sky-500/20 text-[11px] text-sky-500 dark:text-sky-400">
          <Icon icon="lucide:loader-2" width="13" class="animate-spin" />
          {$t("settings.audio_devices_probing")}
        </div>
      {:else if usingProbe}
        <div class="flex items-start gap-2 px-3 py-2 rounded-lg bg-emerald-500/8 border border-emerald-500/20 text-[11px] text-emerald-600 dark:text-emerald-400">
          <Icon icon="lucide:shield-check" width="13" class="mt-0.5 shrink-0" />
          <div>
            <p class="font-medium">{$t("settings.audio_devices_probed_title")}</p>
            <p class="text-[10px] opacity-80 mt-0.5">{$t("settings.audio_devices_probed_hint")}</p>
          </div>
        </div>
      {:else if caps}
        <div class="flex items-start gap-2 px-3 py-2 rounded-lg bg-amber-500/8 border border-amber-500/20 text-[11px] text-amber-600 dark:text-amber-400">
          <Icon icon="lucide:info" width="13" class="mt-0.5 shrink-0" />
          <div>
            <p class="font-medium">{$t("settings.audio_devices_no_exclusive_title")}</p>
            <p class="text-[10px] opacity-80 mt-0.5">{$t("settings.audio_devices_no_exclusive_hint")}</p>
          </div>
        </div>
      {:else if probeError}
        <div class="flex items-start gap-2 px-3 py-2 rounded-lg bg-amber-500/8 border border-amber-500/20 text-[11px] text-amber-600 dark:text-amber-400">
          <Icon icon="lucide:info" width="13" class="mt-0.5 shrink-0" />
          <div>
            <p class="font-medium">{$t("settings.audio_devices_probe_failed")}</p>
            <p class="text-[10px] opacity-80 mt-0.5">{probeError}</p>
          </div>
        </div>
      {:else if !device.wasapiId}
        <div class="flex items-start gap-2 px-3 py-2 rounded-lg bg-neutral-500/8 border border-neutral-500/20 text-[11px] text-neutral-500 dark:text-neutral-400">
          <Icon icon="lucide:info" width="13" class="mt-0.5 shrink-0" />
          <span>{$t("settings.audio_devices_cpal_only")}</span>
        </div>
      {/if}

      <!-- Mix format Windows (si probe dispo) -->
      {#if caps && caps.mixRate}
        <section>
          <p class="text-[10px] font-semibold uppercase tracking-widest text-neutral-400 dark:text-neutral-500 mb-2">
            {$t("settings.audio_devices_windows_mix")}
          </p>
          <div class="p-3 rounded-lg bg-neutral-50 dark:bg-white/3 border border-neutral-200/70 dark:border-white/5">
            <p class="text-xs text-neutral-700 dark:text-neutral-300 font-mono tabular-nums">
              {formatSampleRate(caps.mixRate)}
              {#if caps.mixBitDepth} · {caps.mixBitDepth}-bit{/if}
              {#if caps.mixChannels} · {caps.mixChannels} ch{/if}
            </p>
            <p class="text-[10px] text-neutral-500 dark:text-neutral-400 mt-1">
              {$t("settings.audio_devices_windows_mix_hint")}
            </p>
          </div>
        </section>
      {/if}

      <!-- Formats -->
      {#if effectiveBitDepths.length > 0}
        <section>
          <p class="text-[10px] font-semibold uppercase tracking-widest text-neutral-400 dark:text-neutral-500 mb-2">
            {usingProbe
              ? $t("settings.audio_devices_formats_exclusive")
              : $t("settings.audio_devices_formats")}
          </p>
          <div class="flex flex-wrap gap-1.5">
            {#each effectiveBitDepths as fmt}
              <span
                class="text-[11px] px-2 py-1 rounded-md font-mono
                       bg-neutral-100 dark:bg-white/5 border border-neutral-200 dark:border-white/10
                       {bitDepthTone(fmt)}"
              >
                {fmt}
              </span>
            {/each}
          </div>
        </section>
      {/if}

      <!-- Fréquences groupées par catégorie -->
      {#if effectiveRates.length > 0}
        <section>
          <p class="text-[10px] font-semibold uppercase tracking-widest text-neutral-400 dark:text-neutral-500 mb-2">
            {usingProbe
              ? $t("settings.audio_devices_sample_rates_exclusive")
              : $t("settings.audio_devices_sample_rates")}
          </p>
          <div class="space-y-2">
            {#each ratesByCategory as [category, rates]}
              <div>
                <p class="text-[10px] font-semibold text-neutral-500 dark:text-neutral-400 mb-1">
                  {category}
                </p>
                <div class="flex flex-wrap gap-1.5">
                  {#each rates as hz}
                    <span
                      class="text-[11px] px-2 py-1 rounded-md font-mono tabular-nums
                             bg-neutral-100 dark:bg-white/5 border border-neutral-200 dark:border-white/10
                             {rateTone(hz)}"
                    >
                      {formatSampleRate(hz)}
                    </span>
                  {/each}
                </div>
              </div>
            {/each}
          </div>
        </section>
      {/if}

      <!-- Buffer -->
      {#if device.minBufferFrames !== null && device.maxBufferFrames !== null}
        <section>
          <p class="text-[10px] font-semibold uppercase tracking-widest text-neutral-400 dark:text-neutral-500 mb-2">
            {$t("settings.audio_devices_buffer")}
          </p>
          <div class="p-3 rounded-lg bg-neutral-50 dark:bg-white/3 border border-neutral-200/70 dark:border-white/5">
            <p class="text-xs text-neutral-700 dark:text-neutral-300 font-mono tabular-nums">
              {device.minBufferFrames.toLocaleString($currentLocale)} – {device.maxBufferFrames.toLocaleString($currentLocale)} frames
            </p>
            <p class="text-[10px] text-neutral-500 dark:text-neutral-400 mt-1">
              {$t("settings.audio_devices_buffer_hint")}
            </p>
          </div>
        </section>
      {/if}

      {#if selectError}
        <div class="p-3 rounded-lg bg-red-500/10 border border-red-500/20 text-[11px] text-red-400">
          {selectError}
        </div>
      {/if}
    </div>

    <!-- Footer : sélection comme sortie active -->
    <div class="px-6 py-4 border-t border-neutral-100 dark:border-white/5 flex items-center justify-between gap-3">
      {#if isActive}
        <div class="flex items-center gap-1.5 text-[11px] text-emerald-600 dark:text-emerald-400 font-medium">
          <Icon icon="lucide:check-circle-2" width="14" />
          {$t("settings.audio_devices_active")}
        </div>
      {:else}
        <span class="text-[11px] text-neutral-500 dark:text-neutral-400">
          {$t("settings.audio_devices_select_hint")}
        </span>
      {/if}
      <div class="flex items-center gap-2">
        <button
          class="px-3 py-1.5 text-xs rounded-md cursor-pointer transition-colors
                 text-neutral-600 dark:text-neutral-300
                 hover:bg-neutral-100 dark:hover:bg-white/5"
          onclick={onClose}
        >
          {$t("common.close")}
        </button>
        {#if !isActive}
          <button
            class="px-3 py-1.5 text-xs rounded-md font-medium cursor-pointer transition-colors
                   bg-emerald-500 hover:bg-emerald-600 text-white
                   disabled:opacity-50 disabled:cursor-not-allowed
                   flex items-center gap-1.5"
            disabled={selecting}
            onclick={selectAsOutput}
          >
            {#if selecting}
              <Icon icon="lucide:loader-2" width="12" class="animate-spin" />
            {:else}
              <Icon icon="lucide:check" width="12" />
            {/if}
            {$t("settings.audio_devices_select_action")}
          </button>
        {/if}
      </div>
    </div>
  </div>
</div>
</div>
