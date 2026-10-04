<script lang="ts">
// Vos playlists sur l'accueil : les épinglées d'abord, puis les plus récentes.
import { couverturesPlaylists } from "$lib/stores/playlist/couvertures.store";
import { t, currentLocale } from "$lib/i18n";
import { playlistStore } from "$lib/stores/playlist/playlist.store";
import Carousel from "$lib/components/ui/carousel/Carousel.svelte";
import PlaylistCard from "$lib/components/playlist/PlaylistCard.svelte";
import type { Playlist } from "$lib/types/db/playlist/Playlist";


const MAX = 12;
const affichees = $derived(
  $playlistStore.playlists.filter((p) => !p.is_mix)
    .sort((x, y) => Number(y.pinned) - Number(x.pinned) || y.created_at.localeCompare(x.created_at))
    .slice(0, MAX),
);

function sousTitre(p: Playlist) {
  const genre = p.is_smart ? $t("sidebar.playlist_auto") : $t("sidebar.playlist");
  if (p.track_count === 0) return p.is_smart ? genre : `${genre} · ${$t("sidebar.empty")}`;
  const n = p.track_count;
  return `${genre} · ${n === 1 ? $t("home.track_one") : $t("home.tracks_n").replace("{n}", n.toLocaleString($currentLocale))}`;
}
</script>

{#if affichees.length > 0}
<section class="flex flex-col gap-4">
  <div class="flex items-baseline justify-between gap-4">
    <h2 class="text-2xl font-bold tracking-tight text-neutral-900 dark:text-neutral-100">{$t("playlists_view.home_title")}</h2>
    <span class="hidden @3xl:inline flex-1 text-sm text-neutral-500 dark:text-[#9aa39e]">{$t("playlists_view.home_desc")}</span>
    <a href="/playlists" class="shrink-0 text-sm font-semibold text-emerald-600 dark:text-emerald-400 hover:underline">{$t("playlists_view.all")}</a>
  </div>
  <Carousel axe="80px" class="gap-4 @4xl:gap-5 pt-1.5 pb-2">
    {#each affichees as p (p.id)}
      <div class="w-40 @4xl:w-44 shrink-0 snap-start">
        <PlaylistCard href={`/playlist/${p.id}`} nom={p.name} sousTitre={sousTitre(p)} couleur={p.color} icone={p.icon}
                      auto={p.is_smart} couvertures={$couverturesPlaylists[`playlist:${p.id}`] ?? []} />
      </div>
    {/each}
  </Carousel>
</section>
{/if}
