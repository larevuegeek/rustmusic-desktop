<script lang="ts">
  import Icon from "@iconify/svelte";
  import { onMount } from "svelte";
  import { settingsStore } from "$lib/stores/settings/settings.store";
  import { audioDevicesStore } from "$lib/stores/audio/audioDevices.store";
  import { t, currentLocale } from "$lib/i18n";
  import { playbackPipelineStore } from "$lib/stores/player/playbackPipeline.store";
  import { messageRepli, frequenceLisible } from "$lib/helper/audio/chaineAudio";
  import { detectOS } from "$lib/helper/tools/osDetection";
  import OptionGroup from "$lib/components/ui/input/OptionGroup.svelte";
  import OptionItem from "$lib/components/ui/input/OptionItem.svelte";
  import OptionBlock from "$lib/components/ui/input/OptionBlock.svelte";
  import ChoiceTile from "$lib/components/ui/input/ChoiceTile.svelte";
  import SegmentedPanel from "$lib/components/ui/input/SegmentedPanel.svelte";
  import ToggleSwitch from "$lib/components/ui/input/ToggleSwitch.svelte";
  import GhostButton from "$lib/components/ui/button/GhostButton.svelte";
  import Badge from "$lib/components/ui/text/Badge.svelte";
  import AudioOutputList from "$lib/components/settings/AudioOutputList.svelte";
  import DacTest from "$lib/components/settings/DacTest.svelte";
  import { decrireSortie } from "$lib/helper/audio/deviceLabel";
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

  const os = detectOS();
  const surWindows = os === "windows";
  const plateforme = os === "macos" ? "macos" : os === "linux" ? "linux" : "windows";

  // ─── Sortie et mode ───
  const actif = $derived($audioDevicesStore.devices.find((d) => d.displayName === $audioDevicesStore.activeDisplayName) ?? null);
  const exclusif = $derived($settingsStore.wasapi_exclusive === "true");
  const dop = $derived($settingsStore.dsd_dop === "true");
  // Sous Windows, le DoP passe par WASAPI exclusif ; ailleurs il a son propre chemin.
  const dopDisponible = $derived(!surWindows || exclusif);
  // Ce qui n'a pas marché au dernier morceau : exclusif ou DoP refusés.
  const repli = $derived(
    $playbackPipelineStore && (exclusif || dop)
      ? messageRepli($playbackPipelineStore, $t, (hz) => frequenceLisible(hz, $currentLocale))
      : null,
  );

  // ─── Qualité de décodage ───
  let audioQuality = $state<AudioQualityStatus | null>(null);
  let audioQualitySaving = $state(false);

  // ─── Replay Gain ───
  let replayGain = $state<ReplayGainSettings | null>(null);
  let replayGainSaving = $state(false);
  // Affiché pendant le glissement ; persisté au relâchement seulement.
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

  // Code couleur de l'échelle : « Auto » prend celle du profil qu'il a retenu.
  const TONS = { high: "green", medium: "amber", low: "sky", minimal: "rose" } as const;
  const TEXTES_TONS = {
    green: "text-green-600 dark:text-green-400",
    amber: "text-amber-600 dark:text-amber-400",
    sky: "text-sky-600 dark:text-sky-400",
    rose: "text-rose-600 dark:text-rose-400",
  };
  const tonRetenu = $derived(audioQuality ? TONS[audioQuality.resolved as keyof typeof TONS] : undefined);

  const qualites = $derived([
    { value: "auto", label: $t("settings.audio_quality_auto"), icon: "material-symbols:check-circle-outline-rounded", tone: tonRetenu },
    { value: "high", label: $t("settings.audio_quality_high_short"), level: 4, tone: TONS.high },
    { value: "medium", label: $t("settings.audio_quality_medium_short"), level: 3, tone: TONS.medium },
    { value: "low", label: $t("settings.audio_quality_low_short"), level: 2, tone: TONS.low },
    { value: "minimal", label: $t("settings.audio_quality_minimal_short"), level: 1, tone: TONS.minimal },
  ]);

  /** Sous le choix : le profil retenu par « Auto », ou ce que fait le profil choisi. */
  const noteQualite = $derived.by(() => {
    if (!audioQuality) return null;
    if (audioQuality.setting === "auto") {
      const hote = audioQuality.virt_kind
        ? $t("settings.audio_quality_host_vm_short").replace("{kind}", audioQuality.virt_kind)
        : $t("settings.audio_quality_host_native_short");
      return {
        texte: $t("settings.audio_quality_now"),
        gras: $t(`settings.audio_quality_${audioQuality.resolved}`),
        conseil: $t("settings.audio_quality_auto_reason").replace("{n}", String(audioQuality.cpu_cores)).replace("{host}", hote),
      };
    }
    return {
      texte: $t(`settings.audio_quality_${audioQuality.setting}_summary`),
      conseil: $t(`settings.audio_quality_${audioQuality.setting}_advice`),
    };
  });

  const modesReplayGain = $derived([
    { value: "off", label: $t("settings.replay_gain_off"), icon: "material-symbols:do-not-disturb-on-outline-rounded" },
    { value: "track", label: $t("settings.replay_gain_track"), icon: "material-symbols:music-note-rounded" },
    { value: "album", label: $t("settings.replay_gain_album"), icon: "material-symbols:album-outline-rounded" },
  ]);
</script>

<!-- Explication sous un choix ; `{profile}` dans le texte passe en gras. -->
{#snippet note(texte: string, conseil?: string, gras?: string, tonGras?: string)}
  {@const [avant, apres] = texte.split("{profile}")}
  <div class="flex items-start gap-3 px-5 pt-3.5 pb-4 text-sm leading-normal text-(--rg-tx2)">
    <Icon icon="material-symbols:info-outline-rounded" width="18" class="shrink-0 mt-0.5 text-(--rg-mu)" />
    <div class="min-w-0">
      {avant}{#if gras}<b class="font-semibold {tonGras ?? 'text-(--rg-tx)'}">{gras}</b>{apres}{/if}
      {#if conseil}<small class="block text-[13px] text-(--rg-mu)">{conseil}</small>{/if}
    </div>
  </div>
{/snippet}

<!-- ─── Sortie ─── -->
<OptionGroup title={$t("settings.output_group")} hint={$t("settings.output_group_hint")}>
  {#snippet action()}
    <GhostButton
      icon="material-symbols:sync-rounded"
      label={$t("settings.audio_devices_refresh")}
      title={$t("settings.audio_devices_refresh_hint")}
      busy={$audioDevicesStore.loading}
      onclick={() => audioDevicesStore.refresh()}
    />
  {/snippet}
  <OptionBlock keywords="{$t('settings.output_group')} DAC {$audioDevicesStore.devices.map((d) => decrireSortie(d).nom).join(' ')}">
    <AudioOutputList />
  </OptionBlock>
</OptionGroup>

<!-- ─── Mode de sortie ─── -->
<OptionGroup title={$t("settings.output_mode")} hint={$t("settings.output_mode_hint")}>
  {#snippet action()}<Badge tone="amber">{$t("settings.beta_badge")}</Badge>{/snippet}
  <OptionBlock keywords="{$t('settings.output_mode')} {$t('settings.output_shared')} {$t('settings.output_exclusive')} wasapi alsa bit-perfect" class="flex flex-col gap-4 p-4">
    <div class="grid grid-cols-2 max-sm:grid-cols-1 gap-3" role="radiogroup" aria-label={$t("settings.output_mode")}>
      <ChoiceTile
        title={$t("settings.output_shared")}
        desc={$t(`settings.output_shared_desc_${plateforme}`)}
        selected={!exclusif}
        onclick={() => settingsStore.set("wasapi_exclusive", "false")}
      />
      <ChoiceTile
        title={$t("settings.output_exclusive")}
        desc={$t(`settings.output_exclusive_desc_${plateforme}`)}
        selected={exclusif}
        onclick={() => settingsStore.set("wasapi_exclusive", "true")}
      />
    </div>
    <a href="/settings/guide" class="self-start flex items-center gap-1.5 px-1 text-[13px] font-semibold text-(--rg-gtx) hover:underline">
      <Icon icon="material-symbols:school-outline-rounded" width="17" />{$t("settings.output_mode_learn")}<span aria-hidden="true">→</span>
    </a>
    {#if exclusif}
      <div class="flex gap-2.5 px-3.5 py-3 rounded-[10px] border text-[13px] leading-normal
                  bg-(--rg-ambg) border-(--rg-ambd) text-(--rg-amtx)">
        <Icon icon="material-symbols:warning-outline-rounded" width="18" class="shrink-0 text-(--rg-am)" />
        <span>{$t("settings.output_exclusive_warning")}</span>
      </div>
    {/if}
    {#if repli}
      <div class="flex gap-2.5 px-3.5 py-3 rounded-[10px] border text-[13px] leading-normal
                  bg-(--rg-ambg) border-(--rg-ambd) text-(--rg-amtx)" role="status">
        <Icon icon="material-symbols:error-outline-rounded" width="18" class="shrink-0 text-(--rg-am)" />
        <span><b class="font-semibold">{$t("settings.output_last_track")}</b> {repli}</span>
      </div>
    {/if}
  </OptionBlock>
  <OptionItem
    title={$t("settings.dsd_dop")}
    desc={dopDisponible ? $t("settings.dsd_dop_desc") : `${$t("settings.dsd_dop_desc")} ${$t("settings.dsd_dop_needs_exclusive")}`}
    keywords="dsd dop"
    disabled={!dopDisponible}
    onclick={() => settingsStore.toggle("dsd_dop")}
  >
    <ToggleSwitch checked={dop} disabled={!dopDisponible} label={$t("settings.dsd_dop")} onclick={() => settingsStore.toggle("dsd_dop")} />
  </OptionItem>
  {#if surWindows}
    <OptionBlock keywords="{$t('settings.dac_test')} wasapi">
      <DacTest device={actif} />
    </OptionBlock>
  {/if}
</OptionGroup>

<!-- ─── Qualité de décodage ─── -->
<OptionGroup title={$t("settings.audio_quality")} hint={$t("settings.audio_quality_hint")}>
  <OptionBlock keywords="{$t('settings.audio_quality')} {qualites.map((q) => q.label).join(' ')} dsd cpu">
    <div class="px-4 pt-4">
      <SegmentedPanel
        value={audioQuality?.setting ?? null}
        options={qualites}
        label={$t("settings.audio_quality")}
        disabled={audioQualitySaving}
        onchange={(v) => handleAudioQualityChange(v as AudioQualitySetting)}
      />
    </div>
    {#if noteQualite}{@render note(noteQualite.texte, noteQualite.conseil, noteQualite.gras, tonRetenu && TEXTES_TONS[tonRetenu])}{/if}
  </OptionBlock>
</OptionGroup>

<!-- ─── Replay Gain ─── -->
<OptionGroup title={$t("settings.replay_gain")} hint={$t("settings.replay_gain_hint")}>
  <OptionBlock keywords="{$t('settings.replay_gain')} {modesReplayGain.map((m) => m.label).join(' ')} volume">
    <div class="px-4 pt-4">
      <SegmentedPanel
        value={replayGain?.mode ?? null}
        options={modesReplayGain}
        label={$t("settings.replay_gain")}
        disabled={replayGainSaving}
        onchange={(v) => saveReplayGain(v as ReplayGainMode, preampDraft)}
      />
    </div>
    {#if replayGain}
      {@render note(
        $t(`settings.replay_gain_${replayGain.mode}_summary`),
        replayGain.mode === "off" ? undefined : `${$t(`settings.replay_gain_${replayGain.mode}_advice`)} ${$t("settings.replay_gain_note")}`,
      )}
    {/if}
  </OptionBlock>
  {#if replayGain && replayGain.mode !== "off"}
    <OptionItem title={$t("settings.replay_gain_preamp")} desc={$t("settings.replay_gain_preamp_desc")} keywords="preamp gain db">
      <div class="w-56 max-sm:w-40 flex items-center gap-3">
        <input
          type="range"
          min="-15"
          max="15"
          step="0.5"
          value={preampDraft}
          disabled={replayGainSaving}
          oninput={(e) => (preampDraft = Number(e.currentTarget.value))}
          onchange={(e) => saveReplayGain(replayGain!.mode, Number(e.currentTarget.value))}
          class="flex-1 min-w-0 accent-(--rg-g) cursor-pointer disabled:cursor-wait"
          aria-label={$t("settings.replay_gain_preamp")}
        />
        <span class="shrink-0 w-16 text-right font-mono text-xs tabular-nums text-(--rg-gtx)">
          {preampDraft > 0 ? "+" : ""}{preampDraft.toFixed(1)} dB
        </span>
      </div>
    </OptionItem>
  {/if}
</OptionGroup>
