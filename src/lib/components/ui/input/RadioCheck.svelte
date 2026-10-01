<script lang="ts">
  // Coche ronde des choix exclusifs illustrés.
  import Icon from "@iconify/svelte";

  let { checked }: { checked: boolean } = $props();
</script>

<span
  class="shrink-0 w-5 h-5 rounded-full flex items-center justify-center border-[1.5px]
         {checked ? 'bg-(--rg-g) border-(--rg-g) text-white dark:text-(--rg-on-g)' : 'border-(--rg-bd2) text-transparent'}"
  aria-hidden="true"
>
  <Icon icon="material-symbols:check-rounded" width="14" />
</span>
