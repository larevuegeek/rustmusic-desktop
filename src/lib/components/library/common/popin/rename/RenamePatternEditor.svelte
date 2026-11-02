<script lang="ts">
  // Le motif : mode, destination, saisie, champs et motifs prédéfinis.
  import Icon from "@iconify/svelte";
  import { fade } from "svelte/transition";
  import { t } from "#lib/i18n";
  import {
    PATTERN_FIELDS,
    PRESETS,
    TREE_PRESETS,
    type LibraryDir,
  } from "#lib/services/tags/rename.service";

  let {
    restructure = $bindable(),
    root = $bindable(),
    satellites = $bindable(),
    cleanup = $bindable(),
    dirs,
    pattern,
    patternError,
    onpatternchange,
  }: {
    restructure: boolean;
    root: string;
    satellites: boolean;
    cleanup: boolean;
    dirs: LibraryDir[];
    pattern: string;
    patternError: string | null;
    onpatternchange: (value: string) => void;
  } = $props();

  let presets = $derived(restructure ? TREE_PRESETS : PRESETS);
  let input: HTMLInputElement | null = $state(null);

  /** Insère un champ à la position du curseur. */
  function insert(field: string) {
    const token = `{${field}}`;
    if (!input) {
      onpatternchange(pattern + token);
      return;
    }
    const start = input.selectionStart ?? pattern.length;
    const end = input.selectionEnd ?? start;
    onpatternchange(pattern.slice(0, start) + token + pattern.slice(end));
    // Curseur après le champ : on enchaîne les insertions sans la souris.
    queueMicrotask(() => {
      input?.focus();
      input?.setSelectionRange(start + token.length, start + token.length);
    });
  }
</script>

<div class="shrink-0 px-4 py-3 space-y-2.5
            border-b border-(--rg-bd) bg-(--rg-creux)">
  <!-- Deux modes : l'un touche au nom, l'autre range l'arborescence. -->
  <div class="flex gap-1">
    {#each [[false, "rename.mode_name"], [true, "rename.mode_tree"]] as [value, key] (key)}
      <button
        type="button"
        onclick={() => (restructure = value as boolean)}
        class="px-3 h-8 rounded-xl border text-[12px] font-semibold cursor-pointer transition-colors
               {restructure === value
                 ? 'bg-(--rg-gbg) border-(--rg-gbd) text-(--rg-gtx)'
                 : 'border-transparent text-(--rg-tx2) hover:bg-(--rg-hover) hover:text-(--rg-tx)'}"
      >
        {$t(key as string)}
      </button>
    {/each}

    {#if restructure}
      <span class="flex-1"></span>
      <label class="flex items-center gap-2 text-[11px] font-bold uppercase tracking-[0.1em] text-(--rg-mu)">
        {$t("rename.destination")}
        <select
          bind:value={root}
          class="h-8 px-2.5 rounded-xl text-[12px] font-normal normal-case tracking-normal outline-none cursor-pointer transition-colors
                 bg-(--rg-champ) text-(--rg-tx) border border-(--rg-bd)
                 hover:border-(--rg-bd2) rg-field"
        >
          {#each dirs as dir (dir.id)}
            <option value={dir.path}>{dir.path}</option>
          {/each}
        </select>
      </label>
    {/if}
  </div>

  <div class="flex gap-1.5">
    <input
      bind:this={input}
      type="text"
      value={pattern}
      oninput={(e) => onpatternchange(e.currentTarget.value)}
      spellcheck="false"
      placeholder={PRESETS[0].pattern}
      class="flex-1 min-w-0 h-10 px-3.5 rounded-xl font-mono text-[13px] outline-none transition-colors
             bg-(--rg-champ) text-(--rg-tx) placeholder:text-(--rg-ph) border
             {patternError
               ? 'border-red-500/60 focus:border-red-500'
               : 'border-(--rg-bd) hover:border-(--rg-bd2) rg-field'}"
    />
  </div>

  {#if patternError}
    <p class="flex items-start gap-1.5 text-[12px] text-red-600 dark:text-red-400"
       transition:fade={{ duration: 120 }}>
      <Icon icon="lucide:alert-triangle" width="12" class="shrink-0 mt-0.5" />
      <span class="min-w-0">{patternError}</span>
    </p>
  {/if}

  <!-- Champs cliquables : retapés à la main, une faute ne se voit qu'à l'erreur. -->
  <div class="flex flex-wrap items-center gap-1">
    <span class="mr-1 text-[11px] font-bold uppercase tracking-[0.1em] text-(--rg-mu)">
      {$t("rename.fields")}
    </span>
    {#each PATTERN_FIELDS as field (field)}
      <button
        type="button"
        onclick={() => insert(field)}
        class="px-1.5 h-6 rounded-md border font-mono text-[11px] cursor-pointer
               transition-colors
               bg-(--rg-carte) border-(--rg-bd) text-(--rg-tx2)
               hover:bg-(--rg-gbg) hover:border-(--rg-gbd) hover:text-(--rg-gtx)"
      >
        {field}
      </button>
    {/each}
  </div>

  <div class="flex flex-wrap items-center gap-1">
    <span class="mr-1 text-[11px] font-bold uppercase tracking-[0.1em] text-(--rg-mu)">
      {$t("rename.presets")}
    </span>
    {#each presets as preset (preset.pattern)}
      <button
        type="button"
        onclick={() => onpatternchange(preset.pattern)}
        class="px-2 h-6 rounded-md border text-[11px] font-semibold cursor-pointer transition-colors
               {pattern === preset.pattern
                 ? 'bg-(--rg-gbg) border-(--rg-gbd) text-(--rg-gtx)'
                 : 'bg-(--rg-carte) border-(--rg-bd) text-(--rg-tx2) hover:border-(--rg-bd2) hover:text-(--rg-tx)'}"
      >
        {preset.label}
      </button>
    {/each}
  </div>

  {#if restructure}
    <!-- Ce qu'on regrette de n'avoir pas coché : paroles laissées, dossiers vides. -->
    <div class="flex flex-wrap items-center gap-4 pt-0.5">
      <label class="flex items-center gap-2 cursor-pointer">
        <input type="checkbox" bind:checked={satellites} class="checkbox-app" />
        <span class="text-[12px] text-(--rg-tx2)">
          {$t("rename.satellites")}
        </span>
      </label>
      <label class="flex items-center gap-2 cursor-pointer">
        <input type="checkbox" bind:checked={cleanup} class="checkbox-app" />
        <span class="text-[12px] text-(--rg-tx2)">
          {$t("rename.cleanup")}
        </span>
      </label>
    </div>
  {/if}
</div>
