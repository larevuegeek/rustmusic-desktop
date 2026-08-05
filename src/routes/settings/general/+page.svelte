<script lang="ts">
  import { settingsStore } from "$lib/stores/settings/settings.store";
  import { t } from "$lib/i18n";
  import OptionRow from "$lib/components/ui/input/OptionRow.svelte";
  import ToggleSwitch from "$lib/components/ui/input/ToggleSwitch.svelte";

  let language = $derived($settingsStore.language);
  let autoStart = $derived($settingsStore.auto_start === 'true');
  let minimizeToTray = $derived($settingsStore.minimize_to_tray === 'true');
  let scanOnStartup = $derived($settingsStore.scan_on_startup === 'true');
  let singleClickPlay = $derived($settingsStore.single_click_play === 'true');
  let showSleepTimer = $derived($settingsStore.show_sleep_timer !== 'false');
  let notifications = $derived($settingsStore.show_notifications === 'true');
  let systemMediaControls = $derived($settingsStore.system_media_controls === 'true');

  const languages = [
    { value: 'fr', label: 'Français' },
    { value: 'en', label: 'English' },
    { value: 'es', label: 'Español' },
    { value: 'de', label: 'Deutsch' },
    { value: 'it', label: 'Italiano' },
  ];
</script>

<section class="space-y-1">
  <!-- Langue -->
  <OptionRow icon="lucide:languages" title={$t('settings.language')} desc={$t('settings.language_desc')}>
    <select
      value={language}
      onchange={(e) => settingsStore.set('language', (e.target as HTMLSelectElement).value)}
      class="text-sm appearance-none
             bg-neutral-100 dark:bg-neutral-800/80
             border border-neutral-200/80 dark:border-neutral-700/60
             rounded-xl px-4 py-2 pr-8
             text-neutral-700 dark:text-neutral-200
             shadow-sm
             hover:border-neutral-300 dark:hover:border-neutral-600
             focus:outline-none focus:ring-2 focus:ring-green-500/30 focus:border-green-500/40
             cursor-pointer transition-all duration-150
             bg-[url('data:image/svg+xml;charset=UTF-8,%3csvg%20xmlns%3d%22http%3a//www.w3.org/2000/svg%22%20width%3d%2212%22%20height%3d%2212%22%20viewBox%3d%220%200%2024%2024%22%20fill%3d%22none%22%20stroke%3d%22%239ca3af%22%20stroke-width%3d%222%22%3e%3cpath%20d%3d%22m6%209%206%206%206-6%22/%3e%3c/svg%3e')]
             bg-no-repeat bg-position-[right_0.75rem_center]"
    >
      {#each languages as lang}
        <option value={lang.value}>{lang.label}</option>
      {/each}
    </select>
  </OptionRow>

  <!-- Exécution automatique -->
  <OptionRow icon="lucide:power" title={$t('settings.autostart')} desc={$t('settings.autostart_desc')}>
    <ToggleSwitch checked={autoStart} label="Lancement au démarrage" onclick={() => settingsStore.toggle('auto_start')} />
  </OptionRow>

  <!-- Minimiser dans le tray -->
  <OptionRow icon="lucide:minimize-2" title={$t('settings.tray')} desc={$t('settings.tray_desc')}>
    <ToggleSwitch checked={minimizeToTray} label="Minimiser dans la barre système" onclick={() => settingsStore.toggle('minimize_to_tray')} />
  </OptionRow>

  <!-- Scan au démarrage -->
  <OptionRow icon="lucide:refresh-cw" title={$t('settings.scan_on_startup')} desc={$t('settings.scan_on_startup_desc')}>
    <ToggleSwitch checked={scanOnStartup} label="Scan au démarrage" onclick={() => settingsStore.toggle('scan_on_startup')} />
  </OptionRow>

  <!-- Simple clic = lecture -->
  <OptionRow icon="lucide:mouse-pointer-click" title={$t('settings.single_click_play')} desc={$t('settings.single_click_play_desc')}>
    <ToggleSwitch checked={singleClickPlay} label="Simple clic = lecture" onclick={() => settingsStore.toggle('single_click_play')} />
  </OptionRow>

  <!-- Bouton minuteur de veille dans le header -->
  <OptionRow icon="tabler:alarm-snooze" title={$t('settings.show_sleep_timer')} desc={$t('settings.show_sleep_timer_desc')}>
    <ToggleSwitch checked={showSleepTimer} label="Minuteur de veille" onclick={() => settingsStore.toggle('show_sleep_timer')} />
  </OptionRow>

  <!-- Notifications OS -->
  <OptionRow icon="lucide:bell" title={$t('settings.notifications')} desc={$t('settings.notifications_desc')}>
    <ToggleSwitch checked={notifications} label="Notifications" onclick={() => settingsStore.toggle('show_notifications')} />
  </OptionRow>

  <!-- Contrôles média système (SMTC/MPRIS/NowPlaying) -->
  <OptionRow icon="lucide:radio" title={$t('settings.system_media_controls')} desc={$t('settings.system_media_controls_desc')}>
    <ToggleSwitch checked={systemMediaControls} label="System media controls" onclick={() => settingsStore.toggle('system_media_controls')} />
  </OptionRow>
</section>
