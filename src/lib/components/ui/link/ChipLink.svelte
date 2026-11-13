<script lang="ts">
  // Pastille-lien à contour (genres d'un album ou d'un artiste) ; `icon` facultative devant le libellé.
  import Icon from "@iconify/svelte";

  let { href, label, title, icon }: { href: string; label: string; title?: string; icon?: string } = $props();
</script>

<a {href} {title}
   class="h-7 px-3 flex items-center gap-1.5 whitespace-nowrap rounded-full border text-[12.5px] font-semibold transition-colors border-(--rg-bd2) text-(--rg-tx2) hover:text-(--rg-tx) hover:border-(--rg-mu2)
          {icon ? 'pl-2.5' : ''}">{#if icon}<Icon {icon} width="15" class="shrink-0" />{/if}{label}</a>
