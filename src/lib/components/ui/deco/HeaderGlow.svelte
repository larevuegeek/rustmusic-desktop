<!-- Lueur violette derrière l'en-tête des pages podcasts (parent en `relative overflow-hidden`). -->
<div class="header-glow absolute inset-0 pointer-events-none" aria-hidden="true"></div>

<style>
  .header-glow {
    background:
      radial-gradient(55% 120% at 12% 0%, rgb(139 92 246 / 0.16), transparent 70%),
      radial-gradient(40% 90% at 70% -10%, rgb(217 70 239 / 0.1), transparent 70%);
    mask-image: linear-gradient(#000 55%, transparent);
  }
  :global(.dark) .header-glow {
    background:
      radial-gradient(55% 120% at 12% 0%, rgb(139 92 246 / 0.28), transparent 70%),
      radial-gradient(40% 90% at 70% -10%, rgb(217 70 239 / 0.14), transparent 70%);
  }
</style>
