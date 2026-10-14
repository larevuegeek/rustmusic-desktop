<script lang="ts">
  // Action riche d'un menu : tuile d'icône, titre et une ligne qui dit ce qu'elle fait.
  import Icon from "@iconify/svelte";

  let {
    icon,
    title,
    description,
    disabled = false,
    onclick,
  }: {
    icon: string;
    title: string;
    description: string;
    disabled?: boolean;
    onclick: () => void;
  } = $props();
</script>

<button type="button" role="menuitem" {disabled} {onclick}
        class="group/a w-full flex items-center gap-3 p-2 rounded-[10px] text-left cursor-pointer transition-colors
               hover:bg-(--rg-s2) dark:hover:bg-white/5 disabled:cursor-default disabled:hover:bg-transparent">
  <span class="w-8.5 h-8.5 shrink-0 rounded-[9px] border flex items-center justify-center transition-colors
               bg-(--rg-s2) border-(--rg-bd) dark:bg-[#232a26] dark:border-[#2b332f]
               group-enabled/a:group-hover/a:bg-(--rg-gbg) group-enabled/a:group-hover/a:border-(--rg-gbd)">
    <Icon {icon} width="18" class="text-(--rg-tx2) group-enabled/a:group-hover/a:text-(--rg-g) group-disabled/a:text-(--rg-mu2)" />
  </span>
  <span class="flex-1 min-w-0">
    <span class="block text-[13px] font-semibold text-(--rg-tx) group-disabled/a:text-(--rg-mu)">{title}</span>
    <span class="block mt-0.5 text-[11.5px] leading-snug text-(--rg-mu)">{description}</span>
  </span>
</button>
