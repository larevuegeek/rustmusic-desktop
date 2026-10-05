<script lang="ts">
  // Sorties audio : une ligne par appareil, la sortie en cours cochée ; ⚙ ouvre ses capacités.
  import Icon from "@iconify/svelte";
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { t } from "#lib/i18n";
  import { audioDevicesStore, formatSampleRate, type AudioDeviceInfo } from "#lib/stores/audio/audioDevices.store";
  import { capacitesSortie, decrireSortie, palierQualite, PALIERS } from "#lib/helper/audio/deviceLabel";
  import { detectOS } from "#lib/helper/tools/osDetection";
  import Badge from "#lib/components/ui/text/Badge.svelte";
  import AudioDeviceDetailsModal from "./AudioDeviceDetailsModal.svelte";

  const os = detectOS();
  const systeme = os === "macos" ? "macOS" : os === "linux" ? "Linux" : "Windows";

  let detailsOf = $state<AudioDeviceInfo | null>(null);

  onMount(() => {
    audioDevicesStore.ensureLoaded();
  });

  async function choisir(device: AudioDeviceInfo) {
    if (device.displayName === $audioDevicesStore.activeDisplayName) return;
    try {
      await invoke("set_device", { deviceName: device.name });
      audioDevicesStore.setActive(device.displayName);
    } catch (err) {
      console.error("Failed to set device", err);
    }
  }

  function fiche(d: AudioDeviceInfo) {
    const { taux, bits, canaux } = capacitesSortie(d, $audioDevicesStore.exclusive[d.displayName]);
    const palier = palierQualite(taux, bits);
    return {
      palier: palier ? PALIERS[palier] : null,
      specs: [taux ? formatSampleRate(taux) : null, bits ? `${bits} bit` : null, `${canaux} ch`].filter((s): s is string => !!s),
    };
  }
</script>

{#snippet message(icone: string, texte: string, tourne = false)}
  <div class="flex items-center gap-2.5 px-5 py-4 text-[13px] text-(--rg-mu)">
    <Icon icon={icone} width="18" class={tourne ? "animate-spin" : ""} />
    {texte}
  </div>
{/snippet}

{#if $audioDevicesStore.error}
  {@render message("material-symbols:error-outline-rounded", $audioDevicesStore.error)}
{:else if !$audioDevicesStore.loaded && $audioDevicesStore.loading}
  {@render message("material-symbols:progress-activity", $t("settings.audio_devices_loading"), true)}
{:else if $audioDevicesStore.devices.length === 0}
  {@render message("material-symbols:speaker-outline-rounded", $t("settings.audio_devices_empty"))}
{:else}
  <div role="radiogroup" aria-label={$t("settings.output_group")}>
    {#each $audioDevicesStore.devices as device (device.displayName)}
      {@const actif = $audioDevicesStore.activeDisplayName === device.displayName}
      {@const libelle = decrireSortie(device)}
      {@const f = fiche(device)}
      <div
        role="radio"
        aria-checked={actif}
        tabindex="0"
        title={device.displayName}
        class="group relative flex items-center gap-3.5 pl-5 pr-4 py-3.5 cursor-pointer transition-colors
               border-t border-(--rg-line) first:border-t-0
               focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-(--rg-g)
               {actif ? 'bg-(--rg-creux-on)' : 'hover:bg-(--rg-hover)'}"
        onclick={() => choisir(device)}
        onkeydown={(e) => { if (e.key === " " || e.key === "Enter") { e.preventDefault(); choisir(device); } }}
      >
        {#if actif}<span class="absolute left-0 top-2.5 bottom-2.5 w-0.75 rounded-r-[3px] bg-(--rg-g)"></span>{/if}
        <span class="relative shrink-0 w-5 h-5 rounded-full border-2 {actif ? 'border-(--rg-g)' : 'border-(--rg-bd2)'}">
          {#if actif}<span class="absolute inset-0.75 rounded-full bg-(--rg-g)"></span>{/if}
        </span>
        <span class="shrink-0 w-9.5 h-9.5 rounded-[10px] flex items-center justify-center
                     {actif ? 'bg-(--rg-gbg) text-(--rg-gtx)' : 'bg-(--rg-s2) text-(--rg-mu)'}">
          <Icon icon={libelle.icone} width="20" />
        </span>
        <div class="flex-1 min-w-0 leading-[1.2]">
          <p class="flex items-center gap-2 min-w-0 text-[15px] font-semibold text-(--rg-tx)">
            <span class="truncate">{libelle.nom}</span>
            {#if actif}
              <Badge tone="green">{$t("settings.audio_devices_active_badge")}</Badge>
            {:else if device.isDefault}
              <Badge>{$t("settings.audio_devices_default_os").replace("{os}", systeme)}</Badge>
            {/if}
          </p>
          {#if libelle.detail}<p class="mt-0.75 text-[13px] text-(--rg-mu) truncate">{libelle.detail}</p>{/if}
        </div>
        {#if f.palier}<Badge tone={f.palier.ton}>{$t(f.palier.labelKey)}</Badge>{/if}
        <!-- Étroit : la fréquence seule, le badge résume déjà la qualité. -->
        <div class="shrink-0 flex gap-1.5 font-mono text-xs text-(--rg-mu) max-md:hidden">
          {#each f.specs as spec, i (spec)}
            <span class="px-1.75 py-0.75 rounded-[5px] whitespace-nowrap bg-(--rg-creux) border border-(--rg-line) {i > 0 ? 'max-lg:hidden' : ''}">{spec}</span>
          {/each}
        </div>
        <button
          type="button"
          class="shrink-0 w-8 h-8 rounded-lg flex items-center justify-center cursor-pointer transition
                 text-(--rg-mu) hover:text-(--rg-tx) hover:bg-(--rg-s2)
                 {actif ? '' : 'opacity-0 group-hover:opacity-100 focus-visible:opacity-100'}"
          onclick={(e) => { e.stopPropagation(); detailsOf = device; }}
          aria-label={$t("settings.audio_devices_details_action")}
          title={$t("settings.audio_devices_details_action")}
        >
          <Icon icon="material-symbols:tune-rounded" width="20" />
        </button>
      </div>
    {/each}
  </div>
{/if}

{#if detailsOf}
  <AudioDeviceDetailsModal
    device={detailsOf}
    isActive={$audioDevicesStore.activeDisplayName === detailsOf.displayName}
    onClose={() => (detailsOf = null)}
    onSelect={(d) => choisir(d)}
  />
{/if}
