<script lang="ts">
  // Carte d'une playlist : mosaïque, nom, nature et taille ; la punaise l'épingle dans la barre.
  import Icon from "@iconify/svelte";
  import { goto } from "$app/navigation";
  import { t } from "#lib/i18n";
  import CoverImg from "#lib/components/ui/image/CoverImg.svelte";

  let {
    href,
    nom,
    sousTitre,
    couleur,
    icone,
    couvertures = [],
    auto = false,
    epinglee = null,
    onepingler,
    onmenu,
    liste = false,
  }: {
    href: string;
    nom: string;
    sousTitre: string;
    couleur: string;
    icone: string;
    couvertures?: string[];
    auto?: boolean;
    /** `null` : pas de punaise (Titres aimés, Récents). */
    epinglee?: boolean | null;
    onepingler?: () => void;
    onmenu?: (x: number, y: number) => void;
    /** Ligne de liste plutôt que carte. */
    liste?: boolean;
  } = $props();

  function menu(e: MouseEvent) {
    if (!onmenu) return;
    e.preventDefault();
    onmenu(e.clientX, e.clientY);
  }
  function punaise(e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    onepingler?.();
  }
</script>

<!-- Mosaïque dès quatre pochettes, une seule sinon, et à défaut la couleur et l'icône de la playlist. -->
{#snippet mosaique(classe: string, tailleIcone: number)}
  <span class="relative block overflow-hidden isolate {classe}" style="background: linear-gradient(140deg, {couleur}, color-mix(in oklab, {couleur} 45%, black))">
    {#if couvertures.length >= 4}
      <span class="absolute inset-0 grid grid-cols-2 grid-rows-2 gap-px bg-black/20">
        {#each couvertures.slice(0, 4) as c (c)}<CoverImg path={c} alt="" size="2x" class="w-full h-full object-cover" />{/each}
      </span>
    {:else if couvertures.length > 0}
      <CoverImg path={couvertures[0]} alt="" size="2x" class="absolute inset-0 w-full h-full object-cover" />
    {:else}
      <span class="absolute inset-0 flex items-center justify-center text-white/90"><Icon icon={icone} width={tailleIcone} /></span>
    {/if}
    <span class="absolute inset-0 ring-1 ring-inset ring-black/8 dark:ring-white/8 pointer-events-none" style="border-radius: inherit"></span>
  </span>
{/snippet}

<!-- La punaise est posée à côté du lien, pas dedans : un bouton dans un lien serait invalide. -->
{#snippet punaiseBouton(classe: string, taille: number)}
  {#if epinglee != null}
    <button type="button" onclick={punaise} title={epinglee ? $t("sidebar.unpin") : $t("sidebar.pin")} aria-pressed={epinglee}
            class="flex items-center justify-center rounded-full cursor-pointer transition {classe}">
      <Icon icon={epinglee ? "material-symbols:keep-rounded" : "material-symbols:keep-outline-rounded"} width={taille} />
    </button>
  {/if}
{/snippet}

{#if liste}
  <div class="group flex items-center gap-2 rounded-xl hover:bg-(--rg-hover) transition-colors">
    <a {href} class="flex-1 min-w-0 flex items-center gap-3.5 px-2 py-2" oncontextmenu={menu} onclick={(e) => { e.preventDefault(); goto(href); }}>
      {@render mosaique("w-12 h-12 shrink-0 rounded-lg", 22)}
      <span class="flex-1 min-w-0">
        <span class="flex items-center gap-2">
          <span class="truncate text-[15px] font-semibold text-(--rg-tx)">{nom}</span>
          {#if auto}<Icon icon="material-symbols:auto-awesome-outline-rounded" width="15" class="shrink-0 text-violet-500 dark:text-violet-300" />{/if}
        </span>
        <span class="block truncate text-[13px] text-(--rg-mu)">{sousTitre}</span>
      </span>
    </a>
    {@render punaiseBouton(`shrink-0 mr-2 w-9 h-9 ${epinglee ? "text-(--rg-gtx)" : "text-(--rg-mu2) opacity-0 group-hover:opacity-100 focus-visible:opacity-100 hover:text-(--rg-tx)"}`, 19)}
  </div>
{:else}
  <div class="group relative min-w-0">
    <a {href} class="min-w-0 flex flex-col gap-2.5" oncontextmenu={menu} onclick={(e) => { e.preventDefault(); goto(href); }}>
      <span class="relative block">
        {@render mosaique("aspect-square w-full rounded-2xl shadow-[0_10px_26px_rgba(0,0,0,0.18)] dark:shadow-[0_10px_26px_rgba(0,0,0,0.5)] transition-transform duration-200 group-hover:-translate-y-0.5", 44)}
        {#if auto}
          <span class="absolute left-2 top-2 h-6 px-2 inline-flex items-center gap-1 rounded-full text-[11px] font-bold bg-black/55 text-white backdrop-blur-sm">
            <Icon icon="material-symbols:auto-awesome-rounded" width="13" />{$t("playlists_view.smart_badge")}
          </span>
        {/if}
      </span>
      <span class="min-w-0 px-0.5">
        <span class="block truncate text-[15px] font-bold text-(--rg-tx)">{nom}</span>
        <span class="block truncate text-[13px] text-(--rg-mu)">{sousTitre}</span>
      </span>
    </a>
    {@render punaiseBouton(`absolute right-2 top-2 w-8 h-8 bg-black/55 text-white backdrop-blur-sm hover:bg-black/75 ${epinglee ? "" : "opacity-0 group-hover:opacity-100 focus-visible:opacity-100"}`, 17)}
  </div>
{/if}
