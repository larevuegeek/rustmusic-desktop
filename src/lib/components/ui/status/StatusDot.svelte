<script lang="ts">
  // Voyant d'état, posé dans le texte : vert (joignable), rouge (injoignable) ; rien tant que ce n'est pas vérifié.
  import { t } from "#lib/i18n";

  let { online, class: classe = "" }: { online: boolean | undefined; class?: string } = $props();
</script>

{#if online !== undefined}
  <span
    role="img"
    class="inline-block shrink-0 w-1.5 h-1.5 rounded-full align-middle
           {online ? 'bg-emerald-500 shadow-[0_0_5px_rgb(16_185_129/0.7)]' : 'bg-red-500 shadow-[0_0_5px_rgb(239_68_68/0.7)]'} {classe}"
    title={online ? $t("servers.online") : $t("servers.offline")}
    aria-label={online ? $t("servers.online") : $t("servers.offline")}
  ></span>
{/if}
