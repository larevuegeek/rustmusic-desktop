<script lang="ts">
  // Pastille-lien à contour (genres d'un album ou d'un artiste).
  let { href, label, title }: { href: string; label: string; title?: string } = $props();
</script>

<a {href} {title}
   class="h-7 px-3 flex items-center whitespace-nowrap rounded-full border text-[12.5px] font-semibold transition-colors border-(--rg-bd2) text-(--rg-tx2) hover:text-(--rg-tx) hover:border-(--rg-mu2)">{label}</a>
