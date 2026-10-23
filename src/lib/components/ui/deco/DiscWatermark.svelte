<script lang="ts">
  // `sky` : bibliothèque d'un serveur.
  let { class: classe = "", tone = "green" }: { class?: string; tone?: "green" | "sky" } = $props();
</script>

<svg class="disque pointer-events-none {tone === 'sky' ? 'text-sky-500 dark:text-sky-400' : 'text-(--rg-g)'} {classe}" viewBox="0 0 200 200" aria-hidden="true">
  {#each [96, 84, 72, 60, 48, 36] as r, i (r)}
    <circle cx="100" cy="100" {r} fill="none" stroke="currentColor"
            stroke-width={i === 0 ? 1.4 : 0.7} opacity={0.10 + i * 0.035}
            stroke-dasharray={i % 2 ? "2 5" : "none"} />
  {/each}
  <circle cx="100" cy="100" r="22" fill="currentColor" opacity="0.18" />
  <circle cx="100" cy="100" r="4" fill="currentColor" opacity="0.5" />
</svg>

<style>
  .disque {
    animation: tourner 60s linear infinite;
  }
  @keyframes tourner {
    to { transform: rotate(360deg); }
  }
  @media (prefers-reduced-motion: reduce) {
    .disque { animation: none; }
  }
</style>
