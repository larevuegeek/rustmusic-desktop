<script lang="ts" module>
  export type WheelKey = "up" | "down" | "left" | "right" | "enter";
</script>

<script lang="ts">
  // Molette façon iPod : un appui sur l'anneau donne la direction pointée, un tour du doigt
  // (ou la molette de la souris) fait défiler vers le haut ou le bas ; OK au centre.
  import Icon from "@iconify/svelte";

  let {
    onkey,
    labels,
    size = 152,
  }: {
    onkey: (key: WheelKey) => void;
    labels: { up: string; down: string; left: string; right: string; ok: string; wheel: string };
    size?: number;
  } = $props();

  /** Un cran tous les 30° : un tour complet fait douze pas. */
  const CRAN = Math.PI / 6;

  let anneau = $state<HTMLDivElement | null>(null);
  let appuye = $state<WheelKey | null>(null);
  /** Position du doigt sur l'anneau pendant un tour (angle en radians), pour le témoin. */
  let doigt = $state<number | null>(null);
  let geste: { id: number; angle: number; cumul: number; tourne: boolean } | null = null;
  let tEclat: ReturnType<typeof setTimeout> | undefined;

  function eclat(key: WheelKey) {
    appuye = key;
    clearTimeout(tEclat);
    tEclat = setTimeout(() => (appuye = null), 160);
  }

  function presser(key: WheelKey) {
    eclat(key);
    onkey(key);
  }

  function angleDe(e: PointerEvent): number {
    const r = anneau!.getBoundingClientRect();
    return Math.atan2(e.clientY - (r.top + r.height / 2), e.clientX - (r.left + r.width / 2));
  }

  /** Quart visé : haut, droite, bas ou gauche. */
  function direction(a: number): WheelKey {
    const deg = (a * 180) / Math.PI;
    if (deg >= -135 && deg < -45) return "up";
    if (deg >= -45 && deg < 45) return "right";
    if (deg >= 45 && deg < 135) return "down";
    return "left";
  }

  function debut(e: PointerEvent) {
    if (e.button !== 0) return;
    anneau!.setPointerCapture(e.pointerId);
    geste = { id: e.pointerId, angle: angleDe(e), cumul: 0, tourne: false };
  }

  function suivre(e: PointerEvent) {
    if (!geste || e.pointerId !== geste.id) return;
    const a = angleDe(e);
    let d = a - geste.angle;
    if (d > Math.PI) d -= 2 * Math.PI;
    if (d < -Math.PI) d += 2 * Math.PI;
    geste.angle = a;
    geste.cumul += d;
    if (Math.abs(geste.cumul) > CRAN / 2) geste.tourne = true;
    if (geste.tourne) doigt = a;
    // Sens horaire : vers le bas, comme sur l'iPod.
    while (Math.abs(geste.cumul) >= CRAN) {
      const bas = geste.cumul > 0;
      geste.cumul -= bas ? CRAN : -CRAN;
      presser(bas ? "down" : "up");
    }
  }

  function fin(e: PointerEvent) {
    if (!geste || e.pointerId !== geste.id) return;
    if (!geste.tourne) presser(direction(angleDe(e)));
    geste = null;
    doigt = null;
  }

  // Molette de la souris : un pas par cran, sans faire défiler le panneau derrière.
  function molette(el: HTMLElement) {
    let reste = 0;
    const roue = (e: WheelEvent) => {
      e.preventDefault();
      reste += e.deltaY;
      while (Math.abs(reste) >= 60) {
        presser(reste > 0 ? "down" : "up");
        reste -= Math.sign(reste) * 60;
      }
    };
    el.addEventListener("wheel", roue, { passive: false });
    return () => el.removeEventListener("wheel", roue);
  }

  function clavier(e: KeyboardEvent) {
    const touches: Record<string, WheelKey> = { ArrowUp: "up", ArrowDown: "down", ArrowLeft: "left", ArrowRight: "right", Enter: "enter", " ": "enter" };
    const key = touches[e.key];
    if (!key) return;
    e.preventDefault();
    presser(key);
  }

  const FLECHES: [WheelKey, string, string][] = [
    ["up", "material-symbols:keyboard-arrow-up-rounded", "top-1.5 left-1/2 -translate-x-1/2"],
    ["down", "material-symbols:keyboard-arrow-down-rounded", "bottom-1.5 left-1/2 -translate-x-1/2"],
    ["left", "material-symbols:keyboard-arrow-left-rounded", "left-1.5 top-1/2 -translate-y-1/2"],
    ["right", "material-symbols:keyboard-arrow-right-rounded", "right-1.5 top-1/2 -translate-y-1/2"],
  ];
</script>

<div
  bind:this={anneau}
  {@attach molette}
  class="molette relative shrink-0 rounded-full cursor-pointer select-none touch-none outline-none"
  style="width: {size}px; height: {size}px"
  role="slider"
  tabindex="0"
  aria-label={labels.wheel}
  aria-valuenow={0}
  onpointerdown={debut}
  onpointermove={suivre}
  onpointerup={fin}
  onpointercancel={fin}
  onkeydown={clavier}
>
  {#each FLECHES as [key, icone, place] (key)}
    <span aria-hidden="true"
          class="absolute w-9 h-9 flex items-center justify-center rounded-full transition-colors duration-150 {place}
                 {appuye === key ? 'text-(--rg-gtx) bg-(--rg-g)/15' : 'text-(--rg-tx2)'}">
      <Icon icon={icone} width="24" />
    </span>
  {/each}
  {#if doigt !== null}
    <!-- Témoin du tour : un point qui suit le doigt sur l'anneau. -->
    <span aria-hidden="true" class="absolute w-2.5 h-2.5 -m-1.25 rounded-full bg-(--rg-g) shadow-[0_0_8px_var(--rg-g)] pointer-events-none"
          style="left: {50 + 39 * Math.cos(doigt)}%; top: {50 + 39 * Math.sin(doigt)}%"></span>
  {/if}
  <button
    type="button"
    class="ok absolute inset-0 m-auto rounded-full cursor-pointer text-[13px] font-bold transition-all {appuye === 'enter' ? 'scale-95' : ''}"
    style="width: {Math.round(size * 0.4)}px; height: {Math.round(size * 0.4)}px"
    aria-label={labels.ok}
    onpointerdown={(e) => e.stopPropagation()}
    onclick={() => presser("enter")}
  >OK</button>
  <span class="sr-only">{labels.up}, {labels.down}, {labels.left}, {labels.right}</span>
</div>

<style>
  /* Anneau creusé, bouton central bombé qui s'allume au survol. */
  .molette {
    background: radial-gradient(circle at 50% 30%, color-mix(in oklch, var(--rg-s2) 85%, white), var(--rg-s2) 70%);
    box-shadow: inset 0 2px 6px rgb(0 0 0 / 0.25), inset 0 -1px 0 rgb(255 255 255 / 0.05), 0 6px 18px rgb(0 0 0 / 0.18);
  }
  .molette:focus-visible {
    box-shadow: inset 0 2px 6px rgb(0 0 0 / 0.25), 0 0 0 2px var(--rg-g);
  }
  .ok {
    background: radial-gradient(circle at 50% 30%, color-mix(in oklch, var(--rg-carte) 85%, white), var(--rg-carte));
    color: var(--rg-tx);
    border: 1px solid var(--rg-bd2);
    box-shadow: 0 3px 10px rgb(0 0 0 / 0.3), inset 0 1px 0 rgb(255 255 255 / 0.1);
  }
  .ok:hover {
    color: var(--rg-gtx);
    border-color: color-mix(in oklch, var(--rg-g) 60%, transparent);
    box-shadow: 0 3px 10px rgb(0 0 0 / 0.3), 0 0 14px color-mix(in oklch, var(--rg-g) 35%, transparent);
  }
</style>
