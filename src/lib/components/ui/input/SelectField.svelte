<script lang="ts">
  // Liste déroulante native, habillée comme les réglages.
  import Icon from "@iconify/svelte";

  let {
    value,
    options,
    label,
    onchange,
  }: {
    value: string;
    options: { value: string; label: string }[];
    label: string;
    onchange: (value: string) => void;
  } = $props();
</script>

<div class="relative shrink-0">
  <select
    {value}
    aria-label={label}
    onchange={(e) => onchange(e.currentTarget.value)}
    class="h-9.5 pl-3.5 pr-9 rounded-[10px] appearance-none cursor-pointer
           bg-(--rg-s2) border border-(--rg-off) hover:border-(--rg-bd2)
           text-sm font-semibold text-(--rg-tx) transition-colors"
  >
    {#each options as option (option.value)}
      <option value={option.value}>{option.label}</option>
    {/each}
  </select>
  <Icon
    icon="material-symbols:expand-more-rounded"
    width="18"
    class="pointer-events-none absolute right-2.5 top-1/2 -translate-y-1/2 text-(--rg-mu)"
  />
</div>
