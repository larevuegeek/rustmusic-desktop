<script lang="ts">
  import Icon from "@iconify/svelte";
  import { onMount } from "svelte";
  import { t } from "$lib/i18n";
  import OptionRow from "$lib/components/ui/input/OptionRow.svelte";
  import ToggleSwitch from "$lib/components/ui/input/ToggleSwitch.svelte";
  import {
    dlnaGetSettings,
    dlnaStart,
    dlnaStop,
    dlnaUpdateSettings,
    type DlnaSettings,
  } from "$lib/services/dlna/dlna.service";
  import { dlnaStatusStore, refreshDlnaStatus } from "$lib/stores/dlna/dlna.store";
  import { settingsStore } from "$lib/stores/settings/settings.store";

  let dlnaSettings = $state<DlnaSettings | null>(null);
  let dlnaToggling = $state(false);
  let dlnaSavingName = $state(false);
  let dlnaSavingPort = $state(false);
  let dlnaCopied = $state(false);

  // Absent des réglages d'une installation antérieure : on lit donc
  // « actif » par défaut, pour ne pas changer le comportement d'un coup.
  let autoArtistImages = $derived(
    $settingsStore.auto_download_artist_images !== 'false'
  );

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
      const status = $dlnaStatusStore?.running ? await dlnaStop() : await dlnaStart();
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

<section class="space-y-1">
  <!-- Toggle ON/OFF -->
  <OptionRow icon="lucide:radio-tower" title={$t('settings.dlna_enabled')} desc={$t('settings.dlna_enabled_desc')}>
    <ToggleSwitch
      checked={$dlnaStatusStore?.running ?? false}
      color="emerald"
      disabled={dlnaToggling}
      label={$dlnaStatusStore?.running ? 'Désactiver DLNA' : 'Activer DLNA'}
      onclick={handleDlnaToggle}
    />
  </OptionRow>

  <!-- URL active (visible seulement quand le serveur tourne) -->
  {#if $dlnaStatusStore?.running && $dlnaStatusStore.url}
    <div class="flex items-center justify-between px-4 py-3 rounded-xl
                bg-emerald-50 dark:bg-emerald-500/8 border border-emerald-200/60 dark:border-emerald-500/20">
      <div class="flex items-center gap-3 min-w-0">
        <div class="relative flex h-2.5 w-2.5 items-center justify-center shrink-0">
          <span class="absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-50 animate-ping"></span>
          <span class="relative inline-flex h-2 w-2 rounded-full bg-emerald-500 shadow-[0_0_6px_rgba(16,185,129,0.6)]"></span>
        </div>
        <div class="min-w-0">
          <p class="text-[11px] font-semibold uppercase tracking-wider text-emerald-700 dark:text-emerald-400">{$t('settings.dlna_active')}</p>
          <p class="text-sm font-mono text-neutral-800 dark:text-neutral-200 truncate">{$dlnaStatusStore.url}</p>
        </div>
      </div>
      <button
        type="button"
        class="text-[11px] px-3 py-1.5 rounded-md cursor-pointer
               bg-white dark:bg-white/10 border border-neutral-200 dark:border-white/15
               hover:bg-neutral-50 dark:hover:bg-white/15 text-neutral-700 dark:text-neutral-200
               transition-colors flex items-center gap-1.5 shrink-0"
        onclick={handleDlnaCopyUrl}
        aria-label="Copier l'URL"
      >
        <Icon icon={dlnaCopied ? 'lucide:check' : 'lucide:copy'} width={12} />
        {dlnaCopied ? $t('settings.dlna_copied') : $t('settings.dlna_copy_url')}
      </button>
    </div>
  {/if}

  {#if dlnaSettings}
    <!-- Friendly name -->
    <OptionRow icon="lucide:tag" title={$t('settings.dlna_friendly_name')} desc={$t('settings.dlna_friendly_name_desc')}>
      <input
        type="text"
        value={dlnaSettings.friendly_name}
        onblur={(e) => handleDlnaNameSave(e.currentTarget.value)}
        maxlength="64"
        class="text-sm w-56 px-3 py-1.5 rounded-md
               bg-white dark:bg-white/5 border border-neutral-200 dark:border-white/10
               text-neutral-800 dark:text-neutral-200
               focus:outline-none focus:border-emerald-400 dark:focus:border-emerald-500"
      />
    </OptionRow>

    <!-- Port -->
    <OptionRow icon="lucide:plug" title={$t('settings.dlna_port')} desc={$t('settings.dlna_port_desc')}>
      <input
        type="number"
        min="1"
        max="65535"
        value={dlnaSettings.port}
        onblur={(e) => handleDlnaPortSave(parseInt(e.currentTarget.value, 10))}
        class="text-sm w-24 px-3 py-1.5 rounded-md tabular-nums text-right
               bg-white dark:bg-white/5 border border-neutral-200 dark:border-white/10
               text-neutral-800 dark:text-neutral-200
               focus:outline-none focus:border-emerald-400 dark:focus:border-emerald-500"
      />
    </OptionRow>
  {/if}
</section>

<!-- ───────────────────────────────────────────────────────────────────────
     Téléchargements automatiques

     L'application ne contacte d'elle-même qu'un seul service : Deezer, pour
     le portrait d'un artiste dont la fiche s'ouvre sans image. Tout le reste
     — pochettes, paroles, correction de tags — attend une demande explicite.
     ─────────────────────────────────────────────────────────────────────── -->
<section class="space-y-1 mt-8">
  <OptionRow
    icon="lucide:user-round-search"
    title={$t('settings.auto_download_artist_images')}
    desc={$t('settings.auto_download_artist_images_desc')}
  >
    <ToggleSwitch
      checked={autoArtistImages}
      label={$t('settings.auto_download_artist_images')}
      onclick={() => settingsStore.toggle('auto_download_artist_images')}
    />
  </OptionRow>
</section>
