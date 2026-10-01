<script lang="ts">
  // État d'une page entière : chargement (roue) ou message d'erreur avec lien de retour.
  import Icon from "@iconify/svelte";

  let { chargement = false, icon = "material-symbols:error-outline-rounded", message, lien = null }: {
    chargement?: boolean;
    icon?: string;
    /** Message d'erreur, ou texte lu par les lecteurs d'écran pendant le chargement. */
    message: string;
    lien?: { href: string; label: string } | null;
  } = $props();
</script>

{#if chargement}
  <div class="flex items-center justify-center h-full text-(--rg-mu)">
    <Icon icon="material-symbols:progress-activity" width="26" class="animate-spin" />
    <span class="sr-only">{message}</span>
  </div>
{:else}
  <div class="flex flex-col items-center justify-center gap-3 h-full text-(--rg-mu)">
    <Icon {icon} width="34" />
    <p class="text-sm">{message}</p>
    {#if lien}<a href={lien.href} class="text-sm font-semibold text-(--rg-gtx) hover:underline">{lien.label}</a>{/if}
  </div>
{/if}
