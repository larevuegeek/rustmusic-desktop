<script lang="ts">
  import { onMount } from "svelte";
  import { t } from "#lib/i18n";
  import OptionGroup from "#lib/components/ui/input/OptionGroup.svelte";
  import OptionItem from "#lib/components/ui/input/OptionItem.svelte";
  import OptionBlock from "#lib/components/ui/input/OptionBlock.svelte";
  import TextField from "#lib/components/ui/input/TextField.svelte";
  import ToggleSwitch from "#lib/components/ui/input/ToggleSwitch.svelte";
  import GhostButton from "#lib/components/ui/button/GhostButton.svelte";
  import {
    dlnaGetSettings,
    dlnaStart,
    dlnaStop,
    dlnaUpdateSettings,
    type DlnaSettings,
  } from "#lib/services/dlna/dlna.service";
  import { dlnaStatusStore, refreshDlnaStatus } from "#lib/stores/dlna/dlna.store";
  import { settingsStore } from "#lib/stores/settings/settings.store";

  let dlnaSettings = $state<DlnaSettings | null>(null);
  let dlnaToggling = $state(false);
  let dlnaSavingName = $state(false);
  let dlnaSavingPort = $state(false);
  let dlnaCopied = $state(false);

  const enLigne = $derived($dlnaStatusStore?.running ?? false);

  // Absent des réglages d'une installation antérieure : « actif » par défaut.
  const autoArtistImages = $derived($settingsStore.auto_download_artist_images !== "false");

  onMount(async () => {
    try {
      dlnaSettings = await dlnaGetSettings();
      await refreshDlnaStatus();
    } catch (e) {
      console.error("[dlna] init failed:", e);
    }
  });

  async function handleDlnaToggle() {
    if (dlnaToggling) return;
    dlnaToggling = true;
    try {
      const status = enLigne ? await dlnaStop() : await dlnaStart();
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
</script>

<!-- ─── Serveur DLNA ─── -->
<OptionGroup title={$t("settings.dlna_group")} hint={$t("settings.dlna_group_hint")}>
  <OptionItem
    title={$t("settings.dlna_share")}
    desc={$t("settings.dlna_enabled_desc")}
    keywords="dlna upnp sonos"
    onclick={dlnaToggling ? undefined : handleDlnaToggle}
  >
    <ToggleSwitch checked={enLigne} disabled={dlnaToggling} label={$t("settings.dlna_share")} onclick={handleDlnaToggle} />
  </OptionItem>

  {#if enLigne && $dlnaStatusStore?.url}
    <OptionBlock keywords="{$t('settings.dlna_active')} url adresse" class="flex items-center gap-4 px-5 py-4 leading-[1.2]">
      <span class="relative shrink-0 flex w-2.5 h-2.5">
        <span class="absolute inset-0 rounded-full bg-(--rg-g) opacity-50 animate-ping"></span>
        <span class="relative w-2.5 h-2.5 rounded-full bg-(--rg-g)"></span>
      </span>
      <div class="flex-1 min-w-0">
        <p class="text-[15px] font-semibold text-(--rg-gtx)">{$t("settings.dlna_active")}</p>
        <p class="mt-0.75 font-mono text-[13px] text-(--rg-mu) truncate" title={$dlnaStatusStore.url}>{$dlnaStatusStore.url}</p>
      </div>
      <GhostButton
        icon={dlnaCopied ? "material-symbols:check-rounded" : "material-symbols:content-copy-outline-rounded"}
        label={dlnaCopied ? $t("settings.dlna_copied") : $t("settings.dlna_copy_url")}
        onclick={handleDlnaCopyUrl}
      />
    </OptionBlock>
  {/if}

  {#if dlnaSettings}
    <OptionItem title={$t("settings.dlna_friendly_name")} desc={$t("settings.dlna_friendly_name_desc")}>
      <TextField
        value={dlnaSettings.friendly_name}
        label={$t("settings.dlna_friendly_name")}
        maxlength={64}
        class="w-56 max-sm:w-40"
        onchange={handleDlnaNameSave}
      />
    </OptionItem>
    <OptionItem title={$t("settings.dlna_port")} desc={$t("settings.dlna_port_desc")} keywords="http">
      <TextField
        type="number"
        value={dlnaSettings.port}
        label={$t("settings.dlna_port")}
        min={1}
        max={65535}
        mono
        class="w-24"
        onchange={(v) => handleDlnaPortSave(parseInt(v, 10))}
      />
    </OptionItem>
  {/if}
</OptionGroup>

<!-- ─── Services en ligne : seul Deezer est interrogé sans demande explicite ─── -->
<OptionGroup title={$t("settings.online_group")} hint={$t("settings.online_group_hint")}>
  <OptionItem
    title={$t("settings.auto_download_artist_images")}
    desc={$t("settings.auto_download_artist_images_desc")}
    keywords="deezer portrait"
    onclick={() => settingsStore.toggle("auto_download_artist_images")}
  >
    <ToggleSwitch
      checked={autoArtistImages}
      label={$t("settings.auto_download_artist_images")}
      onclick={() => settingsStore.toggle("auto_download_artist_images")}
    />
  </OptionItem>
</OptionGroup>
