<script lang="ts">
  // Sortie, sourdine et volume du lecteur. En DSD natif (DoP), le volume se règle sur le DAC.
  import { invoke } from "@tauri-apps/api/core";
  import { playerService } from "#lib/services/player/player.service";
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import Icon from "@iconify/svelte";
  import { audioDevicesStore, type AudioDeviceInfo } from "#lib/stores/audio/audioDevices.store";
  import AudioDeviceDetailsModal from "#lib/components/settings/AudioDeviceDetailsModal.svelte";
  import ToggleSwitch from "#lib/components/ui/input/ToggleSwitch.svelte";
  import { settingsStore } from "#lib/stores/settings/settings.store";
  import { detectOS } from "#lib/helper/tools/osDetection";
  import { capacitesSortie, decrireSortie, palierQualite, PALIERS } from "#lib/helper/audio/deviceLabel";
  import Badge from "#lib/components/ui/text/Badge.svelte";
  import { t } from "#lib/i18n";
  import { playbackPipelineStore } from "#lib/stores/player/playbackPipeline.store";
  import { volume, chargerVolume, reglerVolume } from "#lib/stores/player/volume.store";
  import { networkIcon, networkPlayerStore, type NetworkRenderer } from "#lib/stores/audio/networkPlayer.store";

  const surWindows = detectOS() === "windows";
  const wasapiExclusive = $derived($settingsStore.wasapi_exclusive === "true");
  const dop = $derived($settingsStore.dsd_dop === "true");
  // Sous Windows, le DoP passe par WASAPI exclusif.
  const dopDisponible = $derived(!surWindows || wasapiExclusive);
  // Un backend DoP par OS (« WASAPI DoP », « ALSA DoP », « CoreAudio DoP »).
  const dopActive = $derived($playbackPipelineStore?.backend?.endsWith("DoP") === true);

  let value = $state(80);
  let previousValue = $state(80);
  // Le store fait foi : les raccourcis clavier le changent aussi.
  $effect(() => {
    value = $volume;
  });
  const isMuted = $derived(value === 0);

  let showDevices = $state(false);
  let menuEl: HTMLElement | null = $state(null);
  let modalDevice: AudioDeviceInfo | null = $state(null);

  const devices = $derived($audioDevicesStore.devices);
  const active = $derived(devices.find((d) => d.displayName === $audioDevicesStore.activeDisplayName) ?? null);
  // Lecteur réseau choisi : c'est lui la sortie, les réglages locaux ne s'appliquent pas.
  const distant = $derived($networkPlayerStore.selected);

  // Désactivés par défaut : sans le réglage, rien ne part sur le réseau.
  const reseauActif = $derived($settingsStore.network_players === "true" || $settingsStore.airplay === "true");

  // Première ouverture du menu : on cherche les lecteurs réseau.
  $effect(() => {
    if (reseauActif && showDevices && !$networkPlayerStore.searched && !$networkPlayerStore.searching) networkPlayerStore.discover();
  });

  onMount(async () => {
    await chargerVolume();
    previousValue = $volume;
    audioDevicesStore.ensureLoaded();
  });

  function handleChange() {
    reglerVolume(value);
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

  async function selectDevice(device: AudioDeviceInfo) {
    audioDevicesStore.setActive(device.displayName);
    showDevices = false;
    try {
      // Depuis un lecteur réseau : le périphérique d'abord, la reprise locale part dessus.
      if (distant) {
        await invoke("set_device", { deviceName: device.name });
        await networkPlayerStore.select(null);
      } else {
        await playerService.applyOutputChange(() => invoke("set_device", { deviceName: device.name }));
      }
    } catch (e) {
      console.error("Failed to set device", e);
    }
  }

  async function selectRenderer(r: NetworkRenderer) {
    showDevices = false;
    if (distant?.id === r.id) return;
    try {
      await networkPlayerStore.select(r.id);
    } catch (e) {
      console.error("[réseau] choix du lecteur :", e);
    }
  }

  // Fermeture au clic extérieur.
  $effect(() => {
    if (!showDevices) return;
    const dehors = (e: MouseEvent) => {
      if (menuEl && !menuEl.contains(e.target as Node)) showDevices = false;
    };
    const timer = setTimeout(() => document.addEventListener("click", dehors), 50);
    return () => {
      clearTimeout(timer);
      document.removeEventListener("click", dehors);
    };
  });

  // Palier de qualité réel (exclusif sondé, sinon mixeur) : badge coloré comme dans les réglages.
  function palier(d: AudioDeviceInfo) {
    const { taux, bits } = capacitesSortie(d, $audioDevicesStore.exclusive[d.displayName]);
    const p = palierQualite(taux, bits);
    return p ? PALIERS[p] : null;
  }

  const bouton = "w-9.5 h-9.5 shrink-0 flex items-center justify-center rounded-full cursor-pointer transition-colors";
</script>

<!-- Sans `relative` : le menu se cale sur le bord haut de la carte du lecteur. -->
<div bind:this={menuEl}>
  <button
    type="button"
    class="{bouton} text-(--lc-ic) hover:text-(--lc-ic-fort) hover:bg-(--lc-survol) {showDevices ? 'text-(--lc-acc)' : ''}"
    onclick={() => (showDevices = !showDevices)}
    title={distant ? `${$t("player.output")} : ${distant.name}` : active ? `${$t("player.output")} : ${decrireSortie(active).nom}` : $t("player.output")}
    aria-label={$t("player.output")}
    aria-expanded={showDevices}
  >
    <Icon icon={distant ? networkIcon(distant) : "material-symbols-light:speaker-outline-rounded"} width="22" />
  </button>

  {#if showDevices}
    <div
      class="absolute bottom-full right-2 mb-2 z-50 w-100 p-1.5 rounded-[14px] border
             bg-(--lc-menu) border-(--lc-menu-bd) shadow-[0_18px_40px_rgba(0,0,0,0.35)] text-(--lc-tx)"
    >
      <p class="px-2.5 pt-1.5 pb-2 text-[11px] font-bold uppercase tracking-[0.08em] text-(--lc-mu)">{$t("player.output")}</p>
      {#each devices as device (device.displayName)}
        {@const choisi = !distant && device.displayName === $audioDevicesStore.activeDisplayName}
        {@const libelle = decrireSortie(device)}
        {@const qualite = palier(device)}
        <!-- Sortie choisie : le vert des réglages, quelle que soit la pochette. -->
        <div class="relative flex items-stretch rounded-[10px] transition-colors {choisi ? 'bg-(--rg-g)/12' : 'hover:bg-(--lc-survol)'}">
          {#if choisi}<span class="absolute left-0 top-2.5 bottom-2.5 w-0.75 rounded-r-[3px] bg-(--rg-g)"></span>{/if}
          <button type="button" class="flex-1 min-w-0 flex items-center gap-2.5 px-2.5 py-2 text-left cursor-pointer" onclick={() => selectDevice(device)}>
            <Icon icon={libelle.icone} width="20" class="shrink-0 {choisi ? 'text-(--rg-g)' : 'text-(--lc-mu)'}" />
            <span class="min-w-0 flex-1 leading-[1.2]">
              <span class="block truncate text-[13px] font-semibold">{libelle.nom}</span>
              {#if libelle.detail}<span class="block truncate text-[11px] text-(--lc-mu)">{libelle.detail}</span>{/if}
            </span>
            {#if qualite}<Badge tone={qualite.ton}>{$t(qualite.labelKey)}</Badge>{/if}
            <Icon icon="material-symbols-light:check-rounded" width="20" class="shrink-0 text-(--rg-g) {choisi ? '' : 'invisible'}" />
          </button>
          <button
            type="button"
            class="shrink-0 w-8 flex items-center justify-center rounded-r-[10px] cursor-pointer text-(--lc-mu) hover:text-(--lc-tx)"
            onclick={() => { modalDevice = device; showDevices = false; }}
            title={$t("settings.audio_devices_details_action")}
            aria-label={$t("settings.audio_devices_details_action")}
          >
            <Icon icon="material-symbols-light:info-outline-rounded" width="18" />
          </button>
        </div>
      {/each}
      <!-- Lecteurs réseau : la recherche part à la première ouverture. -->
      {#if !reseauActif}
        <div class="mt-1 pt-1 border-t border-(--lc-menu-bd)">
          <button
            type="button"
            class="w-full flex items-center gap-2.5 px-2.5 py-2 rounded-[10px] text-left cursor-pointer text-(--lc-mu) hover:text-(--lc-tx) hover:bg-(--lc-survol)"
            onclick={() => { showDevices = false; goto("/settings/network"); }}
          >
            <Icon icon="material-symbols-light:cast-rounded" width="18" class="shrink-0" />
            <span class="flex-1 min-w-0 text-[12px] leading-snug">{$t("network_player.enable_hint")}</span>
            <Icon icon="material-symbols-light:arrow-forward-rounded" width="16" class="shrink-0" />
          </button>
        </div>
      {:else}
      <div class="mt-1 pt-1 border-t border-(--lc-menu-bd)">
        <div class="flex items-center gap-2 px-2.5 pt-1.5 pb-1.5">
          <p class="flex-1 flex items-center gap-2 text-[11px] font-bold uppercase tracking-[0.08em] text-(--lc-mu)">
            {$t("network_player.section")}<Badge tone="amber">{$t("settings.beta_badge")}</Badge>
          </p>
          <button
            type="button"
            class="h-6.5 px-2 flex items-center gap-1 rounded-md text-[11px] font-semibold cursor-pointer text-(--lc-mu) hover:text-(--lc-tx) hover:bg-(--lc-survol) disabled:cursor-default disabled:opacity-60"
            disabled={$networkPlayerStore.searching}
            onclick={() => networkPlayerStore.discover()}
          >
            <Icon icon="material-symbols-light:refresh-rounded" width="15" class={$networkPlayerStore.searching ? "animate-spin" : ""} />
            {$networkPlayerStore.searching ? $t("network_player.searching") : $t("network_player.search")}
          </button>
        </div>
        {#each $networkPlayerStore.renderers as r (r.id)}
          {@const choisi = distant?.id === r.id}
          <div class="relative flex items-stretch rounded-[10px] transition-colors {choisi ? 'bg-(--rg-g)/12' : 'hover:bg-(--lc-survol)'}">
            {#if choisi}<span class="absolute left-0 top-2.5 bottom-2.5 w-0.75 rounded-r-[3px] bg-(--rg-g)"></span>{/if}
            <button type="button" class="flex-1 min-w-0 flex items-center gap-2.5 px-2.5 py-2 text-left cursor-pointer" onclick={() => selectRenderer(r)}>
              <Icon icon={networkIcon(r)} width="20" class="shrink-0 {choisi ? 'text-(--rg-g)' : 'text-(--lc-mu)'}" />
              <span class="min-w-0 flex-1 leading-[1.2]">
                <span class="block truncate text-[13px] font-semibold">{r.name}</span>
                <span class="block truncate text-[11px] text-(--lc-mu)">{[r.kind === "airplay" ? "AirPlay" : r.manufacturer, r.model].filter(Boolean).join(" · ") || r.ip}</span>
              </span>
              <Icon icon="material-symbols-light:check-rounded" width="20" class="shrink-0 text-(--rg-g) {choisi ? '' : 'invisible'}" />
            </button>
          </div>
        {:else}
          {#if $networkPlayerStore.searched && !$networkPlayerStore.searching}
            <p class="px-2.5 pb-2 text-[11.5px] leading-snug text-(--lc-mu)">{$t("network_player.none")}</p>
          {/if}
        {/each}
      </div>
      {/if}

      {#if !distant}
      <div class="mt-1 pt-1 border-t border-(--lc-menu-bd)">
        {#if surWindows}
          <div class="flex items-center gap-2.5 px-2.5 py-2">
            <span class="min-w-0 flex-1 leading-[1.2]">
              <span class="block text-[13px] font-semibold">{$t("settings.output_exclusive")}</span>
              <span class="block text-[11px] text-(--lc-mu)">{wasapiExclusive ? $t("player.wasapi_on_short") : $t("player.wasapi_off_short")}</span>
            </span>
            <ToggleSwitch size="sm" checked={wasapiExclusive} label={$t("settings.output_exclusive")} onclick={() => settingsStore.toggle("wasapi_exclusive")} />
          </div>
        {/if}
        <div class="flex items-center gap-2.5 px-2.5 py-2">
          <span class="min-w-0 flex-1 leading-[1.2] {dopDisponible ? '' : 'opacity-60'}">
            <span class="block text-[13px] font-semibold">{$t("settings.dsd_dop")}</span>
            <span class="block text-[11px] text-(--lc-mu)">
              {!dopDisponible ? $t("settings.dsd_dop_needs_exclusive") : dop ? $t("player.dop_on_short") : $t("player.dop_off_short")}
            </span>
          </span>
          <ToggleSwitch size="sm" checked={dop} disabled={!dopDisponible} label={$t("settings.dsd_dop")} onclick={() => settingsStore.toggle("dsd_dop")} />
        </div>
      </div>
      {/if}
    </div>
  {/if}
</div>

<button
  type="button"
  disabled={dopActive}
  class="{bouton} disabled:cursor-not-allowed disabled:opacity-40
         {isMuted ? 'text-red-500/85 dark:text-red-400/85 hover:bg-(--lc-survol)' : 'text-(--lc-ic) hover:text-(--lc-ic-fort) hover:bg-(--lc-survol)'}"
  title={dopActive ? $t("player.volume_dop_locked") : isMuted ? $t("player.unmute") : $t("player.mute")}
  aria-label={isMuted ? $t("player.unmute") : $t("player.mute")}
  onclick={toggleMute}
>
  <Icon
    icon={isMuted ? "material-symbols-light:no-sound-outline-rounded" : value < 50 ? "material-symbols-light:volume-down-outline-rounded" : "material-symbols-light:volume-up-outline-rounded"}
    width="28"
  />
</button>

<!-- Volume : rail épais, couleur du morceau ; la pastille reste dans le rail, même à 0. -->
<div
  class="group/vol relative w-30 max-[1250px]:w-22 h-5 ml-1.5 shrink-0 flex items-center max-md:hidden {dopActive ? 'opacity-40' : ''}"
  title={dopActive ? $t("player.volume_dop_locked") : `${value} %`}
>
  <div class="relative w-full h-1.25 group-hover/vol:h-1.5 rounded-full bg-(--lc-rail) transition-[height] duration-150">
    <div
      class="absolute inset-y-0 left-0 rounded-full {dopActive ? 'bg-(--lc-mu2)' : isMuted ? 'bg-red-500/60' : 'bg-(--lc-acc)'}"
      style:width={dopActive ? "100%" : `calc((100% - 12px) * ${value / 100} + 6px)`}
    ></div>
    {#if !dopActive}
      <div
        class="absolute top-1/2 w-3 h-3 -mt-1.5 rounded-full bg-white shadow-[0_1px_4px_rgba(0,0,0,0.45)] pointer-events-none
               transition-transform group-hover/vol:scale-115"
        style:left="calc((100% - 12px) * {value / 100})"
      ></div>
    {/if}
  </div>
  {#if !dopActive}
    <input
      type="range"
      min="0"
      max="100"
      step="1"
      bind:value
      oninput={handleChange}
      class="absolute inset-0 w-full h-full cursor-pointer opacity-0"
      aria-label={$t("common.volume")}
    />
  {/if}
</div>

{#if modalDevice}
  <AudioDeviceDetailsModal
    device={modalDevice}
    isActive={$audioDevicesStore.activeDisplayName === modalDevice.displayName}
    onClose={() => (modalDevice = null)}
    onSelect={(d) => audioDevicesStore.setActive(d.displayName)}
  />
{/if}
