<script lang="ts">
  // Rangée défilante sans barre : flèches aux bords et fondu tant qu'il reste à voir.
  import Icon from "@iconify/svelte";
  import type { Snippet } from "svelte";
  import { t } from "$lib/i18n";

  let {
    children,
    axe = "50%",
    class: classes = "",
    fleches = true,
    avant = $bindable(false),
    apres = $bindable(false),
  }: {
    children: Snippet;
    /** Hauteur des flèches depuis le haut : le milieu des pochettes plutôt que de la rangée. */
    axe?: string;
    class?: string;
    /** Flèches aux bords ; sans elles, le parent pilote via `avant`/`apres` et `defiler()`. */
    fleches?: boolean;
    avant?: boolean;
    apres?: boolean;
  } = $props();

  let piste: HTMLDivElement | null = $state(null);

  function mesurer() {
    if (!piste) return;
    avant = piste.scrollLeft > 2;
    apres = piste.scrollLeft + piste.clientWidth < piste.scrollWidth - 2;
  }

  // Une page à la fois, moins un bout de tuile pour garder le fil.
  export function defiler(sens: 1 | -1) {
    piste?.scrollBy({ left: sens * piste.clientWidth * 0.85, behavior: "smooth" });
  }

  $effect(() => {
    if (!piste) return;
    const ro = new ResizeObserver(mesurer);
    ro.observe(piste);
    for (const enfant of piste.children) ro.observe(enfant);
    mesurer();
    return () => ro.disconnect();
  });

  const masque = $derived(
    `linear-gradient(to right, ${avant ? "transparent, #000 48px" : "#000"}, ${apres ? "#000 calc(100% - 48px), transparent" : "#000"})`,
  );
  const cachee = "opacity-0 scale-90 pointer-events-none";
  const fleche = `absolute z-10 w-10 h-10 -mt-5 rounded-full flex items-center justify-center cursor-pointer transition duration-200
                  bg-white text-neutral-800 shadow-[0_6px_18px_rgba(0,0,0,0.22)] ring-1 ring-black/8 hover:scale-105
                  dark:bg-[#1c211e] dark:text-white dark:ring-white/12 dark:shadow-[0_6px_18px_rgba(0,0,0,0.6)]`;
</script>

<div class="relative">
  <div
    bind:this={piste}
    onscroll={mesurer}
    class="flex overflow-x-auto snap-x snap-mandatory [scrollbar-width:none] [&::-webkit-scrollbar]:hidden {classes}"
    style:mask-image={masque}
  >
    {@render children()}
  </div>
  <!-- Toujours montées (icônes prêtes), elles apparaissent en fondu. -->
  {#if fleches}
  <button type="button" class="{fleche} left-1 {avant ? '' : cachee}" style:top={axe} tabindex={avant ? 0 : -1} aria-hidden={!avant}
          aria-label={$t("common.previous")} title={$t("common.previous")} onclick={() => defiler(-1)}>
    <Icon icon="material-symbols:chevron-left-rounded" width="26" />
  </button>
  <button type="button" class="{fleche} right-1 {apres ? '' : cachee}" style:top={axe} tabindex={apres ? 0 : -1} aria-hidden={!apres}
          aria-label={$t("common.next")} title={$t("common.next")} onclick={() => defiler(1)}>
    <Icon icon="material-symbols:chevron-right-rounded" width="26" />
  </button>
  {/if}
</div>
