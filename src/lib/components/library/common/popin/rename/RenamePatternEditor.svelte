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
            border-b border-neutral-200/70 dark:border-white/8
            bg-neutral-50/70 dark:bg-black/20">
  <!-- Deux modes : l'un touche au nom, l'autre range l'arborescence. -->
  <div class="flex gap-1">
    {#each [[false, "rename.mode_name"], [true, "rename.mode_tree"]] as [value, key] (key)}
      <button
        type="button"
        onclick={() => (restructure = value as boolean)}
        class="px-2.5 h-7 rounded-lg text-[11.5px] cursor-pointer transition-colors
               {restructure === value
                 ? 'bg-emerald-500/15 text-emerald-600 dark:text-emerald-400 font-medium'
                 : 'text-neutral-500 dark:text-neutral-400 hover:bg-neutral-200/70 dark:hover:bg-white/8'}"
      >
        {$t(key as string)}
      </button>
    {/each}

    {#if restructure}
      <span class="flex-1"></span>
      <label class="flex items-center gap-1.5 text-[11px]
                    text-neutral-500 dark:text-neutral-400">
        {$t("rename.destination")}
        <select
          bind:value={root}
          class="h-7 px-2 rounded-lg text-[11.5px] outline-none cursor-pointer
                 bg-white dark:bg-neutral-800
                 ring-1 ring-inset ring-neutral-200 dark:ring-white/10
                 text-neutral-800 dark:text-neutral-100"
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
      class="flex-1 min-w-0 h-9 px-3 rounded-lg font-mono text-[12.5px] outline-none
             bg-white dark:bg-white/5
             ring-1 ring-inset
             {patternError
               ? 'ring-red-500/60'
               : 'ring-neutral-200 dark:ring-white/10 focus:ring-emerald-500/60'}
             text-neutral-900 dark:text-neutral-100
             placeholder:text-neutral-300 dark:placeholder:text-neutral-700"
    />
  </div>

  {#if patternError}
    <p class="flex items-start gap-1.5 text-[11.5px] text-red-500"
       transition:fade={{ duration: 120 }}>
      <Icon icon="lucide:alert-triangle" width="12" class="shrink-0 mt-0.5" />
      <span class="min-w-0">{patternError}</span>
    </p>
  {/if}

  <!-- Champs cliquables : retapés à la main, une faute ne se voit qu'à l'erreur. -->
  <div class="flex flex-wrap items-center gap-1">
    <span class="mr-0.5 text-[10px] font-semibold uppercase tracking-widest
                 text-neutral-400 dark:text-neutral-500">
      {$t("rename.fields")}
    </span>
    {#each PATTERN_FIELDS as field (field)}
      <button
        type="button"
        onclick={() => insert(field)}
        class="px-1.5 h-6 rounded-md font-mono text-[10.5px] cursor-pointer
               transition-colors
               bg-neutral-200/60 dark:bg-white/6
               text-neutral-500 dark:text-neutral-400
               hover:bg-emerald-500/15 hover:text-emerald-600 dark:hover:text-emerald-400"
      >
        {field}
      </button>
    {/each}
  </div>

  <div class="flex flex-wrap items-center gap-1">
    <span class="mr-0.5 text-[10px] font-semibold uppercase tracking-widest
                 text-neutral-400 dark:text-neutral-500">
      {$t("rename.presets")}
    </span>
    {#each presets as preset (preset.pattern)}
      <button
        type="button"
        onclick={() => onpatternchange(preset.pattern)}
        class="px-2 h-6 rounded-md text-[10.5px] cursor-pointer transition-colors
               {pattern === preset.pattern
                 ? 'bg-emerald-500/15 text-emerald-600 dark:text-emerald-400'
                 : 'bg-neutral-200/60 dark:bg-white/6 text-neutral-500 dark:text-neutral-400 hover:bg-neutral-300/60 dark:hover:bg-white/10'}"
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
        <span class="text-[11.5px] text-neutral-600 dark:text-neutral-300">
          {$t("rename.satellites")}
        </span>
      </label>
      <label class="flex items-center gap-2 cursor-pointer">
        <input type="checkbox" bind:checked={cleanup} class="checkbox-app" />
        <span class="text-[11.5px] text-neutral-600 dark:text-neutral-300">
          {$t("rename.cleanup")}
        </span>
      </label>
    </div>
  {/if}
</div>
