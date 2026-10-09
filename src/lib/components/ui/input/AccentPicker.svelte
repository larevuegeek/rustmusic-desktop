<script lang="ts">
  // Pastilles de couleur d'accent, plus une pastille arc-en-ciel pour une couleur libre.
  import Icon from "@iconify/svelte";
  import { t } from "#lib/i18n";
  import { ACCENT_PRESETS, DEFAULT_ACCENT, isValidAccent } from "#lib/helper/theme/accent";

  let { value, onchange }: { value: string; onchange: (hex: string) => void } = $props();

  const current = $derived(isValidAccent(value) ? value.toLowerCase() : DEFAULT_ACCENT);
  const isCustom = $derived(!ACCENT_PRESETS.some((p) => p.hex === current));

  // La roue du système envoie une valeur à chaque mouvement : on n'enregistre qu'au relâchement.
  let preview = $state<string | null>(null);
</script>

<div class="flex flex-wrap items-center gap-2.5" role="radiogroup" aria-label={$t("settings.accent")}>
  {#each ACCENT_PRESETS as preset (preset.hex)}
    {@const selected = current === preset.hex}
    <button
      type="button"
      role="radio"
      aria-checked={selected}
      aria-label={$t(preset.labelKey)}
      title={$t(preset.labelKey)}
      class="w-8 h-8 rounded-full flex items-center justify-center cursor-pointer transition-transform hover:scale-110
             {selected ? 'ring-2 ring-offset-2 ring-(--rg-tx) ring-offset-(--rg-carte)' : ''}"
      style="background: {preset.hex}"
      onclick={() => onchange(preset.hex === DEFAULT_ACCENT ? "" : preset.hex)}
    >
      {#if selected}<Icon icon="material-symbols:check-rounded" width="18" class="text-white drop-shadow" />{/if}
    </button>
  {/each}

  <!-- Couleur libre : la roue du système, sous une pastille arc-en-ciel (ou la couleur choisie). -->
  <label
    class="relative w-8 h-8 rounded-full flex items-center justify-center cursor-pointer transition-transform hover:scale-110
           {isCustom ? 'ring-2 ring-offset-2 ring-(--rg-tx) ring-offset-(--rg-carte)' : ''}"
    style="background: {isCustom || preview ? (preview ?? current) : 'conic-gradient(#ef4444, #f59e0b, #22c55e, #06b6d4, #3b82f6, #a855f7, #ec4899, #ef4444)'}"
    title={$t("settings.accent_custom")}
  >
    {#if isCustom}<Icon icon="material-symbols:check-rounded" width="18" class="text-white drop-shadow" />
    {:else}<Icon icon="material-symbols:add-rounded" width="18" class="text-white drop-shadow" />{/if}
    <input
      type="color"
      class="absolute inset-0 opacity-0 cursor-pointer"
      value={current}
      aria-label={$t("settings.accent_custom")}
      oninput={(e) => (preview = e.currentTarget.value)}
      onchange={(e) => { preview = null; onchange(e.currentTarget.value.toLowerCase()); }}
    />
  </label>
</div>
