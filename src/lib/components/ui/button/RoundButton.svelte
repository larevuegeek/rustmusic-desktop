<script lang="ts">
  // Bouton rond à contour (actions d'en-tête) ; `pressed` le passe en vert.
  import Icon from "@iconify/svelte";

  let { icon, title, pressed, disabled = false, onclick }: {
    icon: string;
    title: string;
    pressed?: boolean;
    disabled?: boolean;
    onclick: (e: MouseEvent) => void;
  } = $props();
</script>

<button type="button" aria-pressed={pressed} {title} aria-label={title} {disabled} {onclick}
        class="w-12 h-12 shrink-0 rounded-full border flex items-center justify-center cursor-pointer transition-colors disabled:opacity-50 disabled:cursor-default
               {pressed ? 'border-(--rg-gbd) text-(--rg-gtx) bg-(--rg-gbg)' : 'border-(--rg-bd2) text-(--rg-tx2) bg-white/40 dark:bg-black/25 hover:text-(--rg-tx) hover:bg-(--rg-carte)'}">
  <Icon {icon} width="22" />
</button>
