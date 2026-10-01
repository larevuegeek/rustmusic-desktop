<script lang="ts">
  // Lecture : pilule verte avec libellé (`lg`), ou rond (`md`) ; `muted` l'éteint jusqu'au survol.
  import Icon from "@iconify/svelte";

  let { label, title, size = "lg", muted = false, disabled = false, onclick }: {
    label?: string;
    title?: string;
    size?: "lg" | "md";
    muted?: boolean;
    disabled?: boolean;
    onclick: (e: MouseEvent) => void;
  } = $props();
</script>

{#if size === "lg"}
  <button type="button" {title} {disabled} {onclick}
          class="h-12 pl-4.5 pr-6 flex items-center gap-2 rounded-full cursor-pointer transition-transform hover:scale-[1.02]
                 disabled:opacity-50 disabled:cursor-default disabled:hover:scale-100 bg-(--rg-g) text-(--rg-on-g) text-[15px] font-extrabold">
    <Icon icon="material-symbols:play-arrow-rounded" width="26" />
    {label}
  </button>
{:else}
  <button type="button" title={title ?? label} aria-label={title ?? label} {disabled} {onclick}
          class="w-9.5 h-9.5 shrink-0 rounded-full flex items-center justify-center cursor-pointer transition-colors
                 {muted ? 'bg-(--rg-s2) text-(--rg-tx2) hover:bg-(--rg-g) hover:text-(--rg-on-g)' : 'bg-(--rg-g) text-(--rg-on-g) hover:brightness-110'}">
    <Icon icon="material-symbols:play-arrow-rounded" width="24" />
  </button>
{/if}
