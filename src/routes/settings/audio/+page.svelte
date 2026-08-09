<script lang="ts">
  import Icon from "@iconify/svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { settingsStore } from "$lib/stores/settings/settings.store";
  import { t } from "$lib/i18n";
  import { detectOS } from "$lib/helper/tools/osDetection";
  import AppearanceCard from "$lib/components/settings/AppearanceCard.svelte";
  import AudioDevicesCard from "$lib/components/settings/AudioDevicesCard.svelte";
  import ToggleSwitch from "$lib/components/ui/input/ToggleSwitch.svelte";
  import {
    getAudioQualityStatus,
    setAudioQualitySetting,
    type AudioQualityStatus,
    type AudioQualitySetting,
  } from "$lib/services/audio/audioQuality.service";
  import {
    getReplayGainSettings,
    setReplayGainSettings,
    type ReplayGainMode,
    type ReplayGainSettings,
  } from "$lib/services/audio/replayGain.service";

  let gapless = $derived($settingsStore.gapless !== 'false');
  let wasapiExclusive = $derived($settingsStore.wasapi_exclusive === 'true');
  let dsdDop = $derived($settingsStore.dsd_dop === 'true');
  let isWindows = $derived(detectOS() === 'windows');
  let isLinux = $derived(detectOS() === 'linux');
  let isMac = $derived(detectOS() === 'macos');

  // ─── Audio quality state ───
  let audioQuality = $state<AudioQualityStatus | null>(null);
  let audioQualitySaving = $state(false);

  // ─── Replay Gain state ───
  let replayGain = $state<ReplayGainSettings | null>(null);
  let replayGainSaving = $state(false);
  // Valeur affichée pendant le glissement du curseur : on ne persiste qu'au
  // relâchement (onchange) pour ne pas marteler la BDD.
  let preampDraft = $state(0);

  onMount(async () => {
    try {
      audioQuality = await getAudioQualityStatus();
    } catch (e) {
      console.error("[audio quality] init failed:", e);
    }
    try {
      replayGain = await getReplayGainSettings();
      preampDraft = replayGain.preamp_db;
    } catch (e) {
      console.error("[replay gain] init failed:", e);
    }
  });

  async function saveReplayGain(mode: ReplayGainMode, preampDb: number) {
    if (replayGainSaving) return;
    replayGainSaving = true;
    try {
      replayGain = await setReplayGainSettings(mode, preampDb);
      preampDraft = replayGain.preamp_db;
    } catch (e) {
      console.error("[replay gain] save failed:", e);
    } finally {
      replayGainSaving = false;
    }
  }

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
</script>

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
        <ToggleSwitch
          checked={wasapiExclusive}
          color="amber"
          label="WASAPI exclusive"
          onclick={() => settingsStore.toggle('wasapi_exclusive')}
        />
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
            <ToggleSwitch
              checked={dsdDop}
              color="purple"
              label="DSD natif DoP"
              onclick={() => settingsStore.toggle('dsd_dop')}
            />
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

  <!-- ── Sortie exclusive PCM (bit-perfect) — Linux & macOS ──
       Même réglage persisté que Windows (`wasapi_exclusive`) : c'est la
       plateforme qui choisit le backend concret (ALSA hw: / CoreAudio hog).
       Indépendant du DoP ci-dessous, qui a son propre chemin dédié. -->
  {#if isLinux || isMac}
    <div class="mt-8 mb-2 rounded-2xl border border-neutral-200/70 dark:border-white/8
                bg-neutral-50/40 dark:bg-white/2 overflow-hidden">
      <div class="flex items-center gap-3.5 px-4 py-3.5">
        <div class="shrink-0 w-9 h-9 rounded-lg flex items-center justify-center
                    {wasapiExclusive ? 'bg-amber-500/15 text-amber-500 dark:text-amber-400' : 'bg-neutral-200/60 dark:bg-white/5 text-neutral-400'}">
          <Icon icon="lucide:audio-lines" width="18" />
        </div>
        <div class="flex-1 min-w-0">
          <p class="text-sm font-medium text-neutral-800 dark:text-neutral-200 flex items-center gap-1.5">
            {$t('settings.exclusive_output')}
            <span class="text-[9px] font-bold px-1.5 py-0.5 rounded bg-amber-500/15 text-amber-500 border border-amber-500/20 uppercase tracking-wider">Beta</span>
          </p>
          <p class="text-[11px] text-neutral-400 dark:text-neutral-500 mt-0.5">
            {wasapiExclusive
              ? $t('settings.exclusive_output_on_hint')
              : isLinux
                ? $t('settings.exclusive_output_desc_linux')
                : $t('settings.exclusive_output_desc_macos')}
          </p>
        </div>
        <ToggleSwitch
          checked={wasapiExclusive}
          color="amber"
          label={$t('settings.exclusive_output')}
          onclick={() => settingsStore.toggle('wasapi_exclusive')}
        />
      </div>

      {#if wasapiExclusive}
        <div class="px-4 pb-3 -mt-1">
          <p class="flex items-start gap-1.5 text-[10.5px] text-amber-600/90 dark:text-amber-300/70 leading-relaxed">
            <Icon icon="lucide:info" width="12" class="shrink-0 mt-0.5" />
            <span>{$t('settings.exclusive_output_warning')}</span>
          </p>
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
        <ToggleSwitch
          checked={dsdDop}
          color="purple"
          label="DSD natif DoP"
          onclick={() => settingsStore.toggle('dsd_dop')}
        />
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

  <!-- ── Qualité de décodage ── -->
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

  <!-- ── Lecture sans blanc (gapless) ── -->
  <div class="mt-8 mb-2 rounded-2xl border border-neutral-200/70 dark:border-white/8
              bg-neutral-50/40 dark:bg-white/2 overflow-hidden">
    <div class="flex items-center gap-3.5 px-4 py-3.5">
      <div class="shrink-0 w-9 h-9 rounded-lg flex items-center justify-center
                  {gapless ? 'bg-emerald-500/15 text-emerald-600 dark:text-emerald-400' : 'bg-neutral-200/60 dark:bg-white/5 text-neutral-400'}">
        <Icon icon="lucide:link" width="18" />
      </div>
      <div class="flex-1 min-w-0">
        <p class="text-sm font-medium text-neutral-800 dark:text-neutral-200">
          {$t('settings.gapless')}
        </p>
        <p class="text-[11px] text-neutral-400 dark:text-neutral-500 mt-0.5">
          {$t('settings.gapless_desc')}
        </p>
      </div>
      <ToggleSwitch
        checked={gapless}
        color="emerald"
        label={$t('settings.gapless')}
        onclick={async () => {
          const next = !gapless;
          settingsStore.set('gapless', next ? 'true' : 'false');
          try {
            await invoke('set_gapless', { enabled: next });
          } catch (e) {
            console.warn('[gapless] set_gapless:', e);
          }
        }}
      />
    </div>

    {#if gapless}
      <div class="px-4 pb-3 -mt-1">
        <p class="flex items-start gap-1.5 text-[10.5px] text-neutral-500 dark:text-neutral-400 leading-relaxed">
          <Icon icon="lucide:info" width="12" class="shrink-0 mt-0.5" />
          <span>{$t('settings.gapless_note')}</span>
        </p>
      </div>
    {/if}
  </div>

  <!-- ── Replay Gain ── -->
  <div class="mb-2 mt-8">
    <div class="flex items-center gap-3 mb-2.5 px-1">
      <Icon icon="lucide:audio-waveform" width="18" class="text-neutral-400" />
      <div>
        <p class="text-sm font-medium text-neutral-800 dark:text-neutral-200">{$t('settings.replay_gain')}</p>
        <p class="text-[11px] text-neutral-400 dark:text-neutral-500">{$t('settings.replay_gain_desc')}</p>
      </div>
    </div>

    <div class="grid grid-cols-3 gap-2 max-w-lg">
      <!-- Désactivé : niveaux bruts, hétérogènes -->
      <AppearanceCard
        label={$t('settings.replay_gain_off')}
        selected={replayGain?.mode === 'off'}
        onclick={() => saveReplayGain('off', preampDraft)}
      >
        <div class="flex items-end gap-0.5 h-6">
          <div class="w-1.5 h-6 rounded-sm bg-neutral-400 dark:bg-neutral-600"></div>
          <div class="w-1.5 h-2 rounded-sm bg-neutral-400 dark:bg-neutral-600"></div>
          <div class="w-1.5 h-5 rounded-sm bg-neutral-400 dark:bg-neutral-600"></div>
          <div class="w-1.5 h-3 rounded-sm bg-neutral-400 dark:bg-neutral-600"></div>
        </div>
      </AppearanceCard>

      <!-- Par morceau : tout au même niveau -->
      <AppearanceCard
        label={$t('settings.replay_gain_track')}
        selected={replayGain?.mode === 'track'}
        onclick={() => saveReplayGain('track', preampDraft)}
      >
        <div class="flex items-end gap-0.5 h-6">
          <div class="w-1.5 h-4 rounded-sm bg-green-500"></div>
          <div class="w-1.5 h-4 rounded-sm bg-green-500"></div>
          <div class="w-1.5 h-4 rounded-sm bg-green-500"></div>
          <div class="w-1.5 h-4 rounded-sm bg-green-500"></div>
        </div>
      </AppearanceCard>

      <!-- Par album : dynamique interne préservée, albums alignés entre eux -->
      <AppearanceCard
        label={$t('settings.replay_gain_album')}
        selected={replayGain?.mode === 'album'}
        onclick={() => saveReplayGain('album', preampDraft)}
      >
        <div class="flex items-end gap-1.5 h-6">
          <div class="flex items-end gap-0.5">
            <div class="w-1.5 h-5 rounded-sm bg-green-500"></div>
            <div class="w-1.5 h-3 rounded-sm bg-green-500"></div>
          </div>
          <div class="flex items-end gap-0.5">
            <div class="w-1.5 h-5 rounded-sm bg-green-500"></div>
            <div class="w-1.5 h-3 rounded-sm bg-green-500"></div>
          </div>
        </div>
      </AppearanceCard>
    </div>

    {#if replayGain}
      <!-- Explication du mode retenu -->
      <div class="mt-3 px-3 py-2 rounded-lg text-[11px] text-neutral-500 dark:text-neutral-400
                  bg-neutral-50 dark:bg-white/2 border border-neutral-200/60 dark:border-white/5
                  flex items-start gap-2 max-w-2xl">
        <Icon icon="lucide:info" width={13} class="mt-0.5 shrink-0 text-neutral-400" />
        <div>
          {$t(`settings.replay_gain_mode_${replayGain.mode}_desc`)}
          {#if replayGain.mode !== 'off'}
            <span class="block mt-0.5 text-neutral-400 dark:text-neutral-500">
              {$t('settings.replay_gain_note')}
            </span>
          {/if}
        </div>
      </div>

      <!-- Pré-ampli : uniquement quand le Replay Gain est actif -->
      {#if replayGain.mode !== 'off'}
        <div class="mt-3 px-3 py-3 rounded-lg max-w-2xl
                    bg-neutral-50 dark:bg-white/2 border border-neutral-200/60 dark:border-white/5">
          <div class="flex items-center justify-between gap-4 mb-2">
            <div class="min-w-0">
              <p class="text-[12px] font-medium text-neutral-700 dark:text-neutral-300">
                {$t('settings.replay_gain_preamp')}
              </p>
              <p class="text-[11px] text-neutral-400 dark:text-neutral-500">
                {$t('settings.replay_gain_preamp_desc')}
              </p>
            </div>
            <span class="shrink-0 text-[11px] font-mono tabular-nums px-2 py-0.5 rounded-md
                         text-emerald-600 dark:text-emerald-400/90
                         ring-1 ring-emerald-500/25 bg-emerald-500/8">
              {preampDraft > 0 ? '+' : ''}{preampDraft.toFixed(1)} dB
            </span>
          </div>
          <input
            type="range"
            min="-15"
            max="15"
            step="0.5"
            value={preampDraft}
            disabled={replayGainSaving}
            oninput={(e) => (preampDraft = Number(e.currentTarget.value))}
            onchange={(e) => saveReplayGain(replayGain!.mode, Number(e.currentTarget.value))}
            class="w-full accent-emerald-500 cursor-pointer disabled:cursor-not-allowed"
            aria-label={$t('settings.replay_gain_preamp')}
          />
        </div>
      {/if}
    {/if}
  </div>
</section>
