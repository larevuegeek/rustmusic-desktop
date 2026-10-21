<script lang="ts">
  // Illustration d'une diapo : dégradé, grande icône sur verre, petites icônes qui flottent autour.
  import Icon from "@iconify/svelte";
  import type { WhatsNewSlide } from "#lib/config/whatsNew";

  let { slide }: { slide: WhatsNewSlide } = $props();

  const PLACES = ["left-[15%] top-[42%]", "right-[15%] top-[22%]", "right-[25%] bottom-[14%]"];
</script>

<div class="relative h-full w-full overflow-hidden" style="background: linear-gradient(135deg, {slide.colors[0]}, {slide.colors[1]})">
  {#if slide.image}
    <img src={slide.image} alt="" class="absolute inset-0 w-full h-full object-cover" />
  {:else}
    <div class="absolute inset-0 art-lumiere" aria-hidden="true"></div>
    {#each (slide.satellites ?? []).slice(0, PLACES.length) as s, i (s)}
      <span class="absolute {PLACES[i]} w-12 h-12 rounded-2xl flex items-center justify-center text-white
                   bg-white/15 border border-white/25 backdrop-blur-sm shadow-[0_10px_24px_rgba(0,0,0,0.18)] art-flotte"
            style="animation-delay: {i * -1.4}s" aria-hidden="true">
        <Icon icon={s} width="24" />
      </span>
    {/each}
    <span class="absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 w-28 h-28 rounded-[30px] flex items-center justify-center text-white
                 bg-white/20 border border-white/35 backdrop-blur-md shadow-[0_20px_50px_rgba(0,0,0,0.25)]" aria-hidden="true">
      <Icon icon={slide.icon} width="58" />
    </span>
  {/if}
</div>

<style>
  .art-lumiere {
    background:
      radial-gradient(60% 80% at 20% 0%, rgb(255 255 255 / 0.28), transparent 70%),
      radial-gradient(50% 70% at 100% 100%, rgb(0 0 0 / 0.22), transparent 70%);
  }
  .art-flotte {
    animation: flotte 5.6s ease-in-out infinite;
  }
  @keyframes flotte {
    0%, 100% { transform: translateY(0); }
    50% { transform: translateY(-7px); }
  }
  @media (prefers-reduced-motion: reduce) {
    .art-flotte { animation: none; }
  }
</style>
