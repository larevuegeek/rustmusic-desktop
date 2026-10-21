<script lang="ts">
  // Carte d'un abonnement : pochette, titre, auteur ; pastille des nouveaux épisodes,
  // étoile des favoris (épinglés dans la barre latérale).
  import Icon from "@iconify/svelte";
  import { t, currentLocale } from "#lib/i18n";
  import { podcastStore } from "#lib/stores/podcast/podcast.store";
  import { toasts } from "#lib/stores/ui/toast.store";
  import PodcastCover from "./PodcastCover.svelte";
  import PodcastContextMenu from "#lib/components/ui/contextmenu/PodcastContextMenu.svelte";
  import type { PodcastView } from "#lib/types/ui/podcast/Podcast";

  let { podcast }: { podcast: PodcastView } = $props();

  async function togglePinned(e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    try {
      await podcastStore.setPinned(podcast.id, !podcast.pinned);
    } catch (err) {
      toasts.push({ type: "error", title: $t("notify.error"), message: String(err) });
    }
  }

  const fiche = $derived({ title: podcast.title, author: podcast.author, feed_url: podcast.feed_url, image_url: podcast.image_url,
                           genre: null, episode_count: null, released_at: null, source: "rss" });

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
  <a href="/podcasts/{podcast.id}" class="min-w-0 flex flex-col gap-2.5">
    <span class="relative block">
      <PodcastCover path={podcast.cover_path} icone={44}
                    class="aspect-square w-full rounded-2xl shadow-[0_10px_26px_rgba(0,0,0,0.18)] dark:shadow-[0_10px_26px_rgba(0,0,0,0.5)] transition-transform duration-200 group-hover:-translate-y-0.5" />
      {#if podcast.new_count > 0}
        <span class="absolute left-2 top-2 h-6 px-2 inline-flex items-center rounded-full text-[11px] font-bold bg-(--rg-g) text-(--rg-on-g) shadow-sm">
          {podcast.new_count.toLocaleString($currentLocale)} {$t(podcast.new_count === 1 ? "podcasts.new_short_one" : "podcasts.new_short_n")}
        </span>
      {/if}
    </span>
    <span class="min-w-0 pl-0.5 pr-8">
      <span class="block truncate text-[15px] font-bold text-(--rg-tx)">{podcast.title}</span>
      <span class="block truncate text-[13px] text-(--rg-mu)">{podcast.author ?? ""}</span>
    </span>
  </a>
  <!-- Posée à côté du lien, pas dedans : un bouton dans un lien serait invalide. -->
  <button type="button" onclick={togglePinned} aria-pressed={podcast.pinned}
          title={podcast.pinned ? $t("podcasts.unfavorite") : $t("podcasts.favorite")}
          aria-label={podcast.pinned ? $t("podcasts.unfavorite") : $t("podcasts.favorite")}
          class="absolute right-2 top-2 w-8 h-8 rounded-full flex items-center justify-center cursor-pointer transition backdrop-blur-sm
                 {podcast.pinned ? 'bg-black/55 text-amber-300' : 'bg-black/55 text-white opacity-0 group-hover:opacity-100 focus-visible:opacity-100 hover:bg-black/75'}">
    <Icon icon={podcast.pinned ? "material-symbols:star-rounded" : "material-symbols:star-outline-rounded"} width="19" />
  </button>
  <button type="button" onclick={openMenu} title={$t("common.menu")} aria-label={$t("common.menu")} aria-expanded={menu !== null}
          class="absolute right-0 bottom-0.5 w-8 h-8 rounded-full flex items-center justify-center cursor-pointer transition text-(--rg-mu) hover:bg-(--rg-s2) hover:text-(--rg-tx)
                 {menu ? 'bg-(--rg-s2) text-(--rg-tx)' : 'opacity-0 group-hover:opacity-100 focus-visible:opacity-100'}">
    <Icon icon="material-symbols:more-horiz" width="20" />
  </button>
</div>

{#if menu}
  <PodcastContextMenu x={menu.x} y={menu.y} top={menu.top} podcast={fiche} onclose={() => (menu = null)} />
{/if}
