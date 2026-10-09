<script module lang="ts">
  import type { TrackListView } from "#lib/types/ui/library/track/TrackListView";
  export type ActionMix = { icone: string; libelle: string; danger?: boolean; agir: (liste: TrackListView[]) => void };
</script>

<script lang="ts">
  // Carte d'un mix : quatre pochettes du tirage du jour, son nom, ses artistes ; un clic le lance.
  // Le tirage se fait à l'approche de l'écran : une page de cinquante mix n'en tire que ce qu'on voit.
  import Icon from "@iconify/svelte";
  import { t } from "#lib/i18n";
  import CoverImg from "#lib/components/ui/image/CoverImg.svelte";
  import Menu from "#lib/components/ui/menu/Menu.svelte";
  import MenuItem from "#lib/components/ui/menu/MenuItem.svelte";
  import { untrack } from "svelte";
  import { tirer, apercu, generationMix, type SourceMix } from "#lib/stores/mix/mix.store";
  import { lancerMix } from "#lib/actions/mix/MixAction";

  let {
    libraryId,
    cle,
    source,
    type,
    nom,
    aide = "",
    h,
    variante = "carre",
    pre = "",
    court = "",
    actions = [],
  }: {
    libraryId: number;
    cle: string;
    source: SourceMix;
    /** Petit intitulé au-dessus du nom (« Mix du jour »…). */
    type: string;
    nom: string;
    aide?: string;
    /** Teinte quand il n'y a pas de pochette. */
    h: number;
    /** `horizontale` : vignette à gauche sur écran étroit ; `decennie` : le millésime en grand. */
    variante?: "carre" | "horizontale" | "decennie";
    pre?: string;
    court?: string;
    actions?: ActionMix[];
  } = $props();

  let liste = $state<TrackListView[] | null>(null);
  let visible = $state(false);
  let lance = $state(false);
  let menu = $state(false);

  $effect(() => {
    if (!visible) return;
    void $generationMix;
    // La source suit la clé : relire l'objet (recréé à chaque calcul du parent) relancerait le tirage pour rien.
    const [id, c] = [libraryId, cle];
    const s = untrack(() => source);
    let perime = false;
    // Le tirage d'une autre carte rejoue cet effet : le cache rend la même liste, sans clignotement.
    tirer(id, c, s).then((l) => { if (!perime) liste = l; });
    return () => { perime = true; };
  });

  function approcher(noeud: HTMLElement) {
    const io = new IntersectionObserver((e) => {
      if (e.some((x) => x.isIntersecting)) {
        visible = true;
        io.disconnect();
      }
    }, { root: noeud.closest(".overflow-y-auto"), rootMargin: "400px" });
    io.observe(noeud);
    return { destroy: () => io.disconnect() };
  }

  const a = $derived(liste ? apercu(liste, $t) : null);
  const horizontale = $derived(variante === "horizontale");
  const decennie = $derived(variante === "decennie");

  async function jouer() {
    if (!liste?.length || lance) return;
    lance = true;
    try {
      await lancerMix(liste);
    } finally {
      lance = false;
    }
  }
</script>

<div class="group relative min-w-0" use:approcher style="--h: {h}">
  <button type="button" onclick={jouer} disabled={!liste?.length} title={aide}
          class="flex text-left cursor-pointer min-w-0 w-full disabled:cursor-default
                 {horizontale ? 'flex-row items-center gap-4 @4xl:flex-col @4xl:items-stretch @4xl:gap-3' : decennie ? 'flex-col gap-2.5' : 'flex-col gap-3'}">

    <!-- La pochette du mix : quatre albums qu'il contient. -->
    <div class="relative aspect-square overflow-hidden shrink-0
                shadow-[0_12px_32px_rgba(0,0,0,0.28)] ring-1 ring-black/5 dark:ring-white/[0.07]
                transition-transform duration-300 group-hover:-translate-y-1
                {horizontale ? 'w-32 rounded-xl @4xl:w-full @4xl:rounded-2xl' : 'w-full rounded-2xl'}">
      {#if !a}
        <div class="absolute inset-0 bg-neutral-200 dark:bg-white/6 {visible ? 'animate-pulse' : ''}"></div>
      {:else if a.pochettes.length >= 4}
        <div class="absolute inset-0 grid grid-cols-2 grid-rows-2">
          {#each a.pochettes as p (p)}
            <CoverImg path={p} size={decennie ? "1x" : "2x"} class="w-full h-full object-cover" />
          {/each}
        </div>
      {:else if a.pochettes.length > 0}
        <CoverImg path={a.pochettes[0]} size={decennie ? "2x" : "full"} class="absolute inset-0 w-full h-full object-cover" />
      {:else}
        <div class="bande-teintee absolute inset-0"></div>
      {/if}

      <div class="mix-voile absolute inset-0 {horizontale ? 'hidden @4xl:block' : ''}"></div>

      {#if decennie}
        <div class="absolute left-3.5 bottom-3 flex flex-col">
          <span class="mix-accent text-[10.5px] font-bold uppercase tracking-[0.14em]">{pre}</span>
          <span class="text-[40px] font-black tracking-[-0.04em] leading-[0.95] text-white">{court}</span>
        </div>
      {:else}
        <div class="absolute left-4 right-16 bottom-4 flex-col gap-0.5 {horizontale ? 'hidden @4xl:flex' : 'flex'}">
          <span class="mix-accent text-[11px] font-bold uppercase tracking-[0.1em] truncate">{type}</span>
          <span class="{horizontale ? 'text-[22px] @6xl:text-[26px]' : 'text-[20px]'} font-extrabold tracking-[-0.02em] leading-[1.1] text-white line-clamp-2 wrap-anywhere hyphens-auto">{nom}</span>
        </div>
      {/if}

      <span class="absolute rounded-full bg-accent-500 text-(--rg-on-g)
                   flex items-center justify-center shadow-[0_8px_20px_rgba(0,0,0,0.4)] transition-all duration-200
                   {horizontale ? 'right-2 bottom-2 w-9 h-9 @4xl:right-3.5 @4xl:bottom-3.5 @4xl:w-12 @4xl:h-12' : decennie ? 'right-3 bottom-3 w-10 h-10' : 'right-3.5 bottom-3.5 w-12 h-12'}
                   {lance ? 'opacity-100' : 'opacity-0 translate-y-1.5 group-hover:opacity-100 group-hover:translate-y-0'}">
        <Icon icon={lance ? "lucide:loader-circle" : "mynaui:play-solid"} width={decennie ? 18 : 20} class={lance ? "animate-spin" : ""} />
      </span>
    </div>

    <!-- Ce qu'il contient ; en carte horizontale, le type et le nom passent ici. -->
    <div class="flex flex-col gap-1 px-0.5 min-w-0 flex-1">
      {#if horizontale}
        <span class="@4xl:hidden accent-teinte text-[11px] font-bold uppercase tracking-[0.1em] truncate">{type}</span>
        <span class="@4xl:hidden text-xl font-extrabold tracking-[-0.02em] leading-tight truncate text-neutral-900 dark:text-white">{nom}</span>
      {/if}
      {#if a}
        <p class="{decennie ? 'text-[13px]' : 'text-[14px]'} leading-snug text-neutral-700 dark:text-neutral-200 line-clamp-2">{a.description}</p>
        <p class="{decennie ? 'text-xs' : 'text-[12.5px]'} text-neutral-500 dark:text-[#9aa39e]">{a.meta}</p>
      {:else}
        <div class="h-3.5 w-4/5 rounded bg-neutral-200 dark:bg-white/6 {visible ? 'animate-pulse' : ''}"></div>
        <div class="h-3 w-2/5 rounded bg-neutral-200 dark:bg-white/6 {visible ? 'animate-pulse' : ''}"></div>
      {/if}
    </div>
  </button>

  {#if actions.length > 0}
    <!-- Les actions du mix, posées sur la pochette (le bouton n'est pas dans celui de lecture). -->
    <div class="absolute right-2.5 top-2.5">
      <button type="button" onclick={() => (menu = !menu)} title={$t("mix_view.actions")} aria-label={$t("mix_view.actions")} aria-expanded={menu}
              class="w-8 h-8 flex items-center justify-center rounded-full cursor-pointer bg-black/55 text-white backdrop-blur-sm hover:bg-black/75 transition
                     {menu ? '' : 'opacity-0 group-hover:opacity-100 focus-visible:opacity-100'}">
        <Icon icon="material-symbols:more-horiz" width="20" />
      </button>
      <Menu bind:open={menu} class="w-60">
        {#each actions as act (act.libelle)}
          <MenuItem icon={act.icone} danger={act.danger} onclick={() => { menu = false; act.agir(liste ?? []); }}>{act.libelle}</MenuItem>
        {/each}
      </Menu>
    </div>
  {/if}
</div>
