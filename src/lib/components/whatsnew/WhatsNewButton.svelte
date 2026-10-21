<script lang="ts">
  // Mégaphone de la barre du haut : les nouveautés de la version, pastille tant qu'on ne les a pas vues.
  import Icon from "@iconify/svelte";
  import { t } from "#lib/i18n";
  import { whatsNew } from "#lib/stores/ui/whatsNew.store";
  import { openWhatsNew } from "#lib/actions/whatsNew/WhatsNewAction";
</script>

{#if whatsNew.hasSlides}
  <button type="button" onclick={openWhatsNew} aria-label={$t("whats_new.title")} title={$t("whats_new.title")}
          class="relative w-8 h-8 flex items-center justify-center rounded-full cursor-pointer transition-colors
                 text-neutral-500 dark:text-neutral-400 hover:text-neutral-700 dark:hover:text-neutral-200">
    <Icon icon="material-symbols:campaign-outline-rounded" width="22" />
    {#if !$whatsNew.viewed}
      <span class="absolute top-1 right-1 w-2 h-2 rounded-full bg-violet-500 ring-2 ring-(--c-fond) dark:ring-zinc-950" aria-hidden="true"></span>
    {/if}
  </button>
{/if}
