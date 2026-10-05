<script lang="ts">
    import Icon from "@iconify/svelte";
  import { toogleTheme } from "#lib/helper/theme/theme";

  let isDarkTheme = $state(true);
</script>

<button
  type="button"
  role="switch"
  aria-checked={isDarkTheme}
  aria-label={isDarkTheme ? "Activer le thème clair" : "Activer le thème sombre"}
  onclick={() => (isDarkTheme = toogleTheme(isDarkTheme))}
  class="inline-flex items-center rounded-full p-1 transition-colors
         bg-neutral-200 dark:bg-neutral-800"
>
  <span
    class="relative inline-flex h-7 w-12 items-center rounded-full transition-colors
           bg-neutral-300 dark:bg-neutral-700"
  >
    <span
      class={`absolute inline-flex h-6 w-6 items-center justify-center rounded-full bg-white shadow
              transition-transform
              ${isDarkTheme ? "translate-x-6" : "translate-x-1"}`}
    >
      {#if isDarkTheme}
        <Icon icon="tabler:moon" width="16" class="text-neutral-800"/>
      {:else}
        <Icon icon="tabler:sun" width="16" class="text-amber-500" />
      {/if}
    </span>
  </span>
</button>