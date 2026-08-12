<script lang="ts">
  import Icon from "@iconify/svelte";
  import { fade, scale } from "svelte/transition";
  import { popinStore, type PopinSize } from "$lib/stores/ui/popin.store";

  // Un clic sur le fond ferme la popin — mais un clic « commencé » dans le
  // panneau ne doit PAS fermer. C'est le cas quand on sélectionne du texte et
  // qu'on relâche la souris en dehors : le navigateur émet alors un `click`
  // dont la cible est l'ancêtre commun, c'est-à-dire le fond. Sans cette
  // garde, sélectionner une valeur pour la copier referme la fenêtre.
  let pressedOnBackdrop = false;

  // Classes littérales : Tailwind lit le source, une largeur composée
  // (`max-w-${size}`) ne serait jamais générée.
  const WIDTHS: Record<PopinSize, string> = {
    md: "max-w-md",
    lg: "max-w-2xl",
    xl: "max-w-3xl",
  };

  let width = $derived(WIDTHS[$popinStore.size] ?? WIDTHS.md);

  function onBackdropPointerDown(e: MouseEvent) {
    pressedOnBackdrop = e.target === e.currentTarget;
  }

  function onBackdropClick(e: MouseEvent) {
    if (pressedOnBackdrop && e.target === e.currentTarget) {
      popinStore.requestClose();
    }
    pressedOnBackdrop = false;
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Escape" && $popinStore.isOpen) popinStore.requestClose();
  }
</script>

<svelte:window onkeydown={onKeydown} />

{#if $popinStore.isOpen}
  <div
    role="presentation"
    class="fixed inset-0 z-50 flex items-center justify-center
           bg-black/60 backdrop-blur-sm"
    transition:fade={{ duration: 150 }}
    onmousedown={onBackdropPointerDown}
    onclick={onBackdropClick}
  >
    <!-- Colonne : l'en-tête ne défile jamais, et le corps peut confier son
         propre défilement au composant (mode `flush`). -->
    <div
      role="dialog"
      aria-modal="true"
      aria-label={$popinStore.title}
      class="flex flex-col w-full {width} mx-4 max-h-[88vh]
             rounded-2xl overflow-hidden
             bg-white dark:bg-neutral-900
             ring-1 ring-black/5 dark:ring-white/10
             shadow-2xl shadow-black/20 dark:shadow-[0_30px_80px_-20px_rgba(0,0,0,0.9)]"
      transition:scale={{ duration: 180, start: 0.96 }}
    >
      <div
        class="shrink-0 px-6 pt-5 pb-3
               border-b border-neutral-200/70 dark:border-white/8"
      >
        <div class="flex items-center justify-between gap-3">
          <h3 class="flex items-center gap-2.5 min-w-0
                     text-base font-semibold tracking-tight text-neutral-900 dark:text-white">
            {#if $popinStore.icon}
              <!-- Pastille plutôt qu'icône nue : à côté d'un titre en gras une
                   icône seule paraît orpheline, le fond lui donne son assise. -->
              <span
                class="shrink-0 w-7 h-7 rounded-lg flex items-center justify-center
                       bg-emerald-500/12 text-emerald-600 dark:text-emerald-400
                       ring-1 ring-emerald-500/20"
              >
                <Icon icon={$popinStore.icon} width="15" />
              </span>
            {/if}
            <span class="truncate">{$popinStore.title}</span>
          </h3>
          <button
            type="button"
            class="shrink-0 -mr-1.5 w-7 h-7 rounded-lg flex items-center justify-center
                   cursor-pointer text-neutral-400 dark:text-neutral-500
                   hover:text-neutral-800 dark:hover:text-neutral-200
                   hover:bg-neutral-100 dark:hover:bg-white/8 transition-colors"
            aria-label="Fermer"
            onclick={() => popinStore.close()}
          >
            <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round">
              <path d="M18 6 6 18M6 6l12 12" />
            </svg>
          </button>
        </div>
      </div>

      <div
        class="flex-1 min-h-0 {$popinStore.flush
          ? 'flex flex-col'
          : 'overflow-y-auto scrollbar-none px-6 pb-6 pt-4'}"
      >
        {#if $popinStore.component}
          {@const Content = $popinStore.component}
          <Content {...$popinStore.props} />
        {/if}
      </div>
    </div>
  </div>
{/if}
