<script lang="ts">
  // Veto de fermeture tant qu'il reste des modifications, et la bande qui laisse le choix.
  // À garder monté pendant toute la vie de la popin : c'est lui qui porte le veto.
  import ConfirmBand from "#lib/components/ui/feedback/ConfirmBand.svelte";
  import { popinStore } from "#lib/stores/ui/popin.store";
  import { t } from "#lib/i18n";

  let {
    dirty,
    busy,
    confirming = $bindable(false),
  }: {
    dirty: boolean;
    /** Enregistrement en cours : la fermeture passe sans veto. */
    busy: boolean;
    /** Vrai quand une fermeture a été refusée. */
    confirming?: boolean;
  } = $props();

  // `dirty` n'est lu qu'à l'appel du veto : l'effet ne tourne qu'une fois et
  // sa fonction de retour désinscrit le veto.
  $effect(() =>
    popinStore.guard(() => {
      if (!dirty || busy) return true;
      confirming = true;
      return false;
    }),
  );
</script>

<!-- Le bouton par défaut est celui qui ne détruit rien. -->
<ConfirmBand open={confirming && dirty} cancelLabel={$t('tags.discard')} confirmLabel={$t('tags.keep_editing')}
             oncancel={() => popinStore.close()} onconfirm={() => (confirming = false)}>
  {$t('tags.unsaved')}
</ConfirmBand>
