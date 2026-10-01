<script lang="ts">
  // Bande d'erreur posée au-dessus du pied de page : dedans, un long message repousserait les boutons.
  import Icon from "@iconify/svelte";
  import { fade } from "svelte/transition";

  let { message }: { message: string } = $props();
</script>

<div
  class="shrink-0 flex items-start gap-2 px-5 py-2
         bg-red-500/10 border-t border-red-500/20 text-red-500 text-[11px]"
  transition:fade={{ duration: 120 }}
>
  <Icon icon="lucide:alert-triangle" width="13" class="shrink-0 mt-0.5" />
  <span class="min-w-0">{message}</span>
</div>
