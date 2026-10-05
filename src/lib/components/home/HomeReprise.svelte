<script lang="ts">
import Icon from "@iconify/svelte";
import { invoke } from "@tauri-apps/api/core";
import { t, currentLocale } from "$lib/i18n";
import { player } from "$lib/stores/player/player.store";
import { queueState } from "$lib/stores/queue/queueState.store";
import { recent } from "$lib/stores/recent/recent.store";
import { liked } from "$lib/stores/playlist/like.store";
import { playerService } from "$lib/services/player/player.service";
import { openQueuePanel } from "$lib/stores/queue/queueUi.store";
import { handlePlayTrack } from "$lib/actions/player/PlayerAction";
import { versFileDAttente } from "$lib/mapper/queue/mapQueueTrack";
import { fileDeLAlbum } from "$lib/helper/library/trackLocation";
import { formatTime, dateToYear, ilYA } from "$lib/helper/tools/dateTools";
import { displayTitle } from "$lib/helper/tools/stringTools";
import CoverImg from "$lib/components/ui/image/CoverImg.svelte";
import PochetteFloue from "$lib/components/ui/image/PochetteFloue.svelte";
import type { QueueTrack } from "$lib/types/db/queue/QueueTrack";
import type { TrackListView } from "$lib/types/ui/library/track/TrackListView";

let { onajouter, libraryId = null, ancreRecents = false }:
  { onajouter: () => void; libraryId?: number | null; ancreRecents?: boolean } = $props();

// Ce qui joue a la priorité ; sinon, le dernier morceau écouté.
const chemin = $derived($player?.pathFile ?? null);
const audioFile = $derived($player?.audioFile ?? null);
const enCours = $derived(!!chemin && !!audioFile);
const isPlaying = $derived($player?.status === "playing");
const dernier = $derived($recent[0] ?? null);
const qs = $derived($queueState);
const pisteFile = $derived(qs.tracks[qs.currentIndex] ?? null);

type Fiche = {
  path: string; titre: string; artiste: string; album: string;
  annee: string; duree: number; pochette: string | null;
};

const fiche = $derived.by((): Fiche | null => {
  if (enCours && chemin) {
    const tags = audioFile?.tags;
    return {
      path: chemin,
      titre: displayTitle(tags?.title, chemin, $t('common.unknown_title')),
      artiste: tags?.artist ?? "",
      album: tags?.album ?? "",
      annee: tags?.year ? String(dateToYear(tags.year)) : "",
      duree: audioFile?.duration ?? 0,
      pochette: (pisteFile?.path === chemin ? pisteFile?.cover : null)
        ?? tags?.attached_images?.[0]?.image_src ?? null,
    };
  }
  if (dernier) {
    return {
      path: dernier.path,
      titre: dernier.title ?? displayTitle(null, dernier.path, $t('common.unknown_title')),
      artiste: dernier.artist ?? "",
      album: dernier.album ?? "",
      annee: dernier.year ? String(dateToYear(dernier.year)) : "",
      duree: dernier.duration ?? 0,
      pochette: dernier.thumbnail_path ?? null,
    };
  }
  return null;
});

// La position vivante n'est lue que dans le gabarit : jamais dans un effet.
const positionSauvee = $derived(!enCours ? (dernier?.last_position ?? 0) : 0);

// Quatre lignes, comme la maquette : la file, puis les derniers écoutés, puis du hasard.
const TOTAL = 4;

// Un morceau sans vrai titre n'est pas une suite montrable.
const ensuite = $derived(
  qs.tracks.slice(qs.currentIndex + 1)
    .map((track, i) => ({ track, index: qs.currentIndex + 1 + i }))
    .filter(({ track }) => track.title && track.title !== "Inconnu")
    .slice(0, TOTAL)
);

const dejaVus = $derived(new Set([fiche?.path, ...ensuite.map((e) => e.track.path)]));
const recentsSuivants = $derived(
  $recent.filter((r) => !dejaVus.has(r.path)).slice(0, TOTAL - ensuite.length)
);

// Le hasard ne comble que ce qui manque, tiré une seule fois.
let auHasard = $state<TrackListView[]>([]);
const manque = $derived(TOTAL - ensuite.length - recentsSuivants.length);

// Un tirage par bibliothèque : un résultat vide ne doit pas relancer l'effet en boucle.
let tirePour: number | null = null;
$effect(() => {
  const id = libraryId;
  if (manque <= 0 || !id || tirePour === id) return;
  tirePour = id;
  auHasard = [];
  invoke<TrackListView[]>("get_mix_tracks", { libraryId: id, kind: "hasard", limit: TOTAL })
    .then((t) => { if (tirePour === id && t?.length) auHasard = t; })
    .catch(() => {});
});

type Carte = {
  cle: string; titre: string; artiste: string; duree: number | null;
  pochette: string | null; lancer: () => void;
};
type Groupe = { cle: "file" | "recents" | "hasard"; libelle: string; cartes: Carte[] };

const groupes = $derived.by((): Groupe[] => {
  const liste: Groupe[] = [];
  if (ensuite.length) liste.push({
    cle: "file", libelle: $t('home.up_next'),
    cartes: ensuite.map(({ track, index }) => ({
      cle: `f:${track.queueId}`, titre: track.title, pochette: track.cover ?? null,
      artiste: track.artist ?? '', duree: track.duration ?? null,
      lancer: () => jouerDeLaFile(track, index),
    })),
  });
  if (recentsSuivants.length) liste.push({
    cle: "recents", libelle: $t('home.recently_played'),
    cartes: recentsSuivants.map((r) => ({
      cle: `r:${r.path}`, titre: r.title ?? displayTitle(null, r.path, $t('common.unknown_title')),
      pochette: r.thumbnail_path ?? null, artiste: r.artist ?? '', duree: r.duration ?? null,
      lancer: () => handlePlayTrack(r.path, versFileDAttente($recent)),
    })),
  });
  const hasard = auHasard.filter((p) => !dejaVus.has(p.path)).slice(0, Math.max(0, manque));
  if (hasard.length) liste.push({
    cle: "hasard", libelle: $t('home.random_pick'),
    cartes: hasard.map((p) => ({
      cle: `h:${p.path}`, titre: p.title, pochette: p.thumbnail_path ?? null,
      artiste: p.artist ?? '', duree: p.duration ?? null,
      lancer: () => handlePlayTrack(p.path, versFileDAttente(hasard)),
    })),
  });
  return liste;
});

let occupe = $state(false);

/** Le bouton vert et la pochette font la même chose. */
/** Descend jusqu'à la section complète des récents. */
function allerAuxRecents() {
  document.getElementById("recemment-joues")?.scrollIntoView({ behavior: "smooth", block: "start" });
}

function actionPrincipale() {
  if (enCours) playerService.handleTogglePlay();
  else reprendre();
}

async function reprendre() {
  const cible = dernier;
  if (!cible || occupe) return;
  occupe = true;
  try {
    const file = await fileDeLAlbum(cible.path);
    await handlePlayTrack(cible.path, file ?? undefined);
    const pos = cible.last_position ?? 0;
    if (pos > 5 && await playerService.attendrePret()) await playerService.seekTo(pos);
  } finally {
    occupe = false;
  }
}

async function lireAlbum() {
  if (!fiche || occupe) return;
  occupe = true;
  try {
    const file = await fileDeLAlbum(fiche.path);
    if (file?.length) await handlePlayTrack(file[0].path, file);
    else await handlePlayTrack(fiche.path);
  } finally {
    occupe = false;
  }
}

async function jouerDeLaFile(track: QueueTrack, index: number) {
  playerService.expectExplicitAction();
  await queueState.setCurrentIndex(index);
  await playerService.playFile(track);
}
</script>

{#if fiche}
<!-- Découpé sur la largeur du bloc : la barre latérale en mange une part. Tailles
     nommées seulement, pour que l'ordre des règles soit garanti. -->
<div class="@container">
<section class="hero-reprise relative overflow-hidden rounded-[20px] text-[#1a1c1a] dark:text-white
                grid items-center gap-6 p-6 grid-cols-1
                @2xl:grid-cols-[160px_minmax(0,1fr)]
                @3xl:grid-cols-[176px_minmax(0,1fr)_250px]
                @5xl:grid-cols-[220px_minmax(0,1fr)_260px] @5xl:gap-8 @5xl:p-8
                @7xl:grid-cols-[260px_minmax(0,1fr)_300px]">

  <!-- Fond : la pochette floutée, ou un vert discret sans pochette -->
  {#if fiche.pochette}
    <div class="hero-fond absolute inset-0 pointer-events-none" aria-hidden="true">
      <!-- Pré-floutée : un filtre CSS se recalculait à chaque pas de la progression.
           Une saturation par thème, la bonne affichée par le CSS. -->
      <PochetteFloue path={fiche.pochette} saturation={2.4} flou={3} rayon={60}
                     class="w-full h-full object-cover scale-125 opacity-80 dark:hidden" />
      <PochetteFloue path={fiche.pochette} saturation={1.4} flou={3} rayon={60}
                     class="w-full h-full object-cover scale-125 opacity-80 hidden dark:block" />
    </div>
    <div class="hero-voile absolute inset-0 pointer-events-none" aria-hidden="true"></div>
  {:else}
    <div class="hero-vert absolute inset-0 pointer-events-none" aria-hidden="true"></div>
  {/if}

  <!-- Pochette, cliquable comme le bouton principal -->
  <button type="button" onclick={actionPrincipale} disabled={occupe}
          aria-label={enCours ? (isPlaying ? $t('home.pause_short') : $t('home.play')) : $t('home.resume_at')}
          class="group relative w-36 @2xl:w-full aspect-square rounded-xl overflow-hidden bg-white/5 cursor-pointer
                 shadow-[0_18px_40px_rgba(0,0,0,0.2)] dark:shadow-[0_24px_60px_rgba(0,0,0,0.5)] disabled:cursor-wait">
    {#if fiche.pochette}
      <CoverImg path={fiche.pochette} alt={fiche.album} size="full" class="w-full h-full object-cover" />
    {:else}
      <span class="w-full h-full flex items-center justify-center bg-emerald-900/40">
        <Icon icon="lucide:disc-3" width={64} class="text-emerald-200/50" />
      </span>
    {/if}
    <span class="absolute inset-0 flex items-center justify-center bg-black/35
                 opacity-0 group-hover:opacity-100 transition-opacity duration-200">
      <span class="w-16 h-16 rounded-full bg-[#22c55e] text-(--rg-on-g) flex items-center justify-center shadow-xl">
        <Icon icon={enCours && isPlaying ? "mynaui:pause-solid" : "mynaui:play-solid"} width={28} />
      </span>
    </span>
  </button>

  <!-- Infos et actions -->
  <div class="relative flex flex-col gap-2.5 min-w-0">
    <p class="text-[13px] font-semibold uppercase tracking-[0.08em] text-black/55 dark:text-white/75 truncate">
      {#if enCours}
        {isPlaying ? $t('home.now_playing') : $t('home.paused')}
      {:else}
        {$t('home.where_you_left')}{#if dernier?.last_played_at} · {ilYA(dernier.last_played_at, $currentLocale)}{/if}
      {/if}
    </p>

    <h2 class="text-[28px] @3xl:text-[32px] @5xl:text-[38px] @7xl:text-[44px]
               font-extrabold tracking-[-0.025em] leading-[1.05] line-clamp-2" title={fiche.titre}>
      {fiche.titre}
    </h2>

    <p class="text-[16px] @5xl:text-[18px] @7xl:text-[19px] text-[#3d403c] dark:text-white/85 truncate"
       title={[fiche.artiste, fiche.album, fiche.annee].filter(Boolean).join(' · ')}>
      {[fiche.artiste, fiche.album, fiche.annee].filter(Boolean).join(' · ')}
    </p>

    {#if fiche.duree > 0}
      {@const position = enCours ? ($player?.jsPosition ?? 0) : positionSauvee}
      <div class="flex items-center gap-3 mt-2.5 text-sm font-mono tabular-nums text-[#5e625d] dark:text-white/75">
        <span>{formatTime(position)}</span>
        <div class="flex-1 h-[5px] rounded-full overflow-hidden bg-black/10 dark:bg-white/18">
          <div class="h-full bg-[#1a1c1a] dark:bg-white rounded-full" style="width: {Math.min(100, (position / fiche.duree) * 100)}%"></div>
        </div>
        <span>{formatTime(fiche.duree)}</span>
      </div>
    {/if}

    <div class="flex items-center gap-3 mt-3">
      <button type="button" onclick={actionPrincipale} disabled={occupe}
              class="h-[52px] px-[26px] rounded-full bg-[#1a1c1a] hover:bg-black text-white dark:bg-[#22c55e] dark:hover:bg-[#4ade80] dark:text-(--rg-on-g)
                     font-bold text-[17px] flex items-center gap-2.5 cursor-pointer transition-colors shrink-0
                     disabled:opacity-60 disabled:cursor-wait">
        <Icon icon={enCours && isPlaying ? "mynaui:pause-solid" : "mynaui:play-solid"} width={20} />
        {#if enCours}
          {isPlaying ? $t('home.pause_short') : $t('home.play')}
        {:else}
          {positionSauvee > 5 ? $t('home.resume_at') : $t('home.listen')}
        {/if}
      </button>

      {#if fiche.album}
        <!-- Libellé entier quand la place le permet, bouton rond sinon -->
        <button type="button" onclick={lireAlbum} disabled={occupe}
                class="flex @3xl:hidden @7xl:flex items-center h-[52px] px-[22px] rounded-full
                       bg-white/70 hover:bg-white border border-black/8 text-[#1a1c1a]
                       dark:bg-white/10 dark:hover:bg-white/16 dark:border-transparent dark:text-white whitespace-nowrap font-semibold text-base
                       cursor-pointer transition-colors min-w-0 disabled:opacity-60 disabled:cursor-wait">
          <span class="truncate">{$t('home.play_album')}</span>
        </button>
        <button type="button" onclick={lireAlbum} disabled={occupe}
                aria-label={$t('home.play_album')} title={$t('home.play_album')}
                class="hidden @3xl:flex @7xl:hidden shrink-0 w-[52px] h-[52px] rounded-full items-center justify-center
                       bg-white/70 hover:bg-white border border-black/8 text-[#1a1c1a]
                       dark:bg-white/10 dark:hover:bg-white/16 dark:border-transparent dark:text-white cursor-pointer transition-colors
                       disabled:opacity-60 disabled:cursor-wait">
          <Icon icon="lucide:disc-3" width={20} />
        </button>
      {/if}

      <button type="button" onclick={() => liked.toggle(fiche.path)}
              aria-label={$t('home.like')} title={$t('home.like')}
              class="favori shrink-0 w-[52px] h-[52px] rounded-full flex items-center justify-center cursor-pointer
                     bg-white/70 hover:bg-white border border-black/8 dark:bg-white/10 dark:hover:bg-white/16 dark:border-transparent transition-colors
                     {$liked.paths.has(fiche.path) ? 'text-pink-500 dark:text-pink-400' : 'text-[#1a1c1a] dark:text-white'}">
        <Icon icon={$liked.paths.has(fiche.path) ? "mynaui:heart-solid" : "mynaui:heart"} width={20} />
      </button>
    </div>
  </div>

  <!-- Ensuite : un panneau à droite, ou sous le hero quand il est étroit -->
  {#if groupes.length}
  <div class="ensuite-panneau relative self-center min-w-0 col-span-full @3xl:col-span-1
              rounded-2xl p-3.5 @5xl:p-4">
    {#each groupes as g, gi (g.cle)}
      <div class="flex items-center justify-between gap-3 px-1.5 mb-1.5
                  {gi > 0 ? 'mt-3 pt-3 border-t border-black/8 dark:border-white/[0.08]' : ''}">
        <span class="text-[11.5px] font-bold uppercase tracking-[0.11em] text-[#5e625d] dark:text-white/60 truncate">{g.libelle}</span>
        {#if g.cle === "file"}
          <button type="button" onclick={openQueuePanel}
                  class="shrink-0 text-[12px] font-semibold text-[#15803d] hover:text-[#166534] dark:text-emerald-400 dark:hover:text-emerald-300 cursor-pointer transition-colors">
            {$t('home.see_queue')}
          </button>
        {:else if g.cle === "recents" && ancreRecents}
          <button type="button" onclick={allerAuxRecents}
                  class="shrink-0 text-[12px] font-semibold text-[#15803d] hover:text-[#166534] dark:text-emerald-400 dark:hover:text-emerald-300 cursor-pointer transition-colors">
            {$t('home.see_all')}
          </button>
        {/if}
      </div>
      <div class="grid grid-cols-1 @2xl:grid-cols-2 @3xl:grid-cols-1 gap-x-3">
        {#each g.cartes as c (c.cle)}
          <button type="button" onclick={c.lancer} title={c.titre}
                  class="group grid grid-cols-[44px_minmax(0,1fr)_auto] items-center gap-3 text-left
                         rounded-xl px-1.5 py-1.5 cursor-pointer min-w-0
                         hover:bg-black/[0.04] dark:hover:bg-white/[0.07] transition-colors duration-150">
            <span class="relative w-11 h-11 rounded-lg overflow-hidden bg-black/5 dark:bg-white/10
                         shadow-[0_4px_10px_rgba(0,0,0,0.12)] dark:shadow-[0_6px_16px_rgba(0,0,0,0.4)]">
              {#if c.pochette}<CoverImg path={c.pochette} size="1x" class="w-full h-full object-cover" />{/if}
              <span class="absolute inset-0 flex items-center justify-center bg-black/55 text-white
                           opacity-0 group-hover:opacity-100 transition-opacity duration-150">
                <Icon icon="mynaui:play-solid" width={16} />
              </span>
            </span>
            <span class="min-w-0 flex flex-col gap-px">
              <span class="text-[14px] font-semibold leading-tight text-[#1a1c1a] dark:text-white truncate">{c.titre}</span>
              <span class="text-[12.5px] leading-tight text-[#5e625d] dark:text-white/55 truncate">{c.artiste}</span>
            </span>
            <span class="hidden @5xl:inline text-[12px] font-mono tabular-nums text-[#5e625d] group-hover:text-[#1a1c1a]
                         dark:text-white/45 dark:group-hover:text-white/80 transition-colors">
              {c.duree ? formatTime(c.duree) : ''}
            </span>
          </button>
        {/each}
      </div>
    {/each}
  </div>
  {/if}
</section>
</div>

{:else}
<!-- Premier lancement : rien d'écouté encore -->
<section class="hero-reprise relative overflow-hidden rounded-[20px] p-8 md:p-10 text-[#1a1c1a] dark:text-white flex flex-col gap-4">
  <div class="hero-vert absolute inset-0 pointer-events-none" aria-hidden="true"></div>
  <p class="relative text-[13px] font-semibold uppercase tracking-[0.08em] text-black/55 dark:text-white/75">{$t('home.welcome_title')}</p>
  <h2 class="relative text-[34px] font-extrabold tracking-[-0.025em] leading-tight max-w-xl">{$t('home.welcome_desc')}</h2>
  <div class="relative">
    <button type="button" onclick={onajouter}
            class="h-[52px] px-[26px] rounded-full bg-[#1a1c1a] hover:bg-black text-white dark:bg-[#22c55e] dark:hover:bg-[#4ade80] dark:text-(--rg-on-g)
                   font-bold text-[17px] flex items-center gap-2.5 cursor-pointer transition-colors shrink-0">
      <Icon icon="lucide:plus" width={18} /> {$t('home.add_music')}
    </button>
  </div>
</section>
{/if}
