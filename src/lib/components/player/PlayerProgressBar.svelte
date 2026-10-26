<script lang="ts">
  import { t } from "#lib/i18n";
  // Barre de progression du lecteur : rail fin qui s'épaissit au survol, remplissage à la couleur du morceau.
  // `marks` (en %) : débuts de chapitres, découpés dans le rail.
  let { position = 0, marks = [], onseek }: { position?: number; marks?: number[]; onseek?: (percent: number) => void } = $props();

  const masque = $derived.by(() => {
    const stops = marks
      .filter((m) => m > 0.5 && m < 99.5)
      .flatMap((m) => [`#000 calc(${m}% - 1.5px)`, `transparent calc(${m}% - 1.5px)`, `transparent calc(${m}% + 1.5px)`, `#000 calc(${m}% + 1.5px)`]);
    if (!stops.length) return "";
    const g = `linear-gradient(to right, #000 0, ${stops.join(", ")}, #000 100%)`;
    return `mask-image: ${g}; -webkit-mask-image: ${g}`;
  });

  let localPosition = $state(0);
  let seeking = $state(false);
  let seekTarget = $state(-1);

  // Pendant un saut, on attend que la position réelle rattrape la cible.
  $effect(() => {
    if (seeking) {
      if (seekTarget >= 0 && Math.abs(position - seekTarget) < 2) {
        seeking = false;
        seekTarget = -1;
        localPosition = position;
      }
    } else {
      localPosition = position;
    }
  });

  function handleInput(e: Event) {
    seeking = true;
    localPosition = Number((e.target as HTMLInputElement).value);
  }

  function handleChange(e: Event) {
    const value = Number((e.target as HTMLInputElement).value);
    localPosition = value;
    seekTarget = value;
    onseek?.(value);
  }
</script>

<div class="group/progress relative w-full h-4.5 flex items-center">
  <div class="relative w-full h-0.75 group-hover/progress:h-1.25 transition-[height] duration-150">
    <div class="absolute inset-0 rounded-full overflow-hidden bg-(--lc-rail)" style={masque}>
      <div class="absolute inset-y-0 left-0 rounded-full bg-(--lc-acc)" style:width="{localPosition}%"></div>
    </div>
    <div
      class="absolute top-1/2 w-3 h-3 -mt-1.5 -ml-1.5 rounded-full bg-white shadow-[0_1px_4px_rgba(0,0,0,0.5)]
             opacity-0 group-hover/progress:opacity-100 transition-opacity duration-150 pointer-events-none"
      style:left="{localPosition}%"
    ></div>
  </div>
  <input
    type="range"
    min="0"
    max="100"
    step="0.1"
    value={localPosition}
    oninput={handleInput}
    onchange={handleChange}
    class="absolute inset-0 w-full h-full cursor-pointer opacity-0"
    aria-label={$t("common.progress")}
  />
</div>
