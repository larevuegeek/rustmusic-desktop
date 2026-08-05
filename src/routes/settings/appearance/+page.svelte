<script lang="ts">
  import Icon from "@iconify/svelte";
  import { onMount } from "svelte";
  import { settingsStore } from "$lib/stores/settings/settings.store";
  import { t } from "$lib/i18n";
  import AppearanceCard from "$lib/components/settings/AppearanceCard.svelte";
  import {
    getRenderMode,
    setRenderMode,
    type RenderModeStatus,
    type RenderMode,
  } from "$lib/services/system/renderMode.service";

  let theme = $derived($settingsStore.theme);
  let windowControlsStyle = $derived($settingsStore.window_controls_style);
  let windowControlsPosition = $derived($settingsStore.window_controls_position);

  // ─── Mode de rendu (Linux WebKit env vars) ───
  let renderMode = $state<RenderModeStatus | null>(null);
  let renderModeSaving = $state(false);
  let renderModeChanged = $state(false); // for "restart required" hint
  let renderModeInitial: RenderMode | null = null;

  onMount(async () => {
    try {
      renderMode = await getRenderMode();
      renderModeInitial = renderMode.mode;
    } catch (e) {
      console.error("[render mode] init failed:", e);
    }
  });

  async function handleRenderModeChange(value: RenderMode) {
    if (renderModeSaving) return;
    renderModeSaving = true;
    try {
      renderMode = await setRenderMode(value);
      // Flag "restart required" if the user changed the value from what was
      // active at app boot (env vars are baked in at startup).
      renderModeChanged = renderModeInitial !== null && renderModeInitial !== value;
    } catch (e) {
      console.error("[render mode] save failed:", e);
    } finally {
      renderModeSaving = false;
    }
  }
</script>

<section>
  <!-- ─── Thème ─── -->
  <div class="mb-5">
    <div class="flex items-center gap-3 mb-2.5 px-1">
      <Icon icon="lucide:palette" width="18" class="text-neutral-400" />
      <div>
        <p class="text-sm font-medium text-neutral-800 dark:text-neutral-200">{$t('settings.theme')}</p>
        <p class="text-[11px] text-neutral-400 dark:text-neutral-500">{$t('settings.theme_desc')}</p>
      </div>
    </div>

    <div class="grid grid-cols-3 gap-2 max-w-2xl">
      <AppearanceCard
        label={$t('settings.theme_auto')}
        selected={theme === 'auto'}
        onclick={() => settingsStore.set('theme', 'auto')}
      >
        <div class="w-full h-full flex">
          <div class="flex-1 bg-white relative overflow-hidden">
            <div class="absolute top-2 left-2 w-1/2 h-1 rounded-full bg-neutral-300"></div>
            <div class="absolute top-4 left-2 w-1/3 h-1 rounded-full bg-neutral-200"></div>
          </div>
          <div class="flex-1 bg-neutral-900 relative overflow-hidden">
            <div class="absolute top-2 left-2 w-1/2 h-1 rounded-full bg-neutral-500"></div>
            <div class="absolute top-4 left-2 w-1/3 h-1 rounded-full bg-neutral-700"></div>
          </div>
        </div>
      </AppearanceCard>

      <AppearanceCard
        label={$t('settings.theme_light')}
        selected={theme === 'light'}
        onclick={() => settingsStore.set('theme', 'light')}
      >
        <div class="w-full h-full bg-white relative">
          <div class="absolute top-2 left-2 w-1/2 h-1 rounded-full bg-neutral-300"></div>
          <div class="absolute top-4 left-2 w-1/3 h-1 rounded-full bg-neutral-200"></div>
          <div class="absolute bottom-2 right-2 w-3 h-3 rounded-full bg-green-500"></div>
        </div>
      </AppearanceCard>

      <AppearanceCard
        label={$t('settings.theme_dark')}
        selected={theme === 'dark'}
        onclick={() => settingsStore.set('theme', 'dark')}
      >
        <div class="w-full h-full bg-neutral-900 relative">
          <div class="absolute top-2 left-2 w-1/2 h-1 rounded-full bg-neutral-500"></div>
          <div class="absolute top-4 left-2 w-1/3 h-1 rounded-full bg-neutral-700"></div>
          <div class="absolute bottom-2 right-2 w-3 h-3 rounded-full bg-green-500"></div>
        </div>
      </AppearanceCard>
    </div>
  </div>

  <!-- ─── Style des contrôles fenêtre ─── -->
  <div class="mb-5">
    <div class="flex items-center gap-3 mb-2.5 px-1">
      <Icon icon="lucide:app-window" width="18" class="text-neutral-400" />
      <div>
        <p class="text-sm font-medium text-neutral-800 dark:text-neutral-200">{$t('settings.window_controls_style')}</p>
        <p class="text-[11px] text-neutral-400 dark:text-neutral-500">{$t('settings.window_controls_style_desc')}</p>
      </div>
    </div>

    <div class="grid grid-cols-4 gap-2 max-w-2xl">
      <AppearanceCard
        label={$t('settings.auto')}
        selected={windowControlsStyle === 'auto'}
        onclick={() => settingsStore.set('window_controls_style', 'auto')}
      >
        <div class="flex items-center gap-1">
          <Icon icon="lucide:cpu" width={20} class="text-neutral-400" />
          <span class="text-[10px] text-neutral-400">OS</span>
        </div>
      </AppearanceCard>

      <AppearanceCard
        label="macOS"
        selected={windowControlsStyle === 'macos'}
        onclick={() => settingsStore.set('window_controls_style', 'macos')}
      >
        <div class="flex items-center gap-1.5">
          <div class="w-2.5 h-2.5 rounded-full bg-[#ff5f57]"></div>
          <div class="w-2.5 h-2.5 rounded-full bg-[#febc2e]"></div>
          <div class="w-2.5 h-2.5 rounded-full bg-[#28c840]"></div>
        </div>
      </AppearanceCard>

      <AppearanceCard
        label="Windows"
        selected={windowControlsStyle === 'windows'}
        onclick={() => settingsStore.set('window_controls_style', 'windows')}
      >
        <div class="flex items-center gap-2 text-neutral-500 dark:text-neutral-300">
          <svg viewBox="0 0 12 12" width="11" height="11" fill="currentColor"><rect x="2" y="5.5" width="8" height="1"/></svg>
          <svg viewBox="0 0 12 12" width="10" height="10" fill="none" stroke="currentColor" stroke-width="1"><rect x="2.5" y="2.5" width="7" height="7"/></svg>
          <svg viewBox="0 0 12 12" width="10" height="10" fill="none" stroke="currentColor" stroke-width="1.1"><path d="M2.5 2.5l7 7M9.5 2.5l-7 7"/></svg>
        </div>
      </AppearanceCard>

      <AppearanceCard
        label="Linux"
        selected={windowControlsStyle === 'linux'}
        onclick={() => settingsStore.set('window_controls_style', 'linux')}
      >
        <div class="flex items-center gap-1.5">
          <div class="w-4 h-4 rounded-full bg-neutral-200 dark:bg-white/10 flex items-center justify-center">
            <svg viewBox="0 0 12 12" width="7" height="7" fill="currentColor" class="text-neutral-500 dark:text-neutral-300"><rect x="2" y="5.5" width="8" height="1"/></svg>
          </div>
          <div class="w-4 h-4 rounded-full bg-neutral-200 dark:bg-white/10 flex items-center justify-center">
            <svg viewBox="0 0 12 12" width="7" height="7" fill="none" stroke="currentColor" stroke-width="1.4" class="text-neutral-500 dark:text-neutral-300"><rect x="2.5" y="2.5" width="7" height="7"/></svg>
          </div>
          <div class="w-4 h-4 rounded-full bg-neutral-200 dark:bg-white/10 flex items-center justify-center">
            <svg viewBox="0 0 12 12" width="7" height="7" fill="none" stroke="currentColor" stroke-width="1.4" class="text-neutral-500 dark:text-neutral-300"><path d="M2.5 2.5l7 7M9.5 2.5l-7 7"/></svg>
          </div>
        </div>
      </AppearanceCard>
    </div>
  </div>

  <!-- ─── Position des contrôles fenêtre (segmented control) ─── -->
  <div class="flex items-center justify-between px-1">
    <div class="flex items-center gap-3">
      <Icon icon="lucide:arrow-left-right" width="18" class="text-neutral-400" />
      <div>
        <p class="text-sm font-medium text-neutral-800 dark:text-neutral-200">{$t('settings.window_controls_position')}</p>
        <p class="text-[11px] text-neutral-400 dark:text-neutral-500">{$t('settings.window_controls_position_desc')}</p>
      </div>
    </div>

    <div class="inline-flex p-0.5 rounded-lg bg-neutral-100 dark:bg-white/5 border border-neutral-200/60 dark:border-white/8">
      <button
        type="button"
        class="px-3 py-1.5 text-xs font-medium rounded-md cursor-pointer transition-all
               {windowControlsPosition === 'left'
                 ? 'bg-white dark:bg-neutral-800 text-neutral-900 dark:text-white shadow-sm'
                 : 'text-neutral-500 dark:text-neutral-400 hover:text-neutral-800 dark:hover:text-neutral-200'}"
        onclick={() => settingsStore.set('window_controls_position', 'left')}
      >
        {$t('settings.left')}
      </button>
      <button
        type="button"
        class="px-3 py-1.5 text-xs font-medium rounded-md cursor-pointer transition-all
               {windowControlsPosition === 'right'
                 ? 'bg-white dark:bg-neutral-800 text-neutral-900 dark:text-white shadow-sm'
                 : 'text-neutral-500 dark:text-neutral-400 hover:text-neutral-800 dark:hover:text-neutral-200'}"
        onclick={() => settingsStore.set('window_controls_position', 'right')}
      >
        {$t('settings.right')}
      </button>
    </div>
  </div>

  <!-- ─── Mode de rendu (Linux seulement — vide ailleurs) ─── -->
  {#if renderMode}
    <div class="mt-5">
      <div class="flex items-center gap-3 mb-2.5 px-1">
        <Icon icon="lucide:monitor-cog" width="18" class="text-neutral-400" />
        <div>
          <p class="text-sm font-medium text-neutral-800 dark:text-neutral-200">{$t('settings.render_mode')}</p>
          <p class="text-[11px] text-neutral-400 dark:text-neutral-500">{$t('settings.render_mode_desc')}</p>
        </div>
      </div>

      <div class="grid grid-cols-3 gap-2 max-w-2xl">
        <AppearanceCard
          label={$t('settings.render_mode_auto')}
          selected={renderMode.mode === 'auto'}
          onclick={() => handleRenderModeChange('auto')}
        >
          <div class="flex items-center justify-center gap-1.5 text-neutral-400">
            <Icon icon="lucide:cpu" width={18} />
            <span class="text-[10px]">{$t('settings.audio_quality_auto_hint')}</span>
          </div>
        </AppearanceCard>

        <AppearanceCard
          label={$t('settings.render_mode_gpu')}
          selected={renderMode.mode === 'force-gpu'}
          onclick={() => handleRenderModeChange('force-gpu')}
        >
          <div class="flex items-center justify-center gap-1.5 text-emerald-500">
            <Icon icon="lucide:zap" width={18} />
            <span class="text-[10px]">GPU</span>
          </div>
        </AppearanceCard>

        <AppearanceCard
          label={$t('settings.render_mode_software')}
          selected={renderMode.mode === 'force-software'}
          onclick={() => handleRenderModeChange('force-software')}
        >
          <div class="flex items-center justify-center gap-1.5 text-sky-500">
            <Icon icon="lucide:shield-check" width={18} />
            <span class="text-[10px]">SW</span>
          </div>
        </AppearanceCard>
      </div>

      <!-- Détection environnement + alerte "restart required" -->
      {#if renderMode.virt_kind}
        <div class="mt-3 px-3 py-2 rounded-lg text-[11px] text-neutral-500 dark:text-neutral-400
                    bg-neutral-50 dark:bg-white/2 border border-neutral-200/60 dark:border-white/5
                    flex items-start gap-2 max-w-2xl">
          <Icon icon="lucide:info" width={13} class="mt-0.5 shrink-0 text-neutral-400" />
          <span>{$t('settings.render_mode_vm_detected').replace('{kind}', renderMode.virt_kind)}</span>
        </div>
      {/if}

      {#if renderModeChanged}
        <div class="mt-2 px-3 py-2 rounded-lg text-[11px]
                    bg-amber-500/10 border border-amber-500/30 text-amber-700 dark:text-amber-300
                    flex items-start gap-2 max-w-2xl">
          <Icon icon="lucide:rotate-ccw" width={13} class="mt-0.5 shrink-0" />
          <span>{$t('settings.render_mode_restart_required')}</span>
        </div>
      {/if}
    </div>
  {/if}
</section>
