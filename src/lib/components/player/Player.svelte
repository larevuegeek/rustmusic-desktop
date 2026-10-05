<script lang="ts">
import { volume } from "#lib/stores/player/volume.store";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import Icon from "@iconify/svelte";
import AnneauAttente from "#lib/components/ui/loader/AnneauAttente.svelte";
import { t, currentLocale } from "#lib/i18n";
import PlayerProgressBar from "./PlayerProgressBar.svelte";
import PlayerSoundBar from "./PlayerSoundBar.svelte";
import { toggleQueuePanel } from "#lib/stores/queue/queueUi.store";
import { toggleLyricsPanel, lyricsPanelOpened } from "#lib/stores/lyrics/lyricsPanel.store";
import LyricsPanel from "#lib/components/lyrics/LyricsPanel.svelte";
import { player } from "#lib/stores/player/player.store";
import { queueState } from "#lib/stores/queue/queueState.store";
import { displayTitle } from "#lib/helper/tools/stringTools";
import { dateToYear, minutesSecondes } from "#lib/helper/tools/dateTools";
import ImgZoom from "#lib/components/ui/tools/ImgZoom.svelte";
import { playerService } from "#lib/services/player/player.service";
import { liked } from "#lib/stores/playlist/like.store";
import StatusBar from "#lib/components/ui/statusbar/StatusBar.svelte";
import { couleurMorceau } from "#lib/stores/player/trackColor.store";
import { dsdLabel, formatBitrate, formatDsdRate, isDsdFormat } from "#lib/helper/tools/audioFormatTools";
import { dlnaStatusStore } from "#lib/stores/dlna/dlna.store";
import { get } from "svelte/store";
import { localiserUn, lienTitre, lienArtiste, lienAlbum, lienAnnee, type TrackLocation } from "#lib/helper/library/trackLocation";
import { playbackPipelineStore, pipelineMode } from "#lib/stores/player/playbackPipeline.store";
import PipelineInfoPopover from "#lib/components/player/PipelineInfoPopover.svelte";
import { goto } from "$app/navigation";
import { settingsStore } from "#lib/stores/settings/settings.store";
import { detectOS } from "#lib/helper/tools/osDetection";
import { audioDevicesStore } from "#lib/stores/audio/audioDevices.store";
import { decrireSortie } from "#lib/helper/audio/deviceLabel";
import { TEINTE, CHAINE, messageRepli, frequenceLisible } from "#lib/helper/audio/chaineAudio";

const audioFile = $derived($player?.audioFile);
const audioTags = $derived(audioFile?.tags);
const hasTrack = $derived(!!$player?.pathFile);
const isPlaying = $derived($player?.status === "playing");
// Le son n'a pas encore démarré (ouverture du DAC, décodage).
const enAttente = $derived(isPlaying && !!$player?.isPreparing);
const duration = $derived($player?.duration ?? 0);
const jsPosition = $derived($player?.jsPosition ?? 0);
const jsPositionPercent = $derived(duration ? Math.min(100, Math.max(0, (jsPosition / duration) * 100)) : 0);
const coverSrc = $derived(audioTags?.attached_images?.[0]?.image_src ?? "/images/no-cd.png");

const annee = $derived(audioTags?.year ? dateToYear(audioTags.year) : "");

// Titre du tag, sinon nom de fichier (DSF/DFF sans DITI ni ID3).
const trackTitle = $derived(displayTitle(audioTags?.title, $player?.pathFile, $t("common.unknown_title")));

const surWindows = detectOS() === "windows";
const wasapiConfigured = $derived($settingsStore.wasapi_exclusive === "true");
const modeChaine = $derived(pipelineMode($playbackPipelineStore, $volume));
let showPipelinePopover = $state(false);

// ─── Fiche du morceau : titre, artiste et album mènent quelque part ───
// Le chemin seul, pas `$player` : la boucle de progression relançait l'effet à chaque image.
let lieu = $state<TrackLocation | null>(null);
const cheminLu = $derived($player?.pathFile ?? null);
let cheminResolu: string | null = null;

$effect(() => {
  const chemin = cheminLu;
  if (chemin === cheminResolu) return;
  cheminResolu = chemin;
  lieu = null;
  if (!chemin) return;
  localiserUn(chemin).then((trouve) => {
    if (get(player)?.pathFile === chemin) lieu = trouve;
  });
});

// ─── Ligne d'état ───
const nombre = $derived(new Intl.NumberFormat($currentLocale, { maximumFractionDigits: 1 }));
const khz = (hz: number) => `${nombre.format(hz / 1000)} kHz`;
const format = $derived(
  !audioFile ? null : isDsdFormat(audioFile.audio_format) ? dsdLabel(audioFile.sample_rate) : audioFile.audio_format?.toUpperCase() ?? null,
);
const frequence = $derived(
  !audioFile?.sample_rate ? null : isDsdFormat(audioFile.audio_format) ? formatDsdRate(audioFile.sample_rate) : khz(audioFile.sample_rate),
);
// Fréquence réellement envoyée, seulement quand elle diffère de la source.
const sortie = $derived.by(() => {
  const pipe = $playbackPipelineStore;
  if (!pipe || !audioFile?.sample_rate) return null;
  const cible = pipe.intermediate_pcm_rate ?? pipe.output_sample_rate;
  return cible && cible !== audioFile.sample_rate ? khz(cible) : null;
});
const debitTaille = $derived(
  [audioFile?.bitrate ? formatBitrate(audioFile.bitrate) : null,
   audioFile?.file_size ? $t("player.size_mb").replace("{n}", String(Math.round(audioFile.file_size / 1024 / 1024))) : null]
    .filter(Boolean).join(" · "),
);

const cheminDecoupe = $derived.by(() => {
  const p = $player?.pathFile ?? "";
  const i = Math.max(p.lastIndexOf("\\"), p.lastIndexOf("/"));
  return { dossier: p.slice(0, i + 1), fichier: p.slice(i + 1) };
});

const chaine = $derived(modeChaine ? CHAINE[modeChaine] : null);
// Sortie demandée non obtenue : la pastille porte un avertissement et le motif.
const repli = $derived($playbackPipelineStore ? messageRepli($playbackPipelineStore, $t, (hz) => frequenceLisible(hz, $currentLocale)) : null);

// « 16 bit · 44,1 kHz » ; le DSD n'a qu'une fréquence.
const qualite = $derived(
  [audioFile?.bits_per_sample && !isDsdFormat(audioFile.audio_format) ? `${audioFile.bits_per_sample} bit` : null, frequence]
    .filter(Boolean).join(" · "),
);

// Sortie réellement utilisée par la lecture, sinon celle choisie ; nom lisible.
const nomSortie = $derived.by(() => {
  const brut = $playbackPipelineStore?.device_name;
  const d = $audioDevicesStore.devices.find((x) => (brut ? x.name === brut || x.displayName === brut : x.displayName === $audioDevicesStore.activeDisplayName));
  // Nom WASAPI absent de la liste (« Haut-parleurs (3- Fosi Audio K7) ») : on le lit quand même.
  return d ? decrireSortie(d).nom : brut ? decrireSortie({ name: brut }).nom : null;
});

// Marque LTR : le chemin tronqué par la gauche garde ses « \ » à leur place.
const MARQUE = "\u200E";

const pastille = "h-5.5 px-2 flex items-center gap-1.5 rounded-full whitespace-nowrap text-[10.5px] font-semibold border";
// Service actif : pastille verte, voyant allumé.
const voyant = "w-1.5 h-1.5 rounded-full bg-current shadow-[0_0_6px_currentColor]";
const pastilleCliquable = "cursor-pointer transition-colors hover:border-current/50";

let copie = $state(false);
async function copierChemin() {
  const p = $player?.pathFile;
  if (!p) return;
  try {
    await navigator.clipboard.writeText(p);
    copie = true;
    setTimeout(() => (copie = false), 1500);
  } catch (e) {
    console.error("[player] clipboard failed:", e);
  }
}

/** « Révéler » plutôt qu'ouvrir : marche hors $HOME et ne lance jamais le fichier. */
async function handleOpenPath(path: string) {
  try {
    await revealItemInDir(path);
  } catch (e) {
    console.error("Impossible d'ouvrir le dossier du morceau", e);
  }
}

function handleSeek(percent: number) {
  if (duration) playerService.seekTo((percent / 100) * duration);
}


const repetitions = ["off", "one", "all"] as const;
const libelleRepetition = $derived(
  $queueState.repeatMode === "one" ? $t("player.repeat_one") : $queueState.repeatMode === "all" ? $t("player.repeat_all") : $t("player.repeat_off"),
);

const ib = "relative w-9.5 h-9.5 shrink-0 flex items-center justify-center rounded-full cursor-pointer transition-colors text-(--lc-ic) hover:text-(--lc-ic-fort) hover:bg-(--lc-survol)";
const actif = "text-(--lc-acc)! after:absolute after:bottom-0.75 after:left-1/2 after:-ml-[1.5px] after:w-0.75 after:h-0.75 after:rounded-full after:bg-current";
const gros = "w-11 h-11 shrink-0 flex items-center justify-center rounded-full cursor-pointer transition-colors text-(--lc-ic-fort) hover:bg-(--lc-survol)";
</script>

<!-- Info cliquable quand elle mène à une fiche. -->
{#snippet info(texte: string, classes: string, cible: string | null)}
  {#if cible}
    <button type="button" class="{classes} min-w-0 truncate text-left cursor-pointer hover:underline underline-offset-3" title={texte} onclick={() => goto(cible)}>{texte}</button>
  {:else}
    <span class="{classes} min-w-0 truncate" title={texte}>{texte}</span>
  {/if}
{/snippet}

<div class="shrink-0 px-2.5 pt-1.5 pb-2.5">
  <!-- Pas d'overflow-hidden : le menu Sortie et le détail de la chaîne débordent vers le haut. -->
  <div class="lecteur relative rounded-[18px]" style:--lh={$couleurMorceau.h} style:--ls={$couleurMorceau.s}>
    <!-- ─── Pochette et titres · commandes et progression · actions ─── -->
    <div
      class="grid grid-cols-[minmax(220px,1fr)_minmax(300px,1.4fr)_minmax(max-content,1fr)] items-center gap-7 max-[1250px]:gap-5 h-23 pl-2 pr-5
             max-lg:grid-cols-[minmax(0,1fr)_auto] max-lg:h-auto max-lg:py-3 max-lg:gap-x-3 max-lg:gap-y-2"
    >
      <div class="flex items-center gap-4 min-w-0">
        {#if hasTrack}
          <div class="relative shrink-0">
            <div class="lecteur-pochette w-19 h-19 rounded-xl overflow-hidden">
              <ImgZoom src={coverSrc} />
            </div>
            <!-- Pastille de lecture, détourée de la couleur du lecteur. -->
            {#if isPlaying}
              <span class="absolute -bottom-0.5 -right-0.5 w-3 h-3 rounded-full bg-emerald-500 ring-2 ring-white dark:ring-[#141816]"></span>
            {/if}
          </div>
          <div class="min-w-0 flex flex-col gap-0.75 leading-tight">
            {@render info(trackTitle, "text-[17px] font-bold tracking-[-0.01em] text-(--lc-tx)", lienTitre(lieu))}
            <span class="flex items-center gap-1 min-w-0 text-sm font-medium text-(--lc-tx2) whitespace-nowrap">
              {#if audioTags?.artist}{@render info(audioTags.artist, "text-(--lc-acc) shrink-0 max-w-[75%]", lienArtiste(lieu))}{/if}
              {#if audioTags?.album}
                <span class="shrink-0 text-(--lc-mu)">·</span>
                {@render info(audioTags.album, "text-(--lc-mu)", lienAlbum(lieu))}
              {/if}
            </span>
            {#if annee}
              {@const cible = lienAnnee(lieu, annee)}
              {#if cible}
                <button type="button" class="self-start text-xs font-medium tabular-nums text-(--lc-mu) cursor-pointer hover:text-(--lc-tx2) hover:underline underline-offset-3"
                        title={$t("player.year_albums").replace("{y}", annee)} onclick={() => goto(cible)}>{annee}</button>
              {:else}
                <span class="text-xs font-medium tabular-nums text-(--lc-mu)">{annee}</span>
              {/if}
            {/if}
          </div>
        {:else}
          <div class="lecteur-pochette w-19 h-19 shrink-0 rounded-xl overflow-hidden">
            <img src="/images/no-cd.png" alt="" class="w-full h-full object-cover" />
          </div>
          <span class="text-sm text-(--lc-mu)">{$t("player.no_track")}</span>
        {/if}
      </div>

      <div class="justify-self-center w-full max-w-155 flex flex-col items-center gap-1.5 min-w-0 max-lg:max-w-none max-lg:col-span-2 max-lg:row-start-2">
        <div class="flex items-center gap-1.5">
          <button type="button" class="{ib} {$queueState.isShuffled ? actif : ''}" title={$t("player.shuffle")} aria-label={$t("player.shuffle")} aria-pressed={$queueState.isShuffled}
                  onclick={() => queueState.setIsShuffled(!$queueState.isShuffled)}>
            <Icon icon="material-symbols-light:shuffle-rounded" width="22" />
          </button>
          <button type="button" class="{ib} max-[1250px]:hidden" title={$t("player.back10")} aria-label={$t("player.back10")}
                  onclick={() => playerService.seekTo(Math.max(0, jsPosition - 10))}>
            <Icon icon="material-symbols-light:replay-10-rounded" width="22" />
          </button>
          <button type="button" class={gros} title={$t("player.prev")} aria-label={$t("player.prev")} onclick={() => playerService.prevTrack()}>
            <Icon icon="material-symbols-light:skip-previous-rounded" width="32" />
          </button>
          <button
            type="button"
            class="lecteur-lecture relative w-12.5 h-12.5 mx-2 shrink-0 flex items-center justify-center rounded-full cursor-pointer
                   bg-(--lc-play) text-(--lc-play-tx) transition-transform hover:scale-105 active:scale-95"
            title={enAttente ? $t("player.preparing") : isPlaying ? $t("player.pause") : $t("player.play")}
            aria-label={isPlaying ? $t("player.pause") : $t("player.play")}
            aria-busy={enAttente}
            onclick={() => playerService.handleTogglePlay()}
          >
            <Icon icon={isPlaying ? "material-symbols-light:pause-rounded" : "material-symbols-light:play-arrow-rounded"} width="32" />
            <AnneauAttente actif={enAttente} />
          </button>
          <button type="button" class={gros} title={$t("player.next")} aria-label={$t("player.next")} onclick={() => playerService.nextTrack()}>
            <Icon icon="material-symbols-light:skip-next-rounded" width="32" />
          </button>
          <button type="button" class="{ib} max-[1250px]:hidden" title={$t("player.fwd10")} aria-label={$t("player.fwd10")}
                  onclick={() => duration && playerService.seekTo(Math.min(duration, jsPosition + 10))}>
            <Icon icon="material-symbols-light:forward-10-rounded" width="22" />
          </button>
          <button type="button" class="{ib} {$queueState.repeatMode !== 'off' ? actif : ''}" title={libelleRepetition} aria-label={libelleRepetition}
                  onclick={() => queueState.setRepeatMode(repetitions[(repetitions.indexOf($queueState.repeatMode) + 1) % 3])}>
            <Icon icon={$queueState.repeatMode === "one" ? "material-symbols-light:repeat-one-rounded" : "material-symbols-light:repeat-rounded"} width="22" />
          </button>
          <button type="button" class="{ib} max-[1080px]:hidden disabled:opacity-40 disabled:cursor-default" disabled={!hasTrack}
                  title={$t("player.stop")} aria-label={$t("player.stop")} onclick={() => playerService.stopPlay()}>
            <Icon icon="material-symbols-light:stop-outline-rounded" width="22" />
          </button>
        </div>
        <div class="flex items-center gap-3 w-full font-mono text-[11px] tabular-nums text-(--lc-mu)">
          <span class="min-w-9 text-right">{minutesSecondes(jsPosition)}</span>
          <div class="flex-1 min-w-0"><PlayerProgressBar position={jsPositionPercent} onseek={handleSeek} /></div>
          <span class="min-w-9">{minutesSecondes(audioFile?.duration ?? 0)}</span>
        </div>
      </div>

      <div class="flex items-center justify-end gap-0.5 max-lg:col-start-2 max-lg:row-start-1">
        {#if hasTrack && $player?.pathFile}
          {@const aime = $liked.paths.has($player.pathFile)}
          <button type="button" class="favori {ib} {aime ? 'text-[#e2566a]! dark:text-[#ff7a88]!' : ''}" title={$t("player.like")} aria-label={$t("player.like")} aria-pressed={aime}
                  onclick={() => $player?.pathFile && liked.toggle($player.pathFile)}>
            <Icon icon={aime ? "material-symbols-light:favorite-rounded" : "material-symbols-light:favorite-outline-rounded"} width="22" />
          </button>
        {/if}
        <button type="button" class={ib} title={$t("player.queue")} aria-label={$t("player.queue")} onclick={toggleQueuePanel}>
          <Icon icon="material-symbols-light:queue-music-rounded" width="22" />
        </button>
        <button type="button" class="{ib} {$lyricsPanelOpened ? actif : ''}" title={$t("player.lyrics")} aria-label={$t("player.lyrics")} aria-pressed={$lyricsPanelOpened}
                onclick={toggleLyricsPanel}>
          <Icon icon="material-symbols-light:lyrics-outline-rounded" width="22" />
        </button>
        <span class="w-px h-5.5 mx-2 shrink-0 bg-(--lc-trait) max-md:hidden"></span>
        <PlayerSoundBar />
      </div>
    </div>

    <!-- ─── Ligne d'état : qualité du son · fichier · chaîne et services ─── -->
    <div class="flex items-center gap-2.5 h-8 pl-3 pr-2 rounded-b-[18px] text-[11px] text-(--lc-mu) border-t border-(--lc-statut-bd) bg-(--lc-statut)">
      {#if audioFile}
        <div
          role="group"
          aria-label={$t("player.audio_chain")}
          class="shrink-0 flex items-center gap-2 whitespace-nowrap cursor-help"
          onmouseenter={() => (showPipelinePopover = true)}
          onmouseleave={() => (showPipelinePopover = false)}
        >
          {#if format}
            <span class="h-5 px-1.5 flex items-center rounded-[5px] text-[10px] font-bold tracking-[0.06em] bg-(--lc-fmt-bg) text-(--lc-fmt-tx)">{format}</span>
          {/if}
          {#if qualite}<span class="font-mono font-medium text-(--lc-tx)">{qualite}</span>{/if}
          {#if sortie}<span class="font-mono text-amber-600 dark:text-amber-400">→ {sortie}</span>{/if}
          {#if debitTaille}<span class="font-mono max-[1080px]:hidden">{debitTaille}</span>{/if}
        </div>

        <span class="w-px h-3.5 shrink-0 bg-(--lc-trait) max-md:hidden"></span>

        <div class="group/chemin flex-1 min-w-0 flex items-center gap-1 font-mono text-[10.5px] max-md:hidden">
          <button type="button" class="min-w-0 flex items-center gap-1.5 cursor-pointer" title={$t("player.open_folder")} onclick={() => $player?.pathFile && handleOpenPath($player.pathFile)}>
            <Icon icon="material-symbols-light:folder-open-outline-rounded" width="15" class="shrink-0 group-hover/chemin:text-(--lc-tx2)" />
            <span class="lecteur-dossier min-w-0 shrink-[999] truncate group-hover/chemin:text-(--lc-tx2)">{MARQUE + cheminDecoupe.dossier + MARQUE}</span>
            <!-- Le dossier s'efface d'abord, le nom se tronque ensuite. -->
            <span class="min-w-8 truncate text-(--lc-tx2)" title={cheminDecoupe.fichier}>{cheminDecoupe.fichier}</span>
          </button>
          <button type="button" class="shrink-0 w-5.5 h-5.5 flex items-center justify-center rounded-md cursor-pointer opacity-0 group-hover/chemin:opacity-100 focus-visible:opacity-100 hover:bg-(--lc-survol) hover:text-(--lc-tx)"
                  title={copie ? $t("player.path_copied") : $t("player.copy_path")} aria-label={$t("player.copy_path")} onclick={copierChemin}>
            <Icon icon={copie ? "material-symbols-light:check-rounded" : "material-symbols-light:content-copy-outline-rounded"} width="14" />
          </button>
          {#if !isPlaying && $player?.trackId}
            <button type="button" class="shrink-0 w-5.5 h-5.5 flex items-center justify-center rounded-md cursor-pointer hover:text-red-500 hover:bg-(--lc-survol)"
                    title={$t("player.close_track")} aria-label={$t("player.close_track")} onclick={() => player.clearTrack($player.trackId as string)}>
              <Icon icon="material-symbols-light:close-rounded" width="15" />
            </button>
          {/if}
        </div>
      {:else}
        <span class="flex-1 pl-1 text-(--lc-mu2)">{$t("player.inactive")}</span>
      {/if}

      <div class="shrink-0 ml-auto flex items-center gap-1.5">
        {#if chaine}
          <button type="button" class="{pastille} {chaine.teinte} {pastilleCliquable}" title={repli ?? $t("pipeline.learn")} onclick={() => goto("/settings/guide")}>
            {#if repli}
              <Icon icon="material-symbols:warning-rounded" width="13" class="shrink-0 text-amber-600 dark:text-amber-400" />
            {:else}
              <span class="w-1.5 h-1.5 rounded-full bg-current"></span>
            {/if}{$t(chaine.cle)}
          </button>
        {/if}
        {#if nomSortie}
          <button type="button" class="{pastille} {TEINTE.bleu} {pastilleCliquable} max-w-44 max-[1250px]:hidden" title={$t("player.output")} onclick={() => goto("/settings/audio")}>
            <Icon icon="material-symbols-light:speaker-outline-rounded" width="14" class="shrink-0" /><span class="truncate">{nomSortie}</span>
          </button>
        {/if}
        {#if surWindows && wasapiConfigured && !$playbackPipelineStore?.repli}
          <button type="button" class="{pastille} {TEINTE.vert} {pastilleCliquable}" title={$t("player.wasapi_on_short")} onclick={() => goto("/settings/audio")}>
            <span class={voyant}></span>WASAPI
          </button>
        {/if}
        {#if $dlnaStatusStore?.running}
          <button type="button" class="{pastille} {TEINTE.vert} {pastilleCliquable}" title={$dlnaStatusStore.url ?? "DLNA"} onclick={() => goto("/settings/network")}>
            <span class={voyant}></span>DLNA
          </button>
        {/if}
        <StatusBar />
      </div>
    </div>

    <!-- Détail de la chaîne audio, posé sur le bord haut de la carte. -->
    {#if showPipelinePopover && $playbackPipelineStore}
      <div class="absolute bottom-full left-2 mb-2 z-50 pointer-events-none">
        <PipelineInfoPopover info={$playbackPipelineStore} />
      </div>
    {/if}
  </div>
</div>

<LyricsPanel />
