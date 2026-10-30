<script lang="ts">
  // Fichier audio illustré (SVG maison) : feuille à coin plié, égaliseur, étiquette du format ; teinte selon la qualité.
  import type { PalierPiste } from "#lib/helper/tools/audioFormatTools";

  let { format = "", palier = "lossless", class: classes = "" }: { format?: string; palier?: PalierPiste; class?: string } = $props();
  const id = $props.id();

  const TEINTES: Record<PalierPiste, [string, string, string]> = {
    hires: ["#fcd877", "#d98a1c", "#fff3cc"],
    lossless: ["#4ade80", "#15803d", "#c9f7d9"],
    lossy: ["#a3b1c4", "#4b5a70", "#e3e9f1"],
  };
  const t = $derived(TEINTES[palier]);
  const label = $derived(format.toUpperCase().slice(0, 5));
</script>

<svg viewBox="0 0 64 64" class={classes} aria-hidden="true">
  <defs>
    <linearGradient id="{id}-page" x1="0" y1="0" x2="0.4" y2="1">
      <stop offset="0" stop-color={t[0]} />
      <stop offset="1" stop-color={t[1]} />
    </linearGradient>
    <filter id="{id}-ombre" x="-20%" y="-20%" width="140%" height="140%">
      <feDropShadow dx="0" dy="1.4" stdDeviation="1.4" flood-color="#000" flood-opacity="0.3" />
    </filter>
  </defs>

  <!-- Feuille et coin plié -->
  <path d="M15 5h23.5L51 17.5V55a4 4 0 0 1-4 4H15a4 4 0 0 1-4-4V9a4 4 0 0 1 4-4z" fill="url(#{id}-page)" filter="url(#{id}-ombre)" />
  <path d="M38.5 5v8.5a4 4 0 0 0 4 4H51z" fill={t[2]} opacity="0.9" />
  <path d="M15 6.2h22" stroke="#fff" stroke-opacity="0.45" stroke-width="1" stroke-linecap="round" />

  <!-- Égaliseur -->
  <g fill="#fff" opacity="0.92">
    <rect x="18.5" y="28" width="3.4" height="10" rx="1.7" />
    <rect x="24.2" y="23" width="3.4" height="20" rx="1.7" />
    <rect x="29.9" y="26" width="3.4" height="14" rx="1.7" />
    <rect x="35.6" y="21" width="3.4" height="24" rx="1.7" />
    <rect x="41.3" y="27" width="3.4" height="12" rx="1.7" />
  </g>

  {#if label}
    <!-- Étiquette du format -->
    <rect x="15" y="47" width="32" height="8.5" rx="2.5" fill="#000" fill-opacity="0.24" />
    <text x="31" y="53.4" text-anchor="middle" font-size="6.4" font-weight="800" letter-spacing="0.6" fill="#fff">{label}</text>
  {/if}
</svg>
