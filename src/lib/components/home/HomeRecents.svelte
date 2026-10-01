<script lang="ts">
import Icon from "@iconify/svelte";
import { goto } from "$app/navigation";
import { t, currentLocale } from "$lib/i18n";
import { recent } from "$lib/stores/recent/recent.store";
import { lecture } from "$lib/stores/player/lecture.store";
import { liked } from "$lib/stores/playlist/like.store";
import { handlePlayTrack } from "$lib/actions/player/PlayerAction";
import { handleRemoveRecentItem } from "$lib/actions/recent/RecentAction";
import { versFileDAttente } from "$lib/mapper/queue/mapQueueTrack";
import { mapRecentFile } from "$lib/mapper/recent/mapRecentFile";
import { localiser, lienTitre, lienArtiste, lienAlbum, fileDeLAlbum, type TrackLocation } from "$lib/helper/library/trackLocation";
import { grouperParAlbum } from "$lib/helper/recent/grouperParAlbum";
import { ilYA } from "$lib/helper/tools/dateTools";
import { displayTitle, artistesLisibles } from "$lib/helper/tools/stringTools";
import CoverImg from "$lib/components/ui/image/CoverImg.svelte";
import TrackContextMenu from "$lib/components/ui/contextmenu/TrackContextMenu.svelte";
import type { RecentFileListView } from "$lib/types/ui/recent/RecentFileListView";

// Le hero montre déjà le morceau chargé (ou le dernier écouté) : on part du suivant.
const cheminCourant = $derived($lecture.path);
const historique = $derived.by(() => {
  const exclu = cheminCourant ?? $recent[0]?.path;
  return $recent.filter((r) => r.path !== exclu);
});

const vedette = $derived(historique[0] ?? null);
// Le premier album est celui de la vedette : la bande commence au suivant.
const bande = $derived(grouperParAlbum(historique).slice(1, 6));

let lieux = $state(new Map<string, TrackLocation | null>());
$effect(() => {
  const chemins = [vedette?.path, ...bande.map((a) => a.pistes[0].path)].filter(Boolean) as string[];
  let perime = false;
  localiser(chemins).then((m) => { if (!perime) lieux = m; });
  return () => { perime = true; };
});

let occupe = $state<string | null>(null);

/** Joue dans son album quand il est en bibliothèque, sinon dans l'historique. */
async function jouer(cle: string, path: string, depuisLeDebut = false) {
  if (occupe) return;
  occupe = cle;
  try {
    const file = await fileDeLAlbum(path);
    if (depuisLeDebut && file?.length) await handlePlayTrack(file[0].path, file);
    else await handlePlayTrack(path, file ?? versFileDAttente(historique));
  } finally {
    occupe = null;
  }
}

// Les cartes au-delà du nombre de colonnes restent cachées.
const visibilite = ['flex', 'flex', 'flex', 'hidden @6xl:flex', 'hidden @7xl:flex'];

// Avec pochette, la carte prend le fond du hero (pochette floutée) : sombre, texte clair.
const STYLES = {
  fond: {
    carte: "bg-[#121614] border-white/10",
    titre: "text-white", sous: "text-white/70", survol: "hover:text-white", surtitre: "text-[#4ade80]",
    boutonFond: "bg-white/10 hover:bg-white/16", boutonTexte: "text-white",
  },
  uni: {
    carte: "bg-neutral-50 border-neutral-200 dark:bg-[#131715] dark:border-[#1f2522]",
    titre: "text-neutral-900 dark:text-[#f2f5f3]", sous: "text-neutral-500 dark:text-[#9aa39e]",
    survol: "hover:text-neutral-900 dark:hover:text-white", surtitre: "text-emerald-600 dark:text-[#22c55e]",
    boutonFond: "bg-neutral-900/6 hover:bg-neutral-900/10 dark:bg-[#1e2421] dark:hover:bg-[#262d29]",
    boutonTexte: "text-neutral-900 dark:text-[#f2f5f3]",
  },
};

const nbTitres = (n: number) => n === 1 ? $t('home.track_one') : $t('home.tracks_n').replace('{n}', String(n));

let menu = $state<{ x: number; y: number; track: RecentFileListView } | null>(null);

function ouvrirMenu(e: MouseEvent, track: RecentFileListView) {
  e.preventDefault();
  menu = { x: e.clientX, y: e.clientY, track };
}
</script>

{#snippet lien(texte: string | null | undefined, vers: string | null, classes: string, survol: string)}
  {#if texte}
    {#if vers}
      <a href={vers} class="{classes} {survol} hover:underline">{texte}</a>
    {:else}
      <span class={classes}>{texte}</span>
    {/if}
  {/if}
{/snippet}

{#if vedette}
{@const titre = vedette.title ?? displayTitle(null, vedette.path, $t('common.unknown_title'))}
{@const lieu = lieux.get(vedette.path) ?? null}
{@const aime = $liked.paths.has(vedette.path)}
{@const c = vedette.thumbnail_path ? STYLES.fond : STYLES.uni}
<section id="recemment-joues" class="scroll-mt-6 flex flex-col gap-5">
  <div class="flex items-baseline justify-between gap-4">
    <h2 class="text-2xl font-bold tracking-tight text-neutral-900 dark:text-neutral-100">{$t('home.recently_played')}</h2>
    <div class="flex items-center gap-6">
      <button type="button" onclick={() => recent.clearRecent()}
              class="text-sm text-neutral-500 dark:text-[#9aa39e] hover:text-neutral-800 dark:hover:text-neutral-200 cursor-pointer transition-colors">
        {$t('home.clear_history')}
      </button>
      <button type="button" onclick={() => goto('/playlist/recent')}
              class="text-sm font-semibold text-emerald-600 dark:text-[#22c55e] hover:underline cursor-pointer">
        {$t('home.see_all')}
      </button>
    </div>
  </div>

  <div class="grid grid-cols-1 @3xl:grid-cols-[340px_minmax(0,1fr)] @6xl:grid-cols-[400px_minmax(0,1fr)] gap-7 items-start">

    <!-- Le dernier morceau écouté -->
    <div role="group" oncontextmenu={(e) => ouvrirMenu(e, vedette)}
         class="recent-vedette relative overflow-hidden flex flex-col gap-4 p-5 rounded-2xl border min-w-0 {c.carte}">
      {#if vedette.thumbnail_path}
        <div class="absolute inset-0 pointer-events-none" aria-hidden="true">
          <CoverImg path={vedette.thumbnail_path} size="2x"
                    class="w-full h-full object-cover scale-125 blur-[36px] saturate-[1.8] opacity-95" />
        </div>
        <div class="vedette-voile absolute inset-0 pointer-events-none" aria-hidden="true"></div>
      {/if}

      <div class="relative flex items-center gap-4.5 min-w-0">
        <button type="button" onclick={() => jouer('vedette', vedette.path)} aria-label={titre}
                class="group relative w-30 h-30 rounded-[10px] overflow-hidden shrink-0 cursor-pointer
                       bg-neutral-200 dark:bg-neutral-800 shadow-[0_10px_30px_rgba(0,0,0,0.45)]">
          {#if vedette.thumbnail_path}
            <CoverImg path={vedette.thumbnail_path} size="2x" class="w-full h-full object-cover" />
          {:else}
            <span class="w-full h-full flex items-center justify-center">
              <Icon icon="lucide:music" width={32} class="text-neutral-400" />
            </span>
          {/if}
          <span class="absolute inset-0 flex items-center justify-center bg-black/40 text-white
                       opacity-0 group-hover:opacity-100 transition-opacity duration-150">
            <Icon icon="mynaui:play-solid" width={30} />
          </span>
        </button>

        <div class="flex flex-col gap-1 min-w-0">
          <span class="text-[12px] font-bold uppercase tracking-widest truncate {c.surtitre}">
            {$t('home.played_ago').replace('{ago}', ilYA(vedette.last_played_at, $currentLocale, "short"))}
          </span>
          {@render lien(titre, lienTitre(lieu),
            `text-[24px] font-extrabold tracking-[-0.01em] leading-[1.1] line-clamp-2 ${c.titre}`, c.survol)}
          <span class="text-[15px] truncate {c.sous}">
            {@render lien(artistesLisibles(vedette.artist), lienArtiste(lieu), "", c.survol)}{#if vedette.artist && vedette.album}{" · "}{/if}{@render lien(vedette.album, lienAlbum(lieu), "", c.survol)}
          </span>
        </div>
      </div>

      <div class="relative flex items-center gap-2.5 min-w-0">
        <button type="button" onclick={() => jouer('vedette', vedette.path)} disabled={occupe !== null}
                class="h-11 px-5 rounded-full bg-[#22c55e] hover:bg-[#4ade80] text-(--rg-on-g) font-bold text-[15px]
                       flex items-center gap-2 shrink-0 whitespace-nowrap cursor-pointer transition-colors
                       disabled:opacity-60 disabled:cursor-wait">
          <Icon icon="mynaui:play-solid" width={18} /> {$t('home.replay')}
        </button>
        {#if lienAlbum(lieu)}
          <button type="button" onclick={() => jouer('vedette', vedette.path, true)} disabled={occupe !== null}
                  class="h-11 px-4.5 rounded-full font-semibold text-[15px] min-w-0 cursor-pointer transition-colors
                         disabled:opacity-60 disabled:cursor-wait {c.boutonFond} {c.boutonTexte}">
            <span class="block truncate">{$t('library.play_album')}</span>
          </button>
        {/if}
        <button type="button" onclick={() => liked.toggle(vedette.path)}
                aria-label={$t('home.like')} title={$t('home.like')}
                class="favori shrink-0 w-11 h-11 rounded-full flex items-center justify-center cursor-pointer transition-colors
                       {c.boutonFond} {aime ? 'text-pink-500 dark:text-pink-400' : c.boutonTexte}">
          <Icon icon={aime ? "mynaui:heart-solid" : "mynaui:heart"} width={18} />
        </button>
      </div>
    </div>

    <!-- Les albums écoutés avant -->
    {#if bande.length > 0}
      <!-- Autant d'albums que de colonnes : 3, 4 puis 5 selon la place. -->
      <div class="grid grid-cols-3 @6xl:grid-cols-4 @7xl:grid-cols-5 gap-4" role="list">
        {#each bande as a, i (a.cle)}
          {@const lieuA = lieux.get(a.pistes[0].path) ?? null}
          <div role="listitem"
               class="flex-col gap-2 min-w-0 {visibilite[i] ?? 'flex'}">
            <button type="button" onclick={() => jouer(a.cle, a.pistes[0].path)}
                    title={$t('home.resume_album')} aria-label="{$t('home.resume_album')} : {a.album}"
                    class="group relative aspect-square rounded-[10px] overflow-hidden cursor-pointer
                           bg-neutral-200 dark:bg-neutral-800 shadow-[0_8px_24px_rgba(0,0,0,0.3)]
                           transition-transform duration-300 hover:-translate-y-1">
              {#if a.pochette}
                <CoverImg path={a.pochette} size="2x" class="w-full h-full object-cover" />
              {:else}
                <span class="w-full h-full flex items-center justify-center">
                  <Icon icon="lucide:disc-3" width={36} class="text-neutral-400" />
                </span>
              {/if}
              <span class="absolute inset-0 flex items-center justify-center bg-black/30
                           opacity-0 group-hover:opacity-100 transition-opacity duration-200">
                <span class="w-12 h-12 rounded-full bg-[#22c55e] text-(--rg-on-g) flex items-center justify-center
                             shadow-[0_8px_20px_rgba(0,0,0,0.4)]">
                  <Icon icon={occupe === a.cle ? "lucide:loader-circle" : "mynaui:play-solid"} width={22}
                        class={occupe === a.cle ? "animate-spin" : ""} />
                </span>
              </span>
              <span class="absolute right-2 bottom-2 px-2 py-0.75 rounded-[10px] bg-black/70 text-white
                           text-[12px] font-semibold leading-tight">
                {nbTitres(a.pistes.length)}
              </span>
            </button>

            <div class="flex flex-col min-w-0 px-0.5">
              {@render lien(a.album, lienAlbum(lieuA),
                "text-[15px] font-semibold truncate text-neutral-900 dark:text-[#f2f5f3]", STYLES.uni.survol)}
              <span class="text-[13px] truncate text-neutral-500 dark:text-[#9aa39e]">
                {@render lien(a.artiste, lienArtiste(lieuA), "", STYLES.uni.survol)}{#if a.artiste}{" · "}{/if}{ilYA(a.derniereEcoute, $currentLocale, "short")}
              </span>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</section>
{/if}

{#if menu}
  <!-- « Supprimer » retire de l'historique, pas du disque. -->
  <TrackContextMenu
    track={menu.track}
    x={menu.x}
    y={menu.y}
    showDelete={true}
    deleteLabel={$t('home.remove_from_history')}
    ondelete={() => handleRemoveRecentItem(mapRecentFile(menu!.track))}
    onclose={() => menu = null}
  />
{/if}
