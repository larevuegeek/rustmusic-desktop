<script lang="ts">
  // Dossier illustré (SVG maison) : dos et rabat en dégradé ambre, un vinyle qui dépasse s'il contient de la musique.
  let { music = false, class: classes = "" }: { music?: boolean; class?: string } = $props();
  const id = $props.id();
</script>

<svg viewBox="0 0 64 64" class={classes} aria-hidden="true">
  <defs>
    <linearGradient id="{id}-dos" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#d98a1c" />
      <stop offset="1" stop-color="#9a5a0c" />
    </linearGradient>
    <linearGradient id="{id}-face" x1="0" y1="0" x2="0.35" y2="1">
      <stop offset="0" stop-color="#fcd877" />
      <stop offset="0.55" stop-color="#f5b335" />
      <stop offset="1" stop-color="#e0901a" />
    </linearGradient>
    <radialGradient id="{id}-disque" cx="0.5" cy="0.5" r="0.5">
      <stop offset="0" stop-color="#3a3530" />
      <stop offset="1" stop-color="#141210" />
    </radialGradient>
    <filter id="{id}-ombre" x="-20%" y="-20%" width="140%" height="150%">
      <feDropShadow dx="0" dy="1.6" stdDeviation="1.6" flood-color="#5a3206" flood-opacity="0.35" />
    </filter>
  </defs>

  <!-- Dos et onglet -->
  <path d="M7 15.5a4.5 4.5 0 0 1 4.5-4.5h12.6a3 3 0 0 1 2.1.9l3.6 3.6h22.7a4.5 4.5 0 0 1 4.5 4.5v29a4.5 4.5 0 0 1-4.5 4.5h-41A4.5 4.5 0 0 1 7 49z" fill="url(#{id}-dos)" />

  {#if music}
    <!-- Vinyle glissé dans le dossier -->
    <g>
      <circle cx="38" cy="20" r="14" fill="url(#{id}-disque)" />
      <circle cx="38" cy="20" r="11" fill="none" stroke="#fff" stroke-opacity="0.07" stroke-width="0.8" />
      <circle cx="38" cy="20" r="8" fill="none" stroke="#fff" stroke-opacity="0.07" stroke-width="0.8" />
      <path d="M29 12.5a12 12 0 0 1 9-4" fill="none" stroke="#fff" stroke-opacity="0.22" stroke-width="1.2" stroke-linecap="round" />
      <circle cx="38" cy="20" r="4.2" fill="#22c55e" />
      <circle cx="38" cy="20" r="1.1" fill="#141210" />
    </g>
  {:else}
    <!-- Feuillet vide -->
    <rect x="15" y="17" width="34" height="20" rx="2" fill="#fff8e8" opacity="0.9" />
  {/if}

  <!-- Rabat avant -->
  <path d="M5.2 26.6A4.5 4.5 0 0 1 9.7 22h44.6a4.5 4.5 0 0 1 4.5 4.6l-.9 22.6a4.5 4.5 0 0 1-4.5 4.3H10.6a4.5 4.5 0 0 1-4.5-4.3z"
        fill="url(#{id}-face)" filter="url(#{id}-ombre)" />
  <path d="M9.7 23.2h44.6" stroke="#fff" stroke-opacity="0.55" stroke-width="1" stroke-linecap="round" />
  {#if music}
    <!-- Note gravée dans le rabat -->
    <path d="M36 32.5v9.2a3 3 0 1 1-1.6-2.6v-5.4l7.6-1.6v7.8a3 3 0 1 1-1.6-2.6v-4z" fill="#8a4c08" opacity="0.28" />
  {/if}
</svg>
