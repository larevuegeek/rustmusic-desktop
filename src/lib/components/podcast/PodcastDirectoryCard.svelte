<script lang="ts">
  // Carte d'un podcast de l'annuaire : pochette, titre, auteur ; « + » pour s'abonner.
  import Icon from "@iconify/svelte";
  import { t } from "#lib/i18n";
  import PodcastCover from "./PodcastCover.svelte";
  import PodcastContextMenu from "#lib/components/ui/contextmenu/PodcastContextMenu.svelte";
  import type { DirectoryResult } from "#lib/types/ui/podcast/Podcast";

  let { podcast, rang = null, suivi = false, busy = false, onopen, onfollow }: {
    podcast: DirectoryResult;
    /** Place au classement, affichée sur la pochette. */
    rang?: number | null;
    suivi?: boolean;
    busy?: boolean;
    /** Clic sur la carte : ouvrir (abonné) ou s'abonner puis ouvrir. */
    onopen: () => void;
    /** « + » : s'abonner sans quitter la page. */
    onfollow: () => void;
  } = $props();

  // « … » (sous le bouton) ou clic droit (sous le pointeur).
  let menu = $state<{ x: number; y: number; top: number } | null>(null);
  function openMenu(e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    menu = e.type === "contextmenu" ? { x: e.clientX, y: e.clientY, top: e.clientY } : { x: r.left, y: r.bottom + 4, top: r.top - 4 };
  }
</script>

<div class="group relative min-w-0" role="presentation" oncontextmenu={openMenu}>
  <button type="button" onclick={onopen} class="w-full min-w-0 flex flex-col gap-2.5 text-left cursor-pointer">
    <span class="relative block aspect-square w-full overflow-hidden rounded-2xl bg-(--rg-s2)
                 shadow-[0_10px_26px_rgba(0,0,0,0.16)] dark:shadow-[0_10px_26px_rgba(0,0,0,0.5)] transition-transform duration-200 group-hover:-translate-y-0.5">
      {#if podcast.image_url}
        <img src={podcast.image_url} alt="" loading="lazy" decoding="async" class="absolute inset-0 w-full h-full object-cover" />
      {:else}
        <PodcastCover path={null} icone={40} class="w-full h-full" />
      {/if}
      <span class="absolute inset-0 rounded-2xl ring-1 ring-inset ring-black/8 dark:ring-white/8 pointer-events-none"></span>
      {#if rang != null}
        <span class="absolute left-2 top-2 min-w-7 h-7 px-1.5 inline-flex items-center justify-center rounded-full text-[12px] font-extrabold tabular-nums bg-black/60 text-white backdrop-blur-sm">{rang}</span>
      {/if}
    </span>
    <span class="min-w-0 pl-0.5 pr-8">
      <span class="block truncate text-[14.5px] font-bold text-(--rg-tx)">{podcast.title}</span>
      <span class="block truncate text-[12.5px] text-(--rg-mu)">{podcast.author ?? podcast.genre ?? ""}</span>
    </span>
  </button>
  <!-- Frère de la carte : un bouton dans un bouton n'est pas du HTML valide. -->
  <button type="button" onclick={onfollow} disabled={suivi || busy}
          title={suivi ? $t("podcasts.following") : $t("podcasts.follow")} aria-label={suivi ? $t("podcasts.following") : $t("podcasts.follow")}
          class="absolute right-2 top-2 w-8.5 h-8.5 rounded-full flex items-center justify-center transition backdrop-blur-sm
                 {suivi ? 'bg-(--rg-g) text-(--rg-on-g) cursor-default' : 'bg-black/60 text-white cursor-pointer hover:bg-violet-600 opacity-0 group-hover:opacity-100 focus-visible:opacity-100'}
                 {busy ? 'opacity-100!' : ''}">
    <Icon icon={busy ? "material-symbols:progress-activity" : suivi ? "material-symbols:check-rounded" : "material-symbols:add-rounded"} width="20" class={busy ? "animate-spin" : ""} />
  </button>
  <button type="button" onclick={openMenu} title={$t("common.menu")} aria-label={$t("common.menu")} aria-expanded={menu !== null}
          class="absolute right-0 bottom-0.5 w-8 h-8 rounded-full flex items-center justify-center cursor-pointer transition text-(--rg-mu) hover:bg-(--rg-s2) hover:text-(--rg-tx)
                 {menu ? 'bg-(--rg-s2) text-(--rg-tx)' : 'opacity-0 group-hover:opacity-100 focus-visible:opacity-100'}">
    <Icon icon="material-symbols:more-horiz" width="20" />
  </button>
</div>

{#if menu}
  <PodcastContextMenu x={menu.x} y={menu.y} top={menu.top} podcast={podcast} onclose={() => (menu = null)} />
{/if}
