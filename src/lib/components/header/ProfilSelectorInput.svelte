<script lang="ts">
  import { profilSelector } from "#lib/stores/profil/profil.store";
  import { profilPopinStore } from "#lib/stores/profil/profilPopin.store";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { goto } from "$app/navigation";
  import Icon from "@iconify/svelte";
  import { scale } from "svelte/transition";
  import { t } from "#lib/i18n";

  let isOpen = $state(false);
  let menuEl: HTMLElement | null = $state(null);

  const profil = $derived($profilSelector.profilSelected);
  const profilColor = $derived(profil?.color ?? '#22c55e');

  function toggleMenu() {
    isOpen = !isOpen;
  }

  function openProfilSwitch() {
    isOpen = false;
    profilPopinStore.open();
  }

  function openSettings() {
    isOpen = false;
    goto('/settings');
  }

  function openAbout() {
    isOpen = false;
    goto('/settings/about');
  }

  function openFeedback() {
    isOpen = false;
    goto('/feedback');
  }

  async function quitApp() {
    isOpen = false;
    const appWindow = getCurrentWindow();
    await appWindow.close();
  }

  // Fermer au clic extérieur
  $effect(() => {
    if (!isOpen) return;

    function handleClickOutside(e: MouseEvent) {
      if (menuEl && !menuEl.contains(e.target as Node)) {
        isOpen = false;
      }
    }

    const timer = setTimeout(() => {
      document.addEventListener('click', handleClickOutside);
    }, 50);

    return () => {
      clearTimeout(timer);
      document.removeEventListener('click', handleClickOutside);
    };
  });
</script>

<div class="relative" bind:this={menuEl}>
  <!-- Bouton profil -->
  <button
    type="button"
    class="group flex items-center gap-2 rounded-full border py-1 pl-1 pr-2.5
           transition backdrop-blur-md cursor-pointer
           bg-white border-(--c-bord) hover:border-(--c-bord2)
           dark:bg-neutral-900/60 dark:border-white/10 dark:hover:bg-neutral-900/75 dark:hover:border-white/20"
    aria-label={$t("profil.menu_aria")}
    onclick={toggleMenu}
  >
    <!-- Avatar -->
    <span class="relative grid place-items-center w-7.5 h-7.5 rounded-full overflow-hidden text-[11px] font-bold text-white"
          style="background: {profilColor};">
      {#if profil?.avatar}
        <img src={profil.avatar} alt={$t("profil.avatar_alt")} class="w-full h-full object-cover" draggable="false" />
      {:else}
        {profil?.name?.charAt(0)?.toUpperCase() ?? "?"}
      {/if}
    </span>

    <!-- Nom -->
    <span class="hidden lg:block text-sm font-medium text-neutral-900 dark:text-neutral-100 max-w-24 truncate">
      {profil?.name ?? $t("profil.fallback_name")}
    </span>

    <!-- Chevron -->
    <svg class="w-3.5 h-3.5 text-neutral-400 transition-transform duration-200 {isOpen ? 'rotate-180' : ''}"
         viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
      <path d="m6 9 6 6 6-6"/>
    </svg>
  </button>

  <!-- Dropdown menu -->
  {#if isOpen}
    <div
      class="absolute top-full right-0 mt-2 z-50 w-56"
      transition:scale={{ duration: 120, start: 0.97 }}
    >
      <div class="rounded-xl border overflow-hidden
                  bg-white/98 dark:bg-neutral-900/98 backdrop-blur-xl
                  border-neutral-200/60 dark:border-white/8
                  shadow-xl shadow-black/10 dark:shadow-black/40">

        <!-- Header profil -->
        <div class="px-4 py-3 border-b border-neutral-200/60 dark:border-white/6">
          <div class="flex items-center gap-2.5">
            <div class="w-8 h-8 rounded-full shrink-0 flex items-center justify-center text-[11px] font-bold text-white"
                 style="background: {profilColor};">
              {profil?.name?.charAt(0)?.toUpperCase() ?? "?"}
            </div>
            <div class="min-w-0">
              <p class="text-[13px] font-medium text-neutral-800 dark:text-neutral-200 truncate">
                {profil?.name ?? $t("profil.fallback_name")}
              </p>
              <p class="text-[10px] text-neutral-400 dark:text-neutral-500">
                {profil?.role === "admin" ? $t('profil.admin') : $t('profil.user')}
              </p>
            </div>
          </div>
        </div>

        <!-- Items -->
        <div class="py-1 px-1">
          <button
            class="w-full flex items-center gap-2.5 px-3 py-2 rounded-lg text-left cursor-pointer
                   text-[13px] text-neutral-700 dark:text-neutral-300
                   hover:bg-neutral-100/80 dark:hover:bg-white/5
                   transition-colors duration-100"
            onclick={openProfilSwitch}
          >
            <Icon icon="lucide:users" width="15" class="text-neutral-400 dark:text-neutral-500" />
            {$t('menu.change_profil')}
          </button>

          <button
            class="w-full flex items-center gap-2.5 px-3 py-2 rounded-lg text-left cursor-pointer
                   text-[13px] text-neutral-700 dark:text-neutral-300
                   hover:bg-neutral-100/80 dark:hover:bg-white/5
                   transition-colors duration-100"
            onclick={() => { isOpen = false; goto('/stats'); }}
          >
            <Icon icon="lucide:bar-chart-3" width="15" class="text-neutral-400 dark:text-neutral-500" />
            {$t('stats.title')}
          </button>

          <button
            class="w-full flex items-center gap-2.5 px-3 py-2 rounded-lg text-left cursor-pointer
                   text-[13px] text-neutral-700 dark:text-neutral-300
                   hover:bg-neutral-100/80 dark:hover:bg-white/5
                   transition-colors duration-100"
            onclick={openSettings}
          >
            <Icon icon="lucide:settings" width="15" class="text-neutral-400 dark:text-neutral-500" />
            {$t('menu.settings')}
          </button>

          <button
            class="w-full flex items-center gap-2.5 px-3 py-2 rounded-lg text-left cursor-pointer
                   text-[13px] text-neutral-700 dark:text-neutral-300
                   hover:bg-neutral-100/80 dark:hover:bg-white/5
                   transition-colors duration-100"
            onclick={openFeedback}
          >
            <Icon icon="lucide:message-circle" width="15" class="text-neutral-400 dark:text-neutral-500" />
            {$t('menu.feedback')}
          </button>

          <button
            class="w-full flex items-center gap-2.5 px-3 py-2 rounded-lg text-left cursor-pointer
                   text-[13px] text-neutral-700 dark:text-neutral-300
                   hover:bg-neutral-100/80 dark:hover:bg-white/5
                   transition-colors duration-100"
            onclick={openAbout}
          >
            <Icon icon="lucide:info" width="15" class="text-neutral-400 dark:text-neutral-500" />
            {$t('menu.about')}
          </button>
        </div>

        <!-- Séparateur -->
        <div class="h-px mx-3 bg-neutral-200/60 dark:bg-white/6"></div>

        <!-- Quitter -->
        <div class="py-1 px-1">
          <button
            class="w-full flex items-center gap-2.5 px-3 py-2 rounded-lg text-left cursor-pointer
                   text-[13px] text-red-500/70 dark:text-red-400/70
                   hover:bg-red-500/8 dark:hover:bg-red-500/10
                   hover:text-red-500 dark:hover:text-red-400
                   transition-colors duration-100"
            onclick={quitApp}
          >
            <Icon icon="lucide:log-out" width="15" />
            {$t('menu.quit')}
          </button>
        </div>
      </div>
    </div>
  {/if}
</div>
