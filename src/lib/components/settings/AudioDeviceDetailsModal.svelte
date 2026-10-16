<script lang="ts">
  import Icon from "@iconify/svelte";
  import { fade, scale } from "svelte/transition";
  import { invoke } from "@tauri-apps/api/core";
  import { playerService } from "#lib/services/player/player.service";
  import { t, currentLocale } from "#lib/i18n";
  import { portal } from "#lib/helper/portal";
  import Badge from "#lib/components/ui/text/Badge.svelte";
  import {
    formatSampleRate,
    type AudioDeviceInfo,
    type WasapiDeviceCapabilities,
  } from "#lib/stores/audio/audioDevices.store";
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
      await playerService.applyOutputChange(() => invoke("set_device", { deviceName: device.name }), { localOnly: true });
      onSelect?.(device);
      onClose();
    } catch (e) {
      selectError = e instanceof Error ? e.message : String(e);
    } finally {
      selecting = false;
    }
  }

  function bitDepthTone(fmt: string): string {
    if (fmt.includes("32-bit float")) return "text-(--rg-gtx)";
    if (fmt.includes("32-bit") || fmt.includes("24-bit"))
      return "text-(--rg-gtx)";
    if (fmt.includes("16-bit")) return "text-sky-700 dark:text-sky-400";
    return "text-(--rg-mu)";
  }

  function rateTone(hz: number): string {
    if (hz >= 352_800) return "text-purple-700 dark:text-purple-300";
    if (hz >= 88_200) return "text-(--rg-am)";
    return "text-(--rg-mu)";
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
         bg-black/60 backdrop-blur-sm"
  transition:fade={{ duration: 150 }}
  onclick={onClose}
>
  <div
    role="dialog"
    aria-labelledby="device-modal-title"
    aria-modal="true"
    tabindex="-1"
    class="flex flex-col w-full max-w-2xl mx-4 max-h-[88vh]
           rounded-2xl overflow-hidden text-(--rg-tx)
           bg-(--rg-carte) border border-(--rg-bd) dark:border-[#2a312d]
           shadow-[0_24px_60px_-12px_rgba(0,0,0,0.25)] dark:shadow-[0_30px_80px_-20px_rgba(0,0,0,0.9)]"
    transition:scale={{ duration: 180, start: 0.96 }}
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.stopPropagation()}
  >
    <!-- En-tête : même bandeau que les popins -->
    <div
      class="shrink-0 px-6 pt-5 pb-4 border-b border-(--rg-bd)
             bg-linear-135 from-(--rg-g)/8 to-transparent to-60%"
    >
      <div class="flex items-center justify-between gap-3">
        <h3
          id="device-modal-title"
          class="flex items-center gap-2.5 min-w-0
                 text-[17px] font-extrabold tracking-[-0.01em] text-(--rg-tx)"
        >
          <span
            class="shrink-0 w-8 h-8 rounded-[9px] flex items-center justify-center
                   bg-(--rg-gbg) text-(--rg-g) border border-(--rg-gbd)"
          >
            <Icon icon={device.isHires ? "lucide:zap" : "lucide:speaker"} width="15" />
          </span>
          <span class="truncate">{device.displayName}</span>
        </h3>
        <button
          type="button"
          class="shrink-0 -mr-1.5 w-8 h-8 rounded-[9px] flex items-center justify-center
                 cursor-pointer text-(--rg-mu) hover:text-(--rg-tx) hover:bg-(--rg-hover)
                 transition-colors"
          onclick={onClose}
          aria-label={$t("common.close")}
        >
          <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round">
            <path d="M18 6 6 18M6 6l12 12" />
          </svg>
        </button>
      </div>
      {#if device.isDefault || device.isHires || device.manufacturer}
        <div class="mt-2 pl-10.5 flex items-center gap-1.5 flex-wrap">
          {#if device.isDefault}
            <Badge tone="green">{$t("settings.audio_devices_default_badge")}</Badge>
          {/if}
          {#if device.isHires}
            <Badge tone="amber">{$t("settings.audio_devices_rate_hires")}</Badge>
          {/if}
          {#if device.manufacturer}
            <span class="text-[12px] text-(--rg-mu)">{device.manufacturer}</span>
          {/if}
        </div>
      {/if}
    </div>

    <!-- Corps défilant -->
    <div class="flex-1 min-h-0 overflow-y-auto scrollbar-app px-6 py-5 space-y-6">
      <!-- Identité -->
      <section>
        <p class="mb-2 text-[11px] font-bold uppercase tracking-[0.1em] text-(--rg-mu)">
          {$t("settings.audio_devices_identity")}
        </p>
        <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
          <div class="p-3.5 rounded-xl bg-(--rg-creux) border border-(--rg-bd)">
            <p class="mb-1 text-[10.5px] font-bold uppercase tracking-[0.08em] text-(--rg-mu2)">
              {$t("settings.audio_devices_raw_name")}
            </p>
            <p class="text-xs font-mono text-(--rg-tx2) break-all">
              {device.name}
            </p>
          </div>
          {#if device.manufacturer}
            <div class="p-3.5 rounded-xl bg-(--rg-creux) border border-(--rg-bd)">
              <p class="mb-1 text-[10.5px] font-bold uppercase tracking-[0.08em] text-(--rg-mu2)">
                {$t("settings.audio_devices_manufacturer")}
              </p>
              <p class="text-[13px] text-(--rg-tx)">{device.manufacturer}</p>
            </div>
          {/if}
          {#if device.driver}
            <div class="p-3.5 rounded-xl bg-(--rg-creux) border border-(--rg-bd)">
              <p class="mb-1 text-[10.5px] font-bold uppercase tracking-[0.08em] text-(--rg-mu2)">
                {$t("settings.audio_devices_driver")}
              </p>
              <p class="text-[13px] text-(--rg-tx) truncate" title={device.driver}>
                {device.driver}
              </p>
            </div>
          {/if}
          <div class="p-3.5 rounded-xl bg-(--rg-creux) border border-(--rg-bd)">
            <p class="mb-1 text-[10.5px] font-bold uppercase tracking-[0.08em] text-(--rg-mu2)">
              {$t("settings.audio_devices_channels")}
            </p>
            <p class="text-[13px] text-(--rg-tx)">
              {device.maxChannels} ch{device.maxChannels >= 2 ? " (max)" : ""}
            </p>
          </div>
        </div>
      </section>

      <!-- Bannière source des données -->
      {#if probing}
        <div class="flex items-center gap-2.5 px-3.5 py-3 rounded-xl border text-[13px]
                    bg-(--rg-creux) border-(--rg-bd) text-(--rg-mu)">
          <Icon icon="lucide:loader-2" width="16" class="shrink-0 animate-spin" />
          {$t("settings.audio_devices_probing")}
        </div>
      {:else if usingProbe}
        <div class="flex gap-2.5 px-3.5 py-3 rounded-xl border text-[13px] leading-normal
                    bg-(--rg-gbg) border-(--rg-gbd) text-(--rg-gtx)">
          <Icon icon="lucide:shield-check" width="16" class="mt-0.5 shrink-0 text-(--rg-g)" />
          <div>
            <p class="font-semibold">{$t("settings.audio_devices_probed_title")}</p>
            <p class="mt-0.5 text-[12px] opacity-80">{$t("settings.audio_devices_probed_hint")}</p>
          </div>
        </div>
      {:else if caps}
        <div class="flex gap-2.5 px-3.5 py-3 rounded-xl border text-[13px] leading-normal
                    bg-(--rg-ambg) border-(--rg-ambd) text-(--rg-amtx)">
          <Icon icon="lucide:info" width="16" class="mt-0.5 shrink-0 text-(--rg-am)" />
          <div>
            <p class="font-semibold">{$t("settings.audio_devices_no_exclusive_title")}</p>
            <p class="mt-0.5 text-[12px] opacity-80">{$t("settings.audio_devices_no_exclusive_hint")}</p>
          </div>
        </div>
      {:else if probeError}
        <div class="flex gap-2.5 px-3.5 py-3 rounded-xl border text-[13px] leading-normal
                    bg-(--rg-ambg) border-(--rg-ambd) text-(--rg-amtx)">
          <Icon icon="lucide:info" width="16" class="mt-0.5 shrink-0 text-(--rg-am)" />
          <div>
            <p class="font-semibold">{$t("settings.audio_devices_probe_failed")}</p>
            <p class="mt-0.5 text-[12px] opacity-80">{probeError}</p>
          </div>
        </div>
      {:else if !device.wasapiId}
        <div class="flex gap-2.5 px-3.5 py-3 rounded-xl border text-[13px] leading-normal
                    bg-(--rg-creux) border-(--rg-bd) text-(--rg-mu)">
          <Icon icon="lucide:info" width="16" class="mt-0.5 shrink-0" />
          <span>{$t("settings.audio_devices_cpal_only")}</span>
        </div>
      {/if}

      <!-- Mix format Windows (si probe dispo) -->
      {#if caps && caps.mixRate}
        <section>
          <p class="mb-2 text-[11px] font-bold uppercase tracking-[0.1em] text-(--rg-mu)">
            {$t("settings.audio_devices_windows_mix")}
          </p>
          <div class="p-3.5 rounded-xl bg-(--rg-creux) border border-(--rg-bd)">
            <p class="text-[13px] font-mono tabular-nums text-(--rg-tx)">
              {formatSampleRate(caps.mixRate)}
              {#if caps.mixBitDepth} · {caps.mixBitDepth}-bit{/if}
              {#if caps.mixChannels} · {caps.mixChannels} ch{/if}
            </p>
            <p class="mt-1 text-[12px] text-(--rg-mu)">
              {$t("settings.audio_devices_windows_mix_hint")}
            </p>
          </div>
        </section>
      {/if}

      <!-- Formats -->
      {#if effectiveBitDepths.length > 0}
        <section>
          <p class="mb-2 text-[11px] font-bold uppercase tracking-[0.1em] text-(--rg-mu)">
            {usingProbe
              ? $t("settings.audio_devices_formats_exclusive")
              : $t("settings.audio_devices_formats")}
          </p>
          <div class="flex flex-wrap gap-1.5">
            {#each effectiveBitDepths as fmt}
              <span
                class="px-2 py-1 rounded-md border font-mono text-xs font-semibold
                       bg-(--rg-creux) border-(--rg-bd)
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
          <p class="mb-2 text-[11px] font-bold uppercase tracking-[0.1em] text-(--rg-mu)">
            {usingProbe
              ? $t("settings.audio_devices_sample_rates_exclusive")
              : $t("settings.audio_devices_sample_rates")}
          </p>
          <div class="space-y-2.5">
            {#each ratesByCategory as [category, rates]}
              <div>
                <p class="mb-1.5 text-[12px] font-semibold text-(--rg-tx2)">
                  {category}
                </p>
                <div class="flex flex-wrap gap-1.5">
                  {#each rates as hz}
                    <span
                      class="px-2 py-1 rounded-md border font-mono text-xs font-semibold tabular-nums
                             bg-(--rg-creux) border-(--rg-bd)
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
          <p class="mb-2 text-[11px] font-bold uppercase tracking-[0.1em] text-(--rg-mu)">
            {$t("settings.audio_devices_buffer")}
          </p>
          <div class="p-3.5 rounded-xl bg-(--rg-creux) border border-(--rg-bd)">
            <p class="text-[13px] font-mono tabular-nums text-(--rg-tx)">
              {device.minBufferFrames.toLocaleString($currentLocale)} – {device.maxBufferFrames.toLocaleString($currentLocale)} frames
            </p>
            <p class="mt-1 text-[12px] text-(--rg-mu)">
              {$t("settings.audio_devices_buffer_hint")}
            </p>
          </div>
        </section>
      {/if}

      {#if selectError}
        <p class="px-4 py-3 rounded-xl border border-red-500/25 bg-red-500/8 text-[13px] text-red-600 dark:text-red-400">
          {selectError}
        </p>
      {/if}
    </div>

    <!-- Pied : sélection comme sortie active -->
    <footer class="shrink-0 flex items-center justify-between gap-3 px-6 py-3.5 border-t border-(--rg-bd) bg-(--rg-creux)">
      {#if isActive}
        <div class="flex items-center gap-1.5 text-[13px] font-semibold text-(--rg-gtx)">
          <Icon icon="lucide:check-circle-2" width="16" class="text-(--rg-g)" />
          {$t("settings.audio_devices_active")}
        </div>
      {:else}
        <span class="min-w-0 text-[12.5px] text-(--rg-mu)">
          {$t("settings.audio_devices_select_hint")}
        </span>
      {/if}
      <div class="shrink-0 flex items-center gap-2">
        <button
          class="h-10 px-4 rounded-xl text-sm font-semibold cursor-pointer transition-colors
                 text-(--rg-tx2) hover:bg-(--rg-hover) hover:text-(--rg-tx)"
          onclick={onClose}
        >
          {$t("common.close")}
        </button>
        {#if !isActive}
          <button
            class="h-10 pl-4 pr-5 rounded-xl text-sm font-bold cursor-pointer flex items-center gap-2 transition
                   bg-(--rg-g) text-(--rg-on-g) hover:brightness-110
                   disabled:opacity-40 disabled:cursor-not-allowed"
            disabled={selecting}
            onclick={selectAsOutput}
          >
            {#if selecting}
              <Icon icon="lucide:loader-2" width="17" class="animate-spin" />
            {:else}
              <Icon icon="lucide:check" width="17" />
            {/if}
            {$t("settings.audio_devices_select_action")}
          </button>
        {/if}
      </div>
    </footer>
  </div>
</div>
</div>
