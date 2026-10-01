<script lang="ts">
  // Sonde la sortie choisie en mode exclusif : fréquences acceptées, et DSD qu'elles permettent en DoP.
  import Icon from "@iconify/svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { t } from "$lib/i18n";
  import { formatSampleRate, type AudioDeviceInfo, type WasapiDeviceCapabilities } from "$lib/stores/audio/audioDevices.store";
  import { decrireSortie } from "$lib/helper/audio/deviceLabel";
  import GhostButton from "$lib/components/ui/button/GhostButton.svelte";

  let { device }: { device: AudioDeviceInfo | null } = $props();

  // Mêmes fréquences que la sonde du backend (`PROBE_RATES`).
  const FREQUENCES = [44_100, 48_000, 88_200, 96_000, 176_400, 192_000, 352_800, 384_000, 705_600, 768_000];
  // Le DoP transporte le DSD en PCM 24 bits à ces fréquences porteuses.
  const PORTEUSES = [
    { dsd: "DSD64", taux: 176_400 },
    { dsd: "DSD128", taux: 352_800 },
    { dsd: "DSD256", taux: 705_600 },
  ];

  let enCours = $state(false);
  let caps = $state<WasapiDeviceCapabilities | null>(null);
  let erreur = $state<string | null>(null);
  let testee = $state<string | null>(null);

  // Autre sortie choisie : l'ancien résultat ne vaut plus.
  $effect(() => {
    if (device?.displayName !== testee) {
      caps = null;
      erreur = null;
    }
  });

  const dsd = $derived(
    caps && caps.exclusiveBitDepths.some((b) => b >= 24)
      ? PORTEUSES.filter((p) => caps!.exclusiveRates.includes(p.taux)).map((p) => p.dsd)
      : [],
  );

  async function tester() {
    if (!device?.wasapiId) return;
    enCours = true;
    erreur = null;
    caps = null;
    testee = device.displayName;
    try {
      caps = await invoke<WasapiDeviceCapabilities>("wasapi_probe_device_capabilities", { deviceId: device.wasapiId });
    } catch (e) {
      erreur = e instanceof Error ? e.message : String(e);
    } finally {
      enCours = false;
    }
  }
</script>

<div class="flex flex-col gap-3 px-5 py-4 leading-[1.2]">
  <div class="flex items-center gap-4">
    <div class="flex-1 min-w-0">
      <p class="text-[15px] font-semibold text-(--rg-tx)">{$t("settings.dac_test")}</p>
      <p class="mt-0.75 text-[13px] leading-[1.25] text-(--rg-mu) text-pretty">
        {device ? $t("settings.dac_test_desc").replace("{device}", decrireSortie(device).nom) : $t("settings.dac_test_none")}
      </p>
    </div>
    <GhostButton
      icon="material-symbols:science-outline-rounded"
      label={enCours ? $t("settings.dac_test_running") : caps ? $t("settings.dac_test_again") : $t("settings.dac_test_run")}
      busy={enCours}
      disabled={!device?.wasapiId}
      onclick={tester}
    />
  </div>

  {#if erreur}
    <p class="text-[13px] text-(--rg-am)">{$t("settings.dac_test_failed")} <span class="text-(--rg-mu)">{erreur}</span></p>
  {:else if caps}
    <div class="flex flex-wrap gap-1.5 font-mono text-xs">
      {#each FREQUENCES as f (f)}
        {@const ok = caps.exclusiveRates.includes(f)}
        <span
          class="flex items-center gap-1 px-2 py-1 rounded-md border whitespace-nowrap
                 {ok ? 'bg-(--rg-gbg) border-(--rg-gbd) text-(--rg-gtx)' : 'bg-(--rg-creux) border-(--rg-line) text-(--rg-mu2) line-through'}"
        >
          {#if ok}<Icon icon="material-symbols:check-rounded" width="14" />{/if}
          {formatSampleRate(f)}
        </span>
      {/each}
      <span
        class="flex items-center gap-1 px-2 py-1 rounded-md border whitespace-nowrap
               {dsd.length ? 'bg-(--rg-gbg) border-(--rg-gbd) text-(--rg-gtx)' : 'bg-(--rg-creux) border-(--rg-line) text-(--rg-mu2) line-through'}"
      >
        {#if dsd.length}<Icon icon="material-symbols:check-rounded" width="14" />{dsd.join(" · ")}{:else}DSD (DoP){/if}
      </span>
    </div>
  {/if}
</div>
