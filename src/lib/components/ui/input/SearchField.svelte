<script lang="ts">
  // Champ de filtre avec loupe et bouton d'effacement.
  import Icon from "@iconify/svelte";

  let { value = $bindable(""), placeholder, clearLabel, class: classes = "" }: {
    value?: string;
    placeholder: string;
    clearLabel: string;
    class?: string;
  } = $props();
</script>

<label class="h-9.5 flex items-center gap-2 px-3 rounded-[10px] border bg-(--rg-carte) border-(--rg-bd) focus-within:border-(--rg-bd2) {classes}">
  <Icon icon="material-symbols:search-rounded" width="18" class="shrink-0 text-(--rg-mu)" />
  <input bind:value class="flex-1 min-w-0 bg-transparent border-0 outline-0 text-sm text-(--rg-tx) placeholder:text-(--rg-ph)" {placeholder} />
  {#if value}
    <button type="button" class="shrink-0 text-(--rg-mu) hover:text-(--rg-tx) cursor-pointer" aria-label={clearLabel} title={clearLabel} onclick={() => (value = "")}>
      <Icon icon="material-symbols:close-rounded" width="16" />
    </button>
  {/if}
</label>
