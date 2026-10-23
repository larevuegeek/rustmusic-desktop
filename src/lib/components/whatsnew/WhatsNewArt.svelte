<script lang="ts">
  // Illustration d'une diapo : dégradé en taches et grain ; derrière une grande icône de verre, deux rangées
  // de pastilles (ce qui est pris en charge) qui défilent en sens contraires.
  // Sans rien de tout ça, quelques petites icônes flottent autour.
  import Icon from "@iconify/svelte";
  import type { WhatsNewSlide } from "#lib/config/whatsNew";

  // Sans flou d'arrière-plan ni filtre : WebKitGTK les recalcule à chaque image du défilement,
  // et peint en noir une couche floutée montée hors écran (les diapos voisines).
  let { slide, active = true }: { slide: WhatsNewSlide; active?: boolean } = $props();

  const PLACES = ["left-[15%] top-[44%]", "right-[15%] top-[24%]", "right-[25%] bottom-[14%]"];

  const logos = $derived(slide.logos ?? []);

  /** Une rangée : un élément sur deux, répété pour remplir la largeur (puis doublée pour boucler). */
  function rangee<T>(items: T[], depart: number): T[] {
    const moitie = items.filter((_, i) => i % 2 === depart);
    const base = moitie.length ? moitie : items;
    let out = [...base];
    while (out.length < 9) out = [...out, ...base];
    return out;
  }
  const rangeesLogos = $derived(logos.length ? [rangee(logos, 0), rangee(logos, 1)] : []);
  const [c0, c1] = $derived(slide.colors);
</script>

{#snippet pastille(l: { icon: string; label: string })}
  <span class="verre h-11 pl-3 pr-4 inline-flex items-center gap-2 rounded-full text-white text-[14px] font-bold whitespace-nowrap">
    <Icon icon={l.icon} width="21" />{l.label}
  </span>
{/snippet}

<div class="art relative h-full w-full overflow-hidden isolate" class:pause={!active}
     style="--c0: {c0}; --c1: {c1}; background:
            radial-gradient(90% 120% at 0% 0%, color-mix(in oklab, var(--c0) 85%, white) 0%, transparent 60%),
            radial-gradient(80% 110% at 100% 100%, color-mix(in oklab, var(--c1) 80%, black) 0%, transparent 65%),
            linear-gradient(135deg, var(--c0), var(--c1));">
  {#if slide.image}
    <img src={slide.image} alt="" class="absolute inset-0 w-full h-full object-cover" />
  {:else}
    <div class="absolute left-[-10%] top-[-30%] w-[55%] h-[110%] opacity-60 tache-a" aria-hidden="true"></div>
    <div class="absolute right-[-12%] bottom-[-40%] w-[60%] h-[120%] opacity-50 tache-b" aria-hidden="true"></div>
    <div class="absolute inset-0 grain" aria-hidden="true"></div>

    {#if rangeesLogos.length}
      <!-- Deux rangées qui défilent, bords fondus -->
      <div class="absolute inset-x-0 top-1/2 -translate-y-1/2 py-8 flex flex-col gap-5 fondu" aria-hidden="true">
        {#each rangeesLogos as r, i (i)}
          <div class="piste {i ? 'inverse' : ''}">
            {#each [...r, ...r] as l, j (j)}
              <span class="shrink-0 pr-3.5">{@render pastille(l)}</span>
            {/each}
          </div>
        {/each}
      </div>
      <div class="absolute inset-0 projecteur" aria-hidden="true"></div>
    {:else}
      {#each (slide.satellites ?? []).slice(0, PLACES.length) as s, i (s)}
        <span class="absolute {PLACES[i]} art-flotte" style="animation-delay: {i * -1.4}s" aria-hidden="true">
          <span class="verre w-12 h-12 rounded-2xl flex items-center justify-center text-white"><Icon icon={s} width="24" /></span>
        </span>
      {/each}
    {/if}

    <span class="absolute left-1/2 top-1/2 w-80 h-80 -ml-40 -mt-40 halo" aria-hidden="true"></span>
    <span class="centre absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 w-[124px] h-[124px] rounded-[36px]
                 flex items-center justify-center text-white" aria-hidden="true">
      <Icon icon={slide.icon} width="62" class="icone" />
    </span>
  {/if}
</div>

<style>
  .tache-a { background: radial-gradient(closest-side, color-mix(in oklab, var(--c0) 55%, white), transparent); }
  .tache-b { background: radial-gradient(closest-side, color-mix(in oklab, var(--c1) 70%, white), transparent); }
  .halo { background: radial-gradient(closest-side, rgb(255 255 255 / 0.3), transparent); }
  .grain {
    opacity: 0.16;
    mix-blend-mode: overlay;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='160' height='160'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.85' numOctaves='2' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)'/%3E%3C/svg%3E");
  }
  .fondu {
    mask-image: linear-gradient(90deg, transparent, black 14%, black 86%, transparent);
    opacity: 0.78;
  }
  .piste {
    display: flex;
    width: max-content;
    animation: defile 46s linear infinite;
  }
  .piste.inverse {
    animation-direction: reverse;
    margin-left: -40px;
  }
  /* Couches propres et ordre explicite : pendant le glissement d'une diapo à l'autre,
     WebKitGTK passait le centre sous les pastilles animées. */
  .projecteur, .halo, .centre { will-change: transform; }
  .projecteur { z-index: 1; }
  .halo { z-index: 2; }
  .centre { z-index: 3; }
  .projecteur {
    background: radial-gradient(180px 150px at 50% 50%, color-mix(in oklab, var(--c0) 92%, var(--c1)) 38%, transparent 100%);
  }
  .verre {
    background: linear-gradient(160deg, rgb(255 255 255 / 0.38), rgb(255 255 255 / 0.16));
    box-shadow:
      inset 0 1px 0 rgb(255 255 255 / 0.55),
      0 0 0 1px rgb(255 255 255 / 0.28),
      0 10px 24px color-mix(in oklab, var(--c1) 45%, rgb(0 0 0 / 0.5));
  }
  .centre {
    background: linear-gradient(155deg, rgb(255 255 255 / 0.5), rgb(255 255 255 / 0.2) 55%, rgb(255 255 255 / 0.28));
    box-shadow:
      inset 0 1.5px 0 rgb(255 255 255 / 0.7),
      inset 0 -10px 24px rgb(255 255 255 / 0.10),
      0 0 0 1px rgb(255 255 255 / 0.35),
      0 26px 60px color-mix(in oklab, var(--c1) 55%, rgb(0 0 0 / 0.6));
  }
  .centre :global(.icone) { filter: drop-shadow(0 4px 14px rgb(255 255 255 / 0.45)); }
  .art-flotte { animation: flotte 5.6s ease-in-out infinite; }
  .pause .piste, .pause .art-flotte { animation-play-state: paused; }

  @keyframes defile {
    from { transform: translateX(0); }
    to { transform: translateX(-50%); }
  }
  @keyframes flotte {
    0%, 100% { transform: translateY(0); }
    50% { transform: translateY(-7px); }
  }
  @media (prefers-reduced-motion: reduce) {
    .piste, .art-flotte { animation: none; }
  }
</style>
