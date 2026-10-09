<script lang="ts">
  let { width = 140 }: { width?: number } = $props();
</script>

<svg {width} viewBox="0 0 181 28" fill="none" xmlns="http://www.w3.org/2000/svg">
  <!-- Casque audio (ph:headphones-fill, Phosphor) -->
  <g transform="translate(1, 1)">
    <svg viewBox="0 0 256 256" width="26" height="26">
      <path class="fill-accent-600 dark:fill-[url(#rmGrad)]"
            d="M232 128v56a24 24 0 0 1-24 24h-16a24 24 0 0 1-24-24v-40a24 24 0 0 1 24-24h23.65a87.71 87.71 0 0 0-87-80H128a88 88 0 0 0-87.64 80H64a24 24 0 0 1 24 24v40a24 24 0 0 1-24 24H48a24 24 0 0 1-24-24v-56a104.11 104.11 0 0 1 177.89-73.34A103.4 103.4 0 0 1 232 128"/>
    </svg>
  </g>

  <!-- "RustMusic" -->
  <text x="30" y="21"
        font-family="Inter Variable, Inter, system-ui, sans-serif"
        font-size="22" font-weight="800" letter-spacing="-1.2">
    <tspan class="fill-[#1a1c1a] dark:fill-neutral-100">Rust</tspan><tspan class="fill-accent-700 dark:fill-[url(#rmGrad)]">Music</tspan>
  </text>

  <!-- Badge Beta -->
  <rect x="142" y="4" width="38" height="16" rx="4" class="fill-[#1a1c1a] dark:fill-[url(#rmGrad)] dark:opacity-85"/>
  <text x="161" y="15.5" text-anchor="middle"
        font-family="Inter Variable, Inter, system-ui, sans-serif"
        font-size="9" font-weight="700" fill="white" letter-spacing="0.5">BETA</text>

  <defs>
    <linearGradient id="rmGrad" x1="0" y1="0" x2="28" y2="28" gradientUnits="userSpaceOnUse">
      <stop offset="0%" style="stop-color: var(--acc-400, #34d369)"/>
      <stop offset="100%" style="stop-color: var(--acc-500, #22c55e)"/>
    </linearGradient>
  </defs>
</svg>
