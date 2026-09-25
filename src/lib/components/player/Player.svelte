<script lang="ts">
import { revealItemInDir } from '@tauri-apps/plugin-opener';
import Icon from "@iconify/svelte";
import { t } from "$lib/i18n";
import PlayerProgressBar from "./PlayerProgressBar.svelte";
import PlayerSoundBar from "./PlayerSoundBar.svelte";
import { toggleQueuePanel } from "$lib/stores/queue/queueUi.store";
import { toggleLyricsPanel, lyricsPanelOpened } from "$lib/stores/lyrics/lyricsPanel.store";
import LyricsPanel from "$lib/components/lyrics/LyricsPanel.svelte";
import { player } from "$lib/stores/player/player.store";
import { queueState } from "$lib/stores/queue/queueState.store";
import { truncateMiddle, displayTitle }  from "$lib/helper/tools/stringTools";
import { dateToYear, durationToMinutes } from "$lib/helper/tools/dateTools";
import ImgZoom from '$lib/components/ui/tools/ImgZoom.svelte';
import { playerService } from "$lib/services/player/player.service";
import { liked } from "$lib/stores/playlist/like.store";
import StatusBar from "$lib/components/ui/statusbar/StatusBar.svelte";
import { resolveCoverSrc } from "$lib/helper/tools/coverHelper";
import { dsdLabel, formatBitrate, formatDsdRate, isDsdFormat } from "$lib/helper/tools/audioFormatTools";
import { dlnaStatusStore } from "$lib/stores/dlna/dlna.store";
import { get } from "svelte/store";
import { localiserUn, lienTitre, lienArtiste, lienAlbum, type TrackLocation } from "$lib/helper/library/trackLocation";
import { playbackPipelineStore, pipelineMode } from "$lib/stores/player/playbackPipeline.store";
import PipelineInfoPopover from "$lib/components/player/PipelineInfoPopover.svelte";
import { goto } from "$app/navigation";
import { settingsStore } from "$lib/stores/settings/settings.store";
import { detectOS } from "$lib/helper/tools/osDetection";

let audioFile = $derived($player?.audioFile);
let audioTags = $derived(audioFile?.tags);

// Backend audio configuré (indépendant de la lecture — c'est un réglage
// global). Sur Windows : WASAPI exclusive OU CPAL partagé.
let isWindows = detectOS() === "windows";
let wasapiConfigured = $derived($settingsStore.wasapi_exclusive === "true");

// État de la chaîne audio (bit-perfect / resamplé / DSD→PCM) + hover du popover.
let pipelineModeState = $derived(pipelineMode($playbackPipelineStore));
let showPipelinePopover = $state(false);

// Titre affiché : tag title si présent, sinon nom de fichier (utile pour
// les DSF/DFF sans DITI/ID3 où le tag title manque).
let trackTitle = $derived(displayTitle(audioTags?.title, $player?.pathFile, $t('common.unknown_title')));

// Le lecteur ne connaît que le fichier : il faut retrouver sa fiche pour que
// le titre, l'artiste et l'album mènent quelque part. Rien si le morceau est
// joué hors bibliothèque.
let lieu = $state<TrackLocation | null>(null);

// Le chemin, et lui seul. Lire `$player` dans l'effet le relançait à chaque
// image de la boucle de progression : le lien était détruit et reconstruit
// sous le curseur — il clignotait et le clic ne prenait jamais.
let cheminLu = $derived($player?.pathFile ?? null);

// Garde explicite plutôt que de s'en remettre à l'égalité du framework : tant
// que le morceau ne change pas, l'effet ne touche à rien.
let cheminResolu: string | null = null;

$effect(() => {
  const chemin = cheminLu;
  if (chemin === cheminResolu) return;
  cheminResolu = chemin;

  if (!chemin) {
    lieu = null;
    return;
  }
  lieu = null;
  localiserUn(chemin).then((trouve) => {
    // Un autre morceau a pu prendre la main entre-temps.
    if (get(player)?.pathFile === chemin) lieu = trouve;
  });
});

let isPlaying = $derived($player?.status === "playing");
let duration = $derived($player?.duration ?? 0);
let jsPosition = $derived($player?.jsPosition ?? 0);
let jsPositionPercent = $derived(duration ? Math.min(100, Math.max(0, (jsPosition / duration) * 100)) : 0);
let hasTrack = $derived(!!$player?.pathFile);
let coverSrc = $derived(audioTags?.attached_images?.[0]?.image_src ?? '/images/no-cd.png');

// Cover pour le glow ambient
// On n'utilise PAS les data URI (trop lourds, bloqués par le webview)
// On utilise uniquement les URLs asset:// (thumbnail sauvegardée sur disque)
let currentQueueTrack = $derived($queueState.tracks[$queueState.currentIndex] ?? null);
let glowCoverSrc = $state<string | null>(null);

$effect(() => {
  const cover = currentQueueTrack?.cover;
  if (!cover || cover === '/images/no-cd.png' || cover.startsWith('data:')) {
    glowCoverSrc = null;
    return;
  }
  resolveCoverSrc(cover, "1x").then(url => { glowCoverSrc = url; });
});


/**
 * Ouvre l'explorateur sur le morceau, sélectionné dans son dossier.
 *
 * `revealItemInDir` et non `openPath` pour deux raisons. D'abord la portée :
 * la capability `opener:allow-open-path` est limitée à `$HOME` et `$APPDATA`,
 * donc une bibliothèque sur un autre volume — un NAS, un disque dédié — était
 * silencieusement refusée. Ensuite la sûreté : `openPath` lance le fichier
 * avec son application par défaut, là où « révéler » ne peut rien exécuter.
 * C'est aussi plus utile : le morceau est mis en évidence, pas seulement son
 * dossier ouvert.
 */
const handleOpenPath = async (path: string) => {
    try {
        await revealItemInDir(path);
    } catch (e) {
        console.error("Impossible d'ouvrir le dossier du morceau", e);
    }
};

const btnBase = "flex items-center justify-center rounded-full cursor-pointer transition-all duration-150";
const btnSmall = `${btnBase} w-8 h-8`;
const btnMed = `${btnBase} w-9 h-9`;

function handleSeek(percent: number) {
  if (!duration) return;
  const newPosition = (percent / 100) * duration;
  playerService.seekTo(newPosition);
}

function handleRewind() {
  const newPos = Math.max(0, jsPosition - 10);
  playerService.seekTo(newPos);
}

function handleForward() {
  if (!duration) return;
  const newPos = Math.min(duration, jsPosition + 10);
  playerService.seekTo(newPos);
}
</script>

<!-- Snippet partagé : boutons d'action (Like / Queue / Lyrics / Volume).
     Utilisé soit dans la colonne droite (md+), soit sur sa propre row pleine
     largeur à 500-md où le 3-colonnes serait trop serré. -->
{#snippet actionButtons(_inColumn: boolean)}
  {#if $player?.pathFile}
    <button
      class="favori {btnSmall}
             {$liked.paths.has($player.pathFile)
               ? 'text-emerald-500 hover:text-emerald-400'
               : 'text-neutral-500 dark:text-neutral-500 hover:text-neutral-700 dark:hover:text-neutral-300'}"
      onclick={() => $player?.pathFile && liked.toggle($player.pathFile)}
      aria-label="Aimer"
    >
      <Icon
        icon={$liked.paths.has($player.pathFile ?? '') ? 'mynaui:heart-solid' : 'mynaui:heart'}
        width="17" height="17"
      />
    </button>
  {/if}

  <button
    class="{btnSmall} text-neutral-500 dark:text-neutral-500
           hover:text-neutral-700 dark:hover:text-neutral-300"
    onclick={toggleQueuePanel}
    aria-label="File d'attente"
  >
    <Icon icon="ph:queue" width="17" height="17" />
  </button>

  <button
    class="{btnSmall}
           {$lyricsPanelOpened
             ? 'text-emerald-500'
             : 'text-neutral-500 dark:text-neutral-500 hover:text-neutral-700 dark:hover:text-neutral-300'}"
    onclick={toggleLyricsPanel}
    aria-label="Paroles"
  >
    <Icon icon="lucide:mic-vocal" width="16" height="16" />
  </button>

  <div class="flex items-center gap-1">
    <div class="w-px h-4 bg-neutral-200/80 dark:bg-neutral-700/30 mx-0.5"></div>
    <PlayerSoundBar />
  </div>
{/snippet}

<!-- Une info du lecteur, cliquable quand elle mène à une fiche. Les classes
     restent les mêmes : seul le soulignement au survol change. -->
{#snippet info(texte: string, classes: string, cible: string | null)}
  {#if cible}
    <button type="button" class="{classes} text-left cursor-pointer hover:underline"
            title={texte} onclick={() => goto(cible)}>
      {texte}
    </button>
  {:else}
    <div class={classes} title={texte}>{texte}</div>
  {/if}
{/snippet}

<!-- Snippet partagé : cover + titre/artist/album. -->
{#snippet coverAndTitle()}
  {#if hasTrack}
    <div class="relative">
      <div class="w-14 h-14 min-w-14 lg:w-18 lg:h-18 lg:min-w-18 rounded-lg overflow-hidden
                  shadow-lg shadow-black/15 dark:shadow-black/40
                  ring-1 ring-black/6 dark:ring-white/8">
        <ImgZoom src={coverSrc} />
      </div>
      {#if isPlaying}
        <div class="absolute -bottom-0.5 -right-0.5 w-2.5 h-2.5 rounded-full
                    bg-emerald-500 ring-2 ring-white dark:ring-zinc-950"></div>
      {/if}
    </div>

    <div class="min-w-0 flex flex-col gap-0.5">
      {@render info(trackTitle,
        "truncate text-[13px] font-semibold text-neutral-900 dark:text-neutral-100",
        lienTitre(lieu))}
      {@render info(audioTags?.artist ?? "",
        "truncate text-xs text-emerald-600 dark:text-emerald-400",
        lienArtiste(lieu))}
      <div class="hidden md:flex items-center gap-1 min-w-0 text-[11px] text-neutral-400 dark:text-neutral-500">
        {@render info(audioTags?.album ?? "", "truncate", lienAlbum(lieu))}
        {#if audioTags?.year}
          <span class="shrink-0">• {dateToYear(audioTags.year)}</span>
        {/if}
      </div>
    </div>
  {:else}
    <div class="w-14 h-14 min-w-14 rounded-lg
                bg-neutral-100 dark:bg-neutral-800/60
                border border-neutral-200/60 dark:border-neutral-700/40
                flex items-center justify-center">
      <svg class="w-5 h-5 text-neutral-300 dark:text-neutral-600" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
        <path d="M9 18V5l12-2v13" stroke-linecap="round" stroke-linejoin="round"/>
        <circle cx="6" cy="18" r="3"/><circle cx="18" cy="16" r="3"/>
      </svg>
    </div>
    <span class="text-sm text-neutral-400 dark:text-neutral-500">{$t('player.no_track')}</span>
  {/if}
{/snippet}

<!-- Snippet partagé : transport buttons + progress bar.
     `compact = true` à largeur intermédiaire (500-md) : on n'affiche que
     prev/play/next, les boutons accessoires (shuffle/repeat/stop/etc.) restent
     hidden md:flex donc disparaissent naturellement. -->
{#snippet transportControls()}
  <div class="flex items-center gap-0.5">
    <button
      class="{btnSmall}
             {$queueState.isShuffled
               ? 'text-emerald-500'
               : 'text-neutral-500 dark:text-neutral-600 hover:text-neutral-700 dark:hover:text-neutral-300'}"
      onclick={() => queueState.setIsShuffled(!$queueState.isShuffled)}
      aria-label="Shuffle"
    >
      <Icon icon="ph:shuffle" width="15" height="15" />
    </button>

    <button class="{btnSmall} text-neutral-500 dark:text-neutral-500 hover:text-neutral-700 dark:hover:text-neutral-300"
            onclick={handleRewind}
            aria-label="Reculer 10s">
      <Icon icon="ph:clock-counter-clockwise" width="15" height="15" />
    </button>

    <button
      class="{btnMed} text-neutral-700 dark:text-neutral-200
             hover:text-neutral-900 dark:hover:text-white"
      onclick={async() => await playerService.prevTrack()}
      aria-label="Précédent"
    >
      <Icon icon="ph:skip-back-fill" width="17" height="17" />
    </button>

    <button
      class="{btnBase} w-10 h-10 lg:w-11 lg:h-11 mx-1
             text-white
             bg-emerald-500 hover:bg-emerald-400
             shadow-md shadow-emerald-500/20 hover:shadow-lg hover:shadow-emerald-500/30
             hover:scale-[1.06] active:scale-95"
      onclick={() => playerService.handleTogglePlay()}
      aria-label={isPlaying ? 'Pause' : 'Lecture'}
    >
      {#if isPlaying}
        <svg class="w-4.5 h-4.5" fill="currentColor" viewBox="0 0 24 24">
          <rect x="6" y="4" width="4" height="16" rx="1"/>
          <rect x="14" y="4" width="4" height="16" rx="1"/>
        </svg>
      {:else}
        <svg class="w-4.5 h-4.5 ml-0.5" fill="currentColor" viewBox="0 0 24 24">
          <path d="M8 5v14l11-7z"/>
        </svg>
      {/if}
    </button>

    <button
      class="{btnMed} text-neutral-700 dark:text-neutral-200
             hover:text-neutral-900 dark:hover:text-white"
      onclick={async() => await playerService.nextTrack()}
      aria-label="Suivant"
    >
      <Icon icon="ph:skip-forward-fill" width="17" height="17" />
    </button>

    <button class="{btnSmall} text-neutral-500 dark:text-neutral-500 hover:text-neutral-700 dark:hover:text-neutral-300"
            onclick={handleForward}
            aria-label="Avancer 10s">
      <Icon icon="ph:clock-clockwise" width="15" height="15" />
    </button>

    <button
      class="{btnSmall}
             {hasTrack
               ? 'text-neutral-500 dark:text-neutral-500 hover:text-neutral-700 dark:hover:text-neutral-300'
               : 'text-neutral-300 dark:text-neutral-800'}"
      onclick={async() => await playerService.stopPlay()}
      aria-label="Stop"
    >
      <Icon icon="ph:stop-fill" width="15" height="15" />
    </button>

    <button
      class="{btnSmall}
             {$queueState.repeatMode !== 'off'
               ? 'text-emerald-500'
               : 'text-neutral-500 dark:text-neutral-600 hover:text-neutral-700 dark:hover:text-neutral-300'}"
      onclick={() => {
        const modes = ['off', 'one', 'all'] as const;
        const idx = modes.indexOf($queueState.repeatMode);
        queueState.setRepeatMode(modes[(idx + 1) % 3]);
      }}
      aria-label="Repeat"
    >
      <Icon icon={$queueState.repeatMode === 'one' ? 'ph:repeat-once' : 'ph:repeat'} width="15" height="15" />
    </button>
  </div>

  <div class="flex items-center gap-2 md:gap-3 w-full">
    <span class="text-[10px] tabular-nums text-neutral-400 dark:text-neutral-500 w-9 text-right shrink-0">
      {jsPosition ? durationToMinutes(jsPosition) : "0:00"}
    </span>
    <div class="flex-1 relative">
      <PlayerProgressBar position={jsPositionPercent} onseek={handleSeek} />
    </div>
    <span class="text-[10px] tabular-nums text-neutral-400 dark:text-neutral-500 w-9 shrink-0">
      {audioFile?.duration ? durationToMinutes(audioFile?.duration) : "0:00"}
    </span>
  </div>
{/snippet}

<div class="flex w-full flex-col">
  <div class="relative bg-white/95 dark:bg-zinc-950/90 backdrop-blur-sm
              border-t border-neutral-300/80 dark:border-white/6
              shadow-[0_-1px_4px_rgba(0,0,0,0.04)] dark:shadow-none">

    <!-- Note : le loader "Préparation du morceau…" (profil Minimal,
         pre-decode complet) est désormais affiché dans la StatusBar globale
         via la task `playback-preparing` enregistrée dans taskProgress.store. -->

    <!-- Ambient glow : jaquette floutée en fond (style Apple Music) -->
    {#if glowCoverSrc}
      <div class="absolute inset-0 z-0 pointer-events-none overflow-hidden transition-opacity duration-700
                  {isPlaying ? 'opacity-40 dark:opacity-25' : 'opacity-15 dark:opacity-10'}">
        <img src={glowCoverSrc} alt=""
             class="absolute -inset-10 w-[calc(100%+80px)] h-[calc(100%+80px)] object-cover blur-[50px] saturate-[1.8] scale-110" />
      </div>
    {/if}

    <!-- ═══════════════════════════════════════════════════ -->
    <!-- MOBILE PLAYER (<500px) : layout vertical en lignes -->
    <!-- Bascule plus tard que sm (640) pour qu'à 200%      -->
    <!-- OS scaling on garde le format desktop.             -->
    <!-- ═══════════════════════════════════════════════════ -->
    <div class="min-[500px]:hidden relative z-10">
      <!-- Progress bar en haut -->
      <div class="px-3 pt-1">
        <PlayerProgressBar position={jsPositionPercent} onseek={handleSeek} />
        <div class="flex justify-between text-[9px] tabular-nums text-neutral-400 dark:text-neutral-500 mt-0.5 px-0.5">
          <span>{jsPosition ? durationToMinutes(jsPosition) : "0:00"}</span>
          <span>{audioFile?.duration ? durationToMinutes(audioFile?.duration) : "0:00"}</span>
        </div>
      </div>

      <!-- Ligne 1 : cover + infos + actions secondaires (like, queue, lyrics) -->
      <div class="flex items-center gap-2.5 px-3 pt-1">
        {#if hasTrack}
          <div class="w-10 h-10 min-w-10 rounded-md overflow-hidden
                      shadow-sm ring-1 ring-black/5 dark:ring-white/10">
            <img src={coverSrc} alt="" class="w-full h-full object-cover" />
          </div>

          <div class="flex-1 min-w-0 flex flex-col">
            {@render info(trackTitle,
              "truncate text-xs font-semibold text-neutral-900 dark:text-neutral-100",
              lienTitre(lieu))}
            {@render info(audioTags?.artist ?? "",
              "truncate text-[10px] text-emerald-600 dark:text-emerald-400",
              lienArtiste(lieu))}
          </div>

          <!-- Actions secondaires en compact (visible uniquement si une piste joue) -->
          <div class="flex items-center gap-0.5 shrink-0">
            <button
              class="favori {btnSmall}
                     {$liked.paths.has($player?.pathFile ?? '')
                       ? 'text-emerald-500'
                       : 'text-neutral-500 dark:text-neutral-500 hover:text-neutral-700 dark:hover:text-neutral-300'}"
              onclick={() => $player?.pathFile && liked.toggle($player.pathFile)}
              aria-label="Aimer"
            >
              <Icon
                icon={$liked.paths.has($player?.pathFile ?? '') ? 'mynaui:heart-solid' : 'mynaui:heart'}
                width="15" height="15"
              />
            </button>

            <button
              class="{btnSmall} text-neutral-500 dark:text-neutral-500
                     hover:text-neutral-700 dark:hover:text-neutral-300"
              onclick={toggleQueuePanel}
              aria-label="File d'attente"
            >
              <Icon icon="ph:queue" width="15" height="15" />
            </button>

            <button
              class="{btnSmall}
                     {$lyricsPanelOpened
                       ? 'text-emerald-500'
                       : 'text-neutral-500 dark:text-neutral-500 hover:text-neutral-700 dark:hover:text-neutral-300'}"
              onclick={toggleLyricsPanel}
              aria-label="Paroles"
            >
              <Icon icon="lucide:mic-vocal" width="14" height="14" />
            </button>
          </div>
        {:else}
          <div class="flex-1 text-xs text-neutral-400">{$t('player.no_track')}</div>
        {/if}
      </div>

      <!-- Ligne 1.5 : barre infos techniques compactes (visible uniquement si une piste joue) -->
      {#if hasTrack && audioFile}
        <div class="flex items-center gap-1.5 px-3 pt-1.5 pb-0.5 text-[9px] leading-none
                    text-neutral-500 dark:text-neutral-500
                    overflow-x-auto whitespace-nowrap">
          <!-- Format badge -->
          {#if isDsdFormat(audioFile?.audio_format)}
            <span class="px-1 py-0.5 rounded-sm font-bold tracking-wider
                         bg-amber-500/12 text-amber-600 dark:text-amber-200
                         border border-amber-400/45">
              {dsdLabel(audioFile?.sample_rate)}
            </span>
          {:else if audioFile?.bits_per_sample && audioFile.bits_per_sample >= 24}
            <span class="px-1 py-0.5 rounded-sm font-semibold
                         bg-emerald-500/10 text-emerald-600 dark:text-emerald-400">
              Hi-Res
            </span>
          {:else if audioFile?.audio_format}
            <span class="px-1 py-0.5 rounded-sm font-semibold uppercase
                         bg-neutral-200/60 dark:bg-white/6 text-neutral-500 dark:text-neutral-400">
              {audioFile.audio_format}
            </span>
          {/if}

          <!-- Rates source → sortie -->
          {#if audioFile?.sample_rate}
            <span class="tabular-nums">
              {#if isDsdFormat(audioFile?.audio_format)}
                {formatDsdRate(audioFile.sample_rate)}
              {:else}
                {#if audioFile.bits_per_sample}{audioFile.bits_per_sample}/{/if}{audioFile.sample_rate / 1000}k
              {/if}
            </span>
          {/if}

          {#if $playbackPipelineStore}
            {@const pipe = $playbackPipelineStore}
            {@const targetRate = pipe.intermediate_pcm_rate ?? pipe.output_sample_rate}
            <Icon icon="lucide:arrow-right" width={8} class="text-neutral-400 dark:text-neutral-600" />
            <span
              class="tabular-nums {pipe.resampler_active || pipe.intermediate_pcm_rate
                ? 'text-amber-600 dark:text-amber-400/80'
                : pipe.bit_perfect
                  ? 'text-emerald-600 dark:text-emerald-400/80'
                  : 'text-neutral-500 dark:text-neutral-400'}">
              {targetRate / 1000}k
            </span>
          {/if}

          <!-- Badge mode pipeline -->
          {#if pipelineModeState === "dop"}
            <span class="px-1 py-0.5 rounded-sm font-semibold text-[8px] tracking-wider uppercase
                         bg-purple-500/10 text-purple-600 dark:text-purple-400
                         border border-purple-500/25">
              {$t("pipeline.badge_dop")}
            </span>
          {:else if pipelineModeState === "bit-perfect"}
            <span class="px-1 py-0.5 rounded-sm font-semibold text-[8px] tracking-wider uppercase
                         bg-emerald-500/10 text-emerald-600 dark:text-emerald-400
                         border border-emerald-500/25">
              {$t("pipeline.badge_bit_perfect")}
            </span>
          {:else if pipelineModeState === "resampled"}
            <span class="px-1 py-0.5 rounded-sm font-semibold text-[8px] tracking-wider uppercase
                         bg-amber-500/10 text-amber-600 dark:text-amber-400
                         border border-amber-500/25">
              {$t("pipeline.badge_resampled")}
            </span>
          {:else if pipelineModeState === "dsd"}
            <span class="px-1 py-0.5 rounded-sm font-semibold text-[8px] tracking-wider uppercase
                         bg-sky-500/10 text-sky-600 dark:text-sky-400
                         border border-sky-500/25">
              {$t("pipeline.badge_dsd_pcm")}
            </span>
          {:else if pipelineModeState === "shared"}
            <span class="px-1 py-0.5 rounded-sm font-semibold text-[8px] tracking-wider uppercase
                         bg-neutral-500/10 text-neutral-600 dark:text-neutral-400
                         border border-neutral-500/25">
              {$t("pipeline.badge_shared")}
            </span>
          {/if}
        </div>
      {/if}

      <!-- Volume + device (rangée dédiée pour éviter de comprimer les actions du dessus) -->
      {#if hasTrack}
        <div class="flex items-center justify-center gap-2 pt-1 pb-0.5">
          <PlayerSoundBar />
          <!-- Stop -->
          <button
            class="{btnSmall} text-neutral-500 dark:text-neutral-500
                   hover:text-neutral-700 dark:hover:text-neutral-300"
            onclick={async() => await playerService.stopPlay()}
            aria-label="Stop"
          >
            <Icon icon="ph:stop-fill" width="14" height="14" />
          </button>
        </div>
      {/if}

      <!-- Ligne 2 : transport (shuffle, prev, play, next, repeat) centré -->
      <div class="flex items-center justify-center gap-1 pb-2 pt-1">
        <button
          class="{btnSmall}
                 {$queueState.isShuffled
                   ? 'text-emerald-500'
                   : 'text-neutral-500 dark:text-neutral-600 hover:text-neutral-700 dark:hover:text-neutral-300'}"
          onclick={() => queueState.setIsShuffled(!$queueState.isShuffled)}
          aria-label="Shuffle"
        >
          <Icon icon="ph:shuffle" width="14" height="14" />
        </button>

        <button
          class="{btnMed} text-neutral-700 dark:text-neutral-200 hover:text-neutral-900 dark:hover:text-white"
          onclick={async() => await playerService.prevTrack()}
          aria-label="Précédent"
        >
          <Icon icon="ph:skip-back-fill" width="16" />
        </button>

        <button
          class="{btnBase} w-11 h-11 mx-1 text-white bg-emerald-500 hover:bg-emerald-400
                 shadow-md shadow-emerald-500/20 active:scale-95"
          onclick={() => playerService.handleTogglePlay()}
          aria-label={isPlaying ? 'Pause' : 'Lecture'}
        >
          {#if isPlaying}
            <svg class="w-4 h-4" fill="currentColor" viewBox="0 0 24 24">
              <rect x="6" y="4" width="4" height="16" rx="1"/><rect x="14" y="4" width="4" height="16" rx="1"/>
            </svg>
          {:else}
            <svg class="w-4 h-4 ml-0.5" fill="currentColor" viewBox="0 0 24 24"><path d="M8 5v14l11-7z"/></svg>
          {/if}
        </button>

        <button
          class="{btnMed} text-neutral-700 dark:text-neutral-200 hover:text-neutral-900 dark:hover:text-white"
          onclick={async() => await playerService.nextTrack()}
          aria-label="Suivant"
        >
          <Icon icon="ph:skip-forward-fill" width="16" />
        </button>

        <button
          class="{btnSmall}
                 {$queueState.repeatMode !== 'off'
                   ? 'text-emerald-500'
                   : 'text-neutral-500 dark:text-neutral-600 hover:text-neutral-700 dark:hover:text-neutral-300'}"
          onclick={() => {
            const modes = ['off', 'one', 'all'] as const;
            const idx = modes.indexOf($queueState.repeatMode);
            queueState.setRepeatMode(modes[(idx + 1) % 3]);
          }}
          aria-label="Repeat"
        >
          <Icon icon={$queueState.repeatMode === 'one' ? 'ph:repeat-once' : 'ph:repeat'} width="14" height="14" />
        </button>
      </div>
    </div>

    <!-- ═══════════════════════════════════════════════════ -->
    <!-- DESKTOP PLAYER (≥500px) : layout complet 3 colonnes -->
    <!-- ═══════════════════════════════════════════════════ -->
    <!-- ═══ LAYOUT WIDE (md+ ≥ 768px) : 3 colonnes ═══
         cover/title | transport+progress | actions -->
    <div class="hidden md:flex relative z-10 items-center justify-between gap-4 lg:gap-6 px-5 py-3">
      <div class="flex items-center gap-4 w-48 lg:w-72 shrink-0">
        {@render coverAndTitle()}
      </div>
      <div class="flex flex-col items-center gap-2 flex-1 max-w-xl">
        {@render transportControls()}
      </div>
      <div class="flex items-center gap-3 w-44 lg:w-72 justify-end shrink-0">
        {@render actionButtons(true)}
      </div>
    </div>

    <!-- ═══ LAYOUT INTERMÉDIAIRE (500-md) : 2 rows ═══
         Row 1 : cover/title (gauche) + actions (droite)
         Row 2 : transport+progress (full width, centré) -->
    <div class="hidden min-[500px]:flex md:hidden relative z-10 flex-col gap-1.5 px-3 py-2">
      <div class="flex items-center justify-between gap-3">
        <div class="flex items-center gap-3 min-w-0 flex-1">
          {@render coverAndTitle()}
        </div>
        <div class="flex items-center gap-2 shrink-0">
          {@render actionButtons(false)}
        </div>
      </div>
      <div class="flex flex-col items-center gap-1.5 px-1">
        {@render transportControls()}
      </div>
    </div>
  </div>

  <!-- Barre d'infos techniques (desktop only — visible dès 500px) -->
  <div class="hidden min-[500px]:flex items-center gap-2 text-[10px] leading-none
              text-neutral-500 dark:text-neutral-500
              bg-neutral-100/80 dark:bg-neutral-900/60
              border-t border-neutral-200/80 dark:border-neutral-800/30
              px-5 py-1.5">
    {#if audioFile}
      <!-- Zone hover : englobe le badge format + rates + flèche + sortie.
           Au survol, on affiche le popover détaillé (PipelineInfoPopover). -->
      <div
        role="group"
        aria-label="Pipeline audio"
        class="relative flex items-center gap-2 shrink-0 cursor-help"
        onmouseenter={() => (showPipelinePopover = true)}
        onmouseleave={() => (showPipelinePopover = false)}
      >
        <div class="flex items-center gap-1 shrink-0">
          {#if isDsdFormat(audioFile?.audio_format)}
            <span class="px-1.5 py-0.5 rounded-sm font-bold tracking-wider
                         bg-amber-500/12 text-amber-600 dark:text-amber-200
                         border border-amber-400/45
                         shadow-[0_0_8px_rgba(251,191,36,0.22)]">
              {dsdLabel(audioFile?.sample_rate)}
            </span>
          {:else if audioFile?.bits_per_sample && audioFile.bits_per_sample >= 24}
            <span class="px-1.5 py-0.5 rounded-sm font-semibold
                         bg-emerald-500/10 text-emerald-600 dark:text-emerald-400">
              Hi-Res {audioFile.bits_per_sample}bit
            </span>
          {/if}

          {#if audioFile?.audio_format && !isDsdFormat(audioFile?.audio_format)}
            <span class="px-1.5 py-0.5 rounded-sm font-semibold uppercase
                         bg-neutral-200/60 dark:bg-white/6 text-neutral-500 dark:text-neutral-400">
              {audioFile.audio_format}
            </span>
          {/if}
        </div>

        <span class="h-2.5 w-px bg-neutral-200/80 dark:bg-neutral-700/30"></span>

        <div class="flex items-center gap-1.5 shrink-0 tabular-nums">
          <!-- Source : pour DSD on garde le badge MHz, sinon "bits/kHz" (style foobar2000). -->
          {#if audioFile?.sample_rate}
            {#if isDsdFormat(audioFile?.audio_format)}
              <span>{formatDsdRate(audioFile.sample_rate)}</span>
            {:else}
              <span>
                {#if audioFile.bits_per_sample}{audioFile.bits_per_sample}/{/if}{audioFile.sample_rate / 1000} kHz
              </span>
            {/if}
          {/if}

          <!-- Flèche + sortie : affichée dès qu'on a une pipeline info.
               Couleur : ambre si resampling / DSD, vert si bit-perfect,
               gris si shared (mixer Windows sans resampling). -->
          {#if $playbackPipelineStore}
            {@const pipe = $playbackPipelineStore}
            {@const targetRate = pipe.intermediate_pcm_rate ?? pipe.output_sample_rate}
            <Icon icon="lucide:arrow-right" width={10} class="text-neutral-400 dark:text-neutral-600" />
            <span
              class={pipe.resampler_active || pipe.intermediate_pcm_rate
                ? 'text-amber-600 dark:text-amber-400/80'
                : pipe.bit_perfect
                  ? 'text-emerald-600 dark:text-emerald-400/80'
                  : 'text-neutral-500 dark:text-neutral-400'}
            >
              {targetRate / 1000} kHz
            </span>
          {/if}
        </div>

        <!-- Badge mode pipeline : DoP / bit-perfect / shared / resamplé / DSD→PCM -->
        {#if pipelineModeState === "dop"}
          <span class="px-1.5 py-0.5 rounded-sm font-semibold text-[9px] tracking-wider uppercase
                       bg-purple-500/10 text-purple-600 dark:text-purple-400
                       border border-purple-500/25">
            {$t("pipeline.badge_dop")}
          </span>
        {:else if pipelineModeState === "bit-perfect"}
          <span class="px-1.5 py-0.5 rounded-sm font-semibold text-[9px] tracking-wider uppercase
                       bg-emerald-500/10 text-emerald-600 dark:text-emerald-400
                       border border-emerald-500/25">
            {$t("pipeline.badge_bit_perfect")}
          </span>
        {:else if pipelineModeState === "resampled"}
          <span class="px-1.5 py-0.5 rounded-sm font-semibold text-[9px] tracking-wider uppercase
                       bg-amber-500/10 text-amber-600 dark:text-amber-400
                       border border-amber-500/25">
            {$t("pipeline.badge_resampled")}
          </span>
        {:else if pipelineModeState === "dsd"}
          <span class="px-1.5 py-0.5 rounded-sm font-semibold text-[9px] tracking-wider uppercase
                       bg-sky-500/10 text-sky-600 dark:text-sky-400
                       border border-sky-500/25">
            {$t("pipeline.badge_dsd_pcm")}
          </span>
        {:else if pipelineModeState === "shared"}
          <span class="px-1.5 py-0.5 rounded-sm font-semibold text-[9px] tracking-wider uppercase
                       bg-neutral-500/10 text-neutral-600 dark:text-neutral-400
                       border border-neutral-500/25">
            {$t("pipeline.badge_shared")}
          </span>
        {/if}

        <!-- Nom du device en gris discret, tronqué -->
        {#if $playbackPipelineStore?.device_name}
          <span class="text-neutral-400 dark:text-neutral-500 truncate max-w-[16rem] hidden xl:inline">
            · {$playbackPipelineStore.device_name}
          </span>
        {/if}

        <!-- Popover détaillé au hover -->
        {#if showPipelinePopover && $playbackPipelineStore}
          <div class="absolute bottom-full left-0 mb-2 z-50">
            <PipelineInfoPopover info={$playbackPipelineStore} />
          </div>
        {/if}
      </div>

      <span class="h-2.5 w-px bg-neutral-200/80 dark:bg-neutral-700/30"></span>

      <div class="flex items-center gap-1.5 shrink-0 tabular-nums">
        {#if audioFile?.bitrate}
          <span>{formatBitrate(audioFile.bitrate)}</span>
        {/if}
        {#if audioFile?.file_size}
          <span class="hidden lg:inline">{Math.round(audioFile.file_size / 1024 / 1024)} Mo</span>
        {/if}
      </div>

      <span class="h-2.5 w-px bg-neutral-200/80 dark:bg-neutral-700/30"></span>
    {:else}
      <span class="text-neutral-300 dark:text-neutral-700">{$t('player.inactive')}</span>
    {/if}

    {#if $player?.pathFile}
      <div class="flex items-center gap-1.5 min-w-0 flex-1">
        <span class="truncate block">
          {truncateMiddle($player.pathFile, 80)}
        </span>
        <button
          class="cursor-pointer shrink-0 hover:text-neutral-600 dark:hover:text-neutral-300 transition-colors"
          onclick={() => handleOpenPath($player.pathFile as string)}
          aria-label="Ouvrir le dossier"
        >
          <Icon icon="ph:folder-open" width="13" height="13" />
        </button>
      </div>
    {/if}

    {#if $player?.pathFile && $player.status !== "playing" && $player?.trackId}
      <button
        class="cursor-pointer shrink-0 hover:text-red-400 transition-colors"
        onclick={async () => player.clearTrack($player.trackId as string)}
        aria-label="Fermer"
      >
        <Icon icon="ph:x-circle" width="13" height="13" />
      </button>
    {/if}

    <!-- Backend audio configuré (toujours visible sur Windows — reflète la
         config globale, pas seulement la lecture en cours). -->
    {#if isWindows}
      <button
        type="button"
        class="ml-auto shrink-0 inline-flex items-center gap-1.5 px-2 py-0.5 rounded-md
               text-[10px] font-semibold uppercase tracking-wider cursor-pointer
               transition-colors
               {wasapiConfigured
                 ? 'bg-amber-500/10 text-amber-600 dark:text-amber-400 border border-amber-500/20 hover:bg-amber-500/15'
                 : 'bg-neutral-500/10 text-neutral-500 dark:text-neutral-400 border border-neutral-500/20 hover:bg-neutral-500/15'}"
        onclick={() => goto('/settings')}
        title={wasapiConfigured
          ? $t('player.wasapi_on_short')
          : $t('player.wasapi_off_short')}
      >
        <Icon icon={wasapiConfigured ? 'lucide:audio-lines' : 'lucide:volume-2'} width="11" />
        {wasapiConfigured ? 'WASAPI' : $t('pipeline.badge_shared')}
      </button>
    {/if}

    <!-- DLNA active indicator (visible only when server is running) -->
    {#if $dlnaStatusStore?.running}
      <button
        type="button"
        class="{isWindows ? '' : 'ml-auto'} shrink-0 inline-flex items-center gap-1.5 px-2 py-0.5 rounded-md
               text-[10px] font-semibold uppercase tracking-wider cursor-pointer
               bg-emerald-500/10 text-emerald-600 dark:text-emerald-400
               border border-emerald-500/20
               hover:bg-emerald-500/15 hover:border-emerald-500/30 transition-colors"
        onclick={() => goto('/settings')}
        title={$dlnaStatusStore.url ?? 'DLNA actif'}
      >
        <span class="relative flex h-1.5 w-1.5">
          <span class="absolute inline-flex h-full w-full animate-ping rounded-full bg-emerald-400 opacity-60"></span>
          <span class="relative inline-flex h-1.5 w-1.5 rounded-full bg-emerald-500"></span>
        </span>
        DLNA
      </button>
    {/if}

    <!-- StatusBar : tâches en cours (import, images, etc.) -->
    <StatusBar />
  </div>
</div>

<!-- Lyrics Panel (sidebar à droite) -->
<LyricsPanel />

