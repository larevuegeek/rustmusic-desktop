<script lang="ts">
  import { popinStore } from "#lib/stores/ui/popin.store";

  let {
    message = "",
    confirmText = "Confirmer",
    cancelText = "Annuler",
    onConfirm = () => {},
    onCancel = () => {},
    variant = "primary"
  }: {
    message?: string;
    confirmText?: string;
    cancelText?: string;
    onConfirm?: (() => void) | (() => Promise<void>);
    onCancel?: (() => void) | (() => Promise<void>);
    variant?: "danger" | "primary";
  } = $props();

  let isLoading = $state(false);
  
  async function handleConfirm() {
    isLoading = true;
    try {
      await onConfirm();
      popinStore.close();
    } catch (error) {
      console.error("Erreur lors de la confirmation:", error);
    } finally {
      isLoading = false;
    }
  }
  
  async function handleCancel() {
    isLoading = true;
    try {
      await onCancel();
      popinStore.close();
    } catch (error) {
      console.error("Erreur lors de l'annulation:", error);
    } finally {
      isLoading = false;
    }
  }
</script>

<div class="space-y-6">
  <!-- Message -->
  <p class="text-sm text-neutral-600 dark:text-neutral-400">
    {message}
  </p>
  
  <!-- Actions -->
  <div class="flex gap-3 justify-end">
    <button
      type="button"
      class="px-4 py-2 text-sm font-medium rounded-md
             text-neutral-700 bg-neutral-100 
             hover:bg-neutral-200
             dark:text-neutral-300 dark:bg-neutral-800 
             dark:hover:bg-neutral-700
             transition-colors cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed"
      onclick={handleCancel}
      disabled={isLoading}
    >
      {cancelText}
    </button>
    
    <button
      type="button"
      class="px-4 py-2 text-sm font-medium rounded-md text-white
             transition-colors cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed
             {variant === 'danger'
               ? 'bg-red-600 hover:bg-red-700 dark:bg-red-600 dark:hover:bg-red-700'
               : 'bg-blue-600 hover:bg-blue-700 dark:bg-blue-600 dark:hover:bg-blue-700'}"
      onclick={handleConfirm}
      disabled={isLoading}
    >
      {#if isLoading}
        <span class="inline-block w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin"></span>
      {:else}
        {confirmText}
      {/if}
    </button>
  </div>
</div>