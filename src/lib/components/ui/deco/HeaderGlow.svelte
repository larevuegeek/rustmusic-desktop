<script lang="ts">
  // Lueur colorée derrière un en-tête de page (parent en `relative overflow-hidden`).
  let { tone = "violet" }: { tone?: "violet" | "rose" } = $props();
</script>

<div class="header-glow {tone} absolute inset-0 pointer-events-none" aria-hidden="true"></div>

<style>
  .header-glow {
    mask-image: linear-gradient(#000 55%, transparent);
  }
  .violet {
    background:
      radial-gradient(55% 120% at 12% 0%, rgb(139 92 246 / 0.16), transparent 70%),
      radial-gradient(40% 90% at 70% -10%, rgb(217 70 239 / 0.1), transparent 70%);
  }
  :global(.dark) .violet {
    background:
      radial-gradient(55% 120% at 12% 0%, rgb(139 92 246 / 0.28), transparent 70%),
      radial-gradient(40% 90% at 70% -10%, rgb(217 70 239 / 0.14), transparent 70%);
  }
  .rose {
    background:
      radial-gradient(55% 120% at 12% 0%, rgb(244 63 94 / 0.14), transparent 70%),
      radial-gradient(40% 90% at 70% -10%, rgb(249 115 22 / 0.1), transparent 70%);
  }
  :global(.dark) .rose {
    background:
      radial-gradient(55% 120% at 12% 0%, rgb(244 63 94 / 0.24), transparent 70%),
      radial-gradient(40% 90% at 70% -10%, rgb(249 115 22 / 0.13), transparent 70%);
  }
</style>
