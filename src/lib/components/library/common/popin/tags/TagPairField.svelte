<script lang="ts">
  // « n sur N » : un seul tag (« 7/12 »), donc deux saisies dans un seul bloc.
  import TagField from "#lib/components/ui/input/TagField.svelte";

  let {
    label,
    number = $bindable(),
    total = $bindable(),
    dirty,
    disabled,
  }: {
    label: string;
    number?: string;
    total?: string;
    dirty: boolean;
    disabled: boolean;
  } = $props();
</script>

<TagField {label} {dirty}>
  {#snippet children()}
    <div class="flex items-baseline gap-1.5 mt-1.5">
      <input
        type="text"
        inputmode="numeric"
        data-focus-ring="none"
        bind:value={number}
        {disabled}
        placeholder="—"
        class="w-7 bg-transparent text-[13px] tabular-nums leading-tight text-right outline-none
               text-neutral-900 dark:text-neutral-50
               placeholder:text-neutral-300 dark:placeholder:text-neutral-700
               disabled:opacity-50"
      />
      <span class="text-neutral-300 dark:text-neutral-600 select-none text-[13px]">/</span>
      <input
        type="text"
        inputmode="numeric"
        data-focus-ring="none"
        bind:value={total}
        {disabled}
        placeholder="—"
        class="w-7 bg-transparent text-[13px] tabular-nums leading-tight outline-none
               text-neutral-500 dark:text-neutral-400
               placeholder:text-neutral-300 dark:placeholder:text-neutral-700
               disabled:opacity-50"
      />
    </div>
  {/snippet}
</TagField>
