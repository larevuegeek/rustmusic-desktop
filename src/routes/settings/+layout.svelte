<script lang="ts">
  // Layout commun des réglages : sidebar de navigation + header sticky.
  // Chaque section est une sous-route (/settings/general, /settings/audio…),
  // la section active est déduite de l'URL.
  import Icon from "@iconify/svelte";
  import { page } from "$app/state";
  import { t } from "$lib/i18n";
  import type { Snippet } from "svelte";
    import { goto } from "$app/navigation";

  let { children }: { children: Snippet } = $props();

  type SectionMeta = {
    id: string;
    icon: string;
    labelKey: string;
    descKey: string;
    group: 'preferences' | 'app';
  };
  const sections: SectionMeta[] = [
    { id: 'general',    icon: 'lucide:settings',   labelKey: 'settings.general',    descKey: 'settings.general_desc',    group: 'preferences' },
    { id: 'appearance', icon: 'lucide:paintbrush', labelKey: 'settings.appearance', descKey: 'settings.appearance_desc', group: 'preferences' },
    { id: 'audio',      icon: 'lucide:speaker',    labelKey: 'settings.audio',      descKey: 'settings.audio_desc',      group: 'preferences' },
    { id: 'network',    icon: 'lucide:network',    labelKey: 'settings.network',    descKey: 'settings.network_desc',    group: 'preferences' },
    { id: 'storage',    icon: 'lucide:hard-drive', labelKey: 'settings.storage',    descKey: 'settings.storage_desc',    group: 'app' },
    { id: 'about',      icon: 'lucide:info',       labelKey: 'settings.about',      descKey: 'settings.about_desc',      group: 'app' },
  ];

  let activeSection = $derived(page.url.pathname.split('/').pop() ?? 'general');
  let activeSectionMeta = $derived(sections.find((s) => s.id === activeSection) ?? sections[0]);
</script>

{#snippet navGroup(group: 'preferences' | 'app', groupLabelKey: string)}
  <div class="space-y-0.5">
    <p class="px-3 mb-2 text-[9px] font-semibold uppercase tracking-widest text-neutral-400 dark:text-neutral-500">
      {$t(groupLabelKey)}
    </p>
    {#each sections.filter((s) => s.group === group) as section}
      {@const isActive = activeSection === section.id}
      <a
        href="/settings/{section.id}"
        class="relative w-full flex items-center gap-3 pl-3.5 pr-3 py-2.5 rounded-lg text-sm cursor-pointer
               transition-all text-left
               {isActive
                 ? 'bg-emerald-500/10 text-emerald-700 dark:text-emerald-400 font-medium shadow-sm shadow-emerald-500/5'
                 : 'text-neutral-600 dark:text-neutral-300 hover:bg-white dark:hover:bg-white/5 hover:text-neutral-900 dark:hover:text-neutral-100'}"
      >
        {#if isActive}
          <span class="absolute left-0 top-1/2 -translate-y-1/2 w-1 h-5 rounded-r-full bg-emerald-500"></span>
        {/if}
        <Icon
          icon={section.icon}
          width="16"
          class={isActive ? '' : 'text-neutral-400 dark:text-neutral-500'}
        />
        <span>{$t(section.labelKey)}</span>
      </a>
    {/each}
  </div>
{/snippet}

<div class="flex h-full">

  <!-- ═══ SIDEBAR ═══ -->
  <aside class="shrink-0 w-64 border-r border-neutral-200/70 dark:border-white/8
                bg-linear-to-b from-neutral-50/80 to-neutral-100/40
                dark:from-white/2 dark:to-transparent
                overflow-y-auto scrollbar-app flex flex-col">
    <!-- Header sidebar -->
    <div class="px-5 pt-7 pb-6 flex items-center gap-3">
      <button
        class="shrink-0 w-9 h-9 flex items-center justify-center rounded-lg cursor-pointer
               text-neutral-500 hover:text-neutral-800 dark:text-neutral-400 dark:hover:text-neutral-100
               bg-white/60 dark:bg-white/5 border border-neutral-200/70 dark:border-white/10
               hover:bg-white dark:hover:bg-white/10
               shadow-sm shadow-black/5 transition-all"
        onclick={() => goto('/')}
        aria-label="Retour"
        title="Retour"
      >
        <Icon icon="lucide:arrow-left" width="15" />
      </button>
      <div class="min-w-0">
        <h1 class="text-[15px] font-bold tracking-tight text-neutral-900 dark:text-neutral-50 leading-tight">
          {$t('settings.title')}
        </h1>
        <p class="text-[10px] text-neutral-400 dark:text-neutral-500 mt-0.5 tracking-wide">
          RustMusic
        </p>
      </div>
    </div>

    <!-- Nav sections -->
    <nav class="flex-1 px-3 pb-5 space-y-6">
      {@render navGroup('preferences', 'settings.group_preferences')}
      {@render navGroup('app', 'settings.group_app')}
    </nav>

    <!-- Footer sidebar : version -->
    <div class="px-5 py-4 border-t border-neutral-200/60 dark:border-white/5
                text-[10px] text-neutral-400 dark:text-neutral-500 flex items-center gap-1.5">
      <Icon icon="lucide:tag" width="10" />
      <span>v{__APP_VERSION__}</span>
    </div>
  </aside>

  <!-- ═══ CONTENU ═══ -->
  <div class="flex-1 overflow-y-auto scrollbar-app">

    <!-- Page header sticky -->
    <div class="sticky top-0 z-10 backdrop-blur-md
                bg-white/70 dark:bg-neutral-950/70
                border-b border-neutral-200/60 dark:border-white/5
                px-8 md:px-12 py-6">
      <div class="max-w-3xl">
        <h1 class="text-2xl font-bold tracking-tight text-neutral-900 dark:text-neutral-50">
          {$t(activeSectionMeta.labelKey)}
        </h1>
        <p class="text-[13px] text-neutral-500 dark:text-neutral-400 mt-1">
          {$t(activeSectionMeta.descKey)}
        </p>
      </div>
    </div>

    <div class="px-8 md:px-12 py-8 max-w-3xl">
      {@render children()}
    </div>
  </div>
</div>
