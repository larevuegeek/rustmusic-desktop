<script lang="ts">
  import Icon from "@iconify/svelte";
  import { goto } from "$app/navigation";
  import { t } from "$lib/i18n";
  import { portal } from "$lib/helper/portal";
  import { settingsStore } from "$lib/stores/settings/settings.store";

  /** Les options de la section « Ouvrir », sous son bouton « … ». */
  let { x, y, onclose }: { x: number; y: number; onclose: () => void } = $props();

  let menuStyle = $derived.by(() => {
    const largeur = 264;
    const hauteur = 132;
    const posX = Math.max(8, Math.min(x - largeur + 16, window.innerWidth - largeur - 8));
    const posY = y + hauteur > window.innerHeight ? Math.max(8, y - hauteur) : y;
    return `left: ${posX}px; top: ${posY}px;`;
  });

  const ligne = "w-full flex items-center gap-2.5 px-2 py-2 rounded-xl cursor-pointer text-left transition-colors hover:bg-neutral-100 dark:hover:bg-white/5";
</script>

<div use:portal>
<!-- svelte-ignore a11y_no_static_element_interactions -->
<button
  type="button"
  class="fixed inset-0 z-9998 cursor-default"
  onclick={onclose}
  oncontextmenu={(e) => { e.preventDefault(); onclose(); }}
  aria-label="Fermer le menu"
></button>

<div
  role="menu"
  tabindex="-1"
  oncontextmenu={(e) => e.preventDefault()}
  class="fixed z-[9999] w-66 p-1.5
         bg-white/95 dark:bg-neutral-950/95 backdrop-blur-xl
         border border-neutral-200/80 dark:border-white/10
         rounded-2xl overflow-hidden
         shadow-xl shadow-black/10 dark:shadow-2xl dark:shadow-black/50"
  style={menuStyle}
>
  <button
    class={ligne}
    onclick={() => { settingsStore.set('show_open_buttons', 'false'); onclose(); }}
  >
    <span class="w-7 h-7 rounded-lg shrink-0 flex items-center justify-center
                 bg-neutral-100 dark:bg-white/5 border border-neutral-200/80 dark:border-white/8">
      <Icon icon="lucide:eye-off" width="13" class="text-neutral-500 dark:text-neutral-400" />
    </span>
    <span class="flex-1 text-[13px] text-neutral-700 dark:text-neutral-200">
      {$t('sidebar.hide_section')}
    </span>
  </button>

  <!-- Masquée, la section emporte son propre menu : on dit donc où la
       retrouver avant qu'elle ne disparaisse. -->
  <p class="px-2.5 pt-1 pb-2 text-[11px] leading-snug text-neutral-400 dark:text-neutral-500">
    {$t('sidebar.hide_section_note')}
  </p>

  <div class="h-px mx-2 mb-1.5 bg-neutral-200/80 dark:bg-white/8"></div>

  <button class={ligne} onclick={() => { onclose(); goto('/settings/appearance'); }}>
    <span class="w-7 h-7 rounded-lg shrink-0 flex items-center justify-center
                 bg-neutral-100 dark:bg-white/5 border border-neutral-200/80 dark:border-white/8">
      <Icon icon="lucide:sliders-horizontal" width="13" class="text-neutral-500 dark:text-neutral-400" />
    </span>
    <span class="flex-1 text-[13px] text-neutral-700 dark:text-neutral-200">
      {$t('playlist.display_settings')}
    </span>
    <Icon icon="lucide:chevron-right" width="13" class="text-neutral-300 dark:text-neutral-600 shrink-0" />
  </button>
</div>
</div>
