<script lang="ts">
  // Mini-lecteur : fenêtre compacte teintée par la pochette, file et paroles en onglets repliables.
  import Icon from "@iconify/svelte";
  import MiniVolume from "./MiniVolume.svelte";
  import AnneauAttente from "#lib/components/ui/loader/AnneauAttente.svelte";
  import { volume } from "#lib/stores/player/volume.store";
  import { fade } from "svelte/transition";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import CoverImg from "#lib/components/ui/image/CoverImg.svelte";
  import LogoRustMusic from "#lib/components/ui/logo/LogoRustMusic.svelte";
  import PlayerProgressBar from "./PlayerProgressBar.svelte";
  import { t, currentLocale } from "#lib/i18n";
  import { player } from "#lib/stores/player/player.store";
  import { playerService } from "#lib/services/player/player.service";
  import { queueState } from "#lib/stores/queue/queueState.store";
  import { liked } from "#lib/stores/playlist/like.store";
  import { settingsStore } from "#lib/stores/settings/settings.store";
  import { couleurMorceau } from "#lib/stores/player/trackColor.store";
  import { playbackPipelineStore, pipelineMode } from "#lib/stores/player/playbackPipeline.store";
  import { CHAINE } from "#lib/helper/audio/chaineAudio";
  import { displayTitle } from "#lib/helper/tools/stringTools";
  import { minutesSecondes } from "#lib/helper/tools/dateTools";
  import { dsdLabel, formatDsdRate, isDsdFormat } from "#lib/helper/tools/audioFormatTools";
  import { getLyrics, type Lyrics } from "#lib/services/lyrics/lyrics.service";
  import { parseLrc, findActiveLineIndex, type LrcLine } from "#lib/helper/lyrics/lrcParser";
  import { exitMiniPlayer, enterMicroPlayer, setMiniExpanded, reportCollapsedHeight, miniPinned, toggleMiniPin } from "#lib/stores/ui/miniPlayer.store";
  import type { QueueTrack } from "#lib/types/db/queue/QueueTrack";

  const audioFile = $derived($player?.audioFile);
  const audioTags = $derived(audioFile?.tags);
  const pathFile = $derived($player?.pathFile ?? null);
  const hasTrack = $derived(!!pathFile);
  const isPlaying = $derived($player?.status === "playing");
  const enAttente = $derived(isPlaying && !!$player?.isPreparing);
  const trackTitle = $derived(displayTitle(audioTags?.title, pathFile, $t("common.unknown_title")));
  const coverSrc = $derived(audioTags?.attached_images?.[0]?.image_src ?? null);
  const duration = $derived($player?.duration ?? 0);
  const jsPosition = $derived($player?.jsPosition ?? 0);
  const percent = $derived(duration ? Math.min(100, Math.max(0, (jsPosition / duration) * 100)) : 0);
  const aime = $derived(!!pathFile && $liked.paths.has(pathFile));

  // « FLAC » + « 16/44,1 » ; le DSD n'a qu'une fréquence.
  const nombre = $derived(new Intl.NumberFormat($currentLocale, { maximumFractionDigits: 1 }));
  const dsd = $derived(isDsdFormat(audioFile?.audio_format));
  const format = $derived(!audioFile ? null : dsd ? dsdLabel(audioFile.sample_rate) : audioFile.audio_format?.toUpperCase() ?? null);
  const qualite = $derived.by(() => {
    const sr = audioFile?.sample_rate;
    if (!sr) return "";
    if (dsd) return formatDsdRate(sr);
    const khz = nombre.format(sr / 1000);
    return audioFile?.bits_per_sample ? `${audioFile.bits_per_sample}/${khz}` : `${khz} kHz`;
  });
  const chaine = $derived.by(() => {
    const m = pipelineMode($playbackPipelineStore, $volume);
    return m ? CHAINE[m] : null;
  });

  // ─── Onglets : recliquer l'onglet ouvert le replie ───
  type Onglet = "queue" | "lyrics" | null;
  let onglet = $state<Onglet>(null);
  function basculer(o: Exclude<Onglet, null>) {
    onglet = onglet === o ? null : o;
    setMiniExpanded(onglet !== null);
  }

  // ─── File : le titre en cours en tête, puis la suite ───
  const currentIndex = $derived($queueState.currentIndex);
  const enCours = $derived(currentIndex > -1 ? ($queueState.tracks[currentIndex] ?? null) : null);
  const suite = $derived(currentIndex > -1 ? $queueState.tracks.slice(currentIndex + 1) : $queueState.tracks);
  const aVenir = $derived(suite.length + (enCours ? 1 : 0));

  const reel = (i: number) => (currentIndex > -1 ? currentIndex + 1 : 0) + i;

  async function jouer(track: QueueTrack, i: number) {
    await queueState.setCurrentIndex(reel(i));
    await playerService.playFile(track);
  }

  // Glisser-déposer par la poignée (elle capture le pointeur), comme dans la file complète.
  let depart = $state<number | null>(null);
  let survol = $state<number | null>(null);
  function saisir(e: PointerEvent, i: number) {
    if (e.button !== 0) return;
    e.preventDefault();
    e.stopPropagation();
    depart = i;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  }
  function deplacer(e: PointerEvent) {
    if (depart === null) return;
    const ligne = document.elementFromPoint(e.clientX, e.clientY)?.closest("[data-qi]") as HTMLElement | null;
    if (ligne) survol = Number(ligne.dataset.qi);
  }
  function lacher() {
    if (depart !== null && survol !== null && depart !== survol) queueState.reorderTracks(reel(depart), reel(survol));
    depart = null;
    survol = null;
  }

  // ─── Paroles synchronisées ───
  let lignes = $state<LrcLine[]>([]);
  let texteBrut = $state<string | null>(null);
  let source = $state<Lyrics["source"] | null>(null);
  let etat = $state<"idle" | "loading" | "ready" | "empty">("idle");
  let parolesDe = "";
  let boite = $state<HTMLDivElement | null>(null);
  const elLignes: Record<number, HTMLElement> = {};

  const active = $derived(lignes.length > 0 ? findActiveLineIndex(lignes, jsPosition * 1000) : -1);

  async function chargerParoles(path: string) {
    etat = "loading";
    lignes = [];
    texteBrut = null;
    source = null;
    parolesDe = path;
    try {
      const res = await getLyrics(path);
      if (parolesDe !== path) return;
      if (!res || (!res.synced && !res.plain)) {
        etat = "empty";
        return;
      }
      if (res.synced) lignes = parseLrc(res.synced);
      texteBrut = res.plain;
      source = res.source;
      etat = lignes.length > 0 || res.plain ? "ready" : "empty";
    } catch {
      etat = "empty";
    }
  }

  $effect(() => {
    if (onglet === "lyrics" && pathFile && pathFile !== parolesDe) chargerParoles(pathFile);
  });

  // La ligne courante au milieu ; sans animation à l'ouverture.
  let dejaCentre = false;
  $effect(() => {
    const i = active;
    if (onglet !== "lyrics" || i < 0 || !boite) {
      dejaCentre = false;
      return;
    }
    const el = elLignes[i];
    if (!el) return;
    const haut = el.offsetTop - boite.clientHeight / 2 + el.offsetHeight / 2;
    boite.scrollTo({ top: haut, behavior: dejaCentre ? "smooth" : "instant" });
    dejaCentre = true;
  });

  const SOURCES: Record<string, string> = { sidecar: "mini.src_sidecar", lrclib: "mini.src_lrclib", manual: "mini.src_manual" };

  // ─── Fenêtre ───
  let entete = $state<HTMLDivElement | null>(null);
  $effect(() => {
    const el = entete;
    if (!el) return;
    reportCollapsedHeight(el.offsetHeight);
    const ro = new ResizeObserver(() => reportCollapsedHeight(el.offsetHeight));
    ro.observe(el);
    return () => ro.disconnect();
  });

  // Même règle que la barre de titre. Avant de quitter, la fenêtre (masquée) reprend sa taille
  // normale : c'est celle-là que l'appli retiendra pour le prochain lancement.
  async function fermer() {
    const w = getCurrentWindow();
    if ($settingsStore.minimize_to_tray === "true") return w.hide();
    await w.hide();
    await exitMiniPlayer();
    await w.close();
  }

  // L'image « sans pochette » de l'appli, plutôt qu'un carré vide.
  const SANS_POCHETTE = "/images/no-cd.png";

  const tb = "w-7 h-7 flex items-center justify-center rounded-lg cursor-pointer transition-colors text-(--lc-ic) hover:bg-(--lc-survol) hover:text-(--lc-tx)";
  const ib = "w-9 h-9 flex items-center justify-center rounded-full cursor-pointer transition-colors text-(--lc-ic-fort) hover:bg-(--lc-survol)";
  const entetePanneau = "flex items-center justify-between px-4 pt-3 pb-1.5 text-[10px] font-bold uppercase tracking-[0.12em] text-(--lc-mu)";
</script>

{#snippet vignette(path: string | null | undefined)}
  <span class="w-10 h-10 shrink-0 rounded-md overflow-hidden bg-(--lc-survol)">
    {#if path && path !== SANS_POCHETTE}
      <CoverImg {path} alt="" size="1x" class="w-full h-full object-cover" />
    {:else}
      <img src={SANS_POCHETTE} alt="" class="w-full h-full object-cover" />
    {/if}
  </span>
{/snippet}

<div
  class="lecteur mini-lecteur fixed inset-0 flex flex-col overflow-hidden select-none"
  style:--lh={$couleurMorceau.h}
  style:--ls={$couleurMorceau.s}
>
  <!-- Partie repliée, mesurée : la fenêtre s'y ajuste au pixel. -->
  <div bind:this={entete} class="shrink-0">
    <div class="h-8.5 flex items-center gap-0.5 pl-3 pr-1.5">
      <!-- Le vrai logo, comme la barre de titre ; toute la zone déplace la fenêtre. -->
      <div data-tauri-drag-region class="flex-1 h-full flex items-center gap-1 cursor-grab">
        <Icon icon="material-symbols-light:drag-indicator" width="16" class="shrink-0 text-(--lc-ic) opacity-60 pointer-events-none" />
        <span class="flex pointer-events-none"><LogoRustMusic width={104} /></span>
      </div>
      <button type="button" class="{tb} {$miniPinned ? 'text-(--lc-acc)!' : ''}" title={$t("mini.pin")} aria-label={$t("mini.pin")} aria-pressed={$miniPinned} onclick={toggleMiniPin}>
        <Icon icon={$miniPinned ? "material-symbols:keep" : "material-symbols-light:keep-outline"} width="17" />
      </button>
      <button type="button" class={tb} title={$t("mini.micro")} aria-label={$t("mini.micro")} onclick={() => enterMicroPlayer()}>
        <Icon icon="ph:picture-in-picture" width="17" />
      </button>
      <button type="button" class={tb} title={$t("mini.restore")} aria-label={$t("mini.restore")} onclick={() => exitMiniPlayer()}>
        <Icon icon="material-symbols-light:open-in-full-rounded" width="17" />
      </button>
      <button type="button" class="{tb} hover:bg-[#c42b1c]! hover:text-white!" title={$t("common.close")} aria-label={$t("common.close")} onclick={fermer}>
        <Icon icon="material-symbols-light:close-rounded" width="18" />
      </button>
    </div>

    <!-- Pochette, titre, qualité · j'aime -->
    <div class="flex items-center gap-3.5 px-4 pt-1">
      <div class="lecteur-pochette w-21 h-21 shrink-0 rounded-[10px] overflow-hidden">
        <img src={hasTrack && coverSrc ? coverSrc : SANS_POCHETTE} alt="" class="w-full h-full object-cover" />
      </div>
      <div class="min-w-0 flex-1 flex flex-col gap-0.75">
        <span class="truncate text-lg font-extrabold tracking-[-0.01em] text-(--lc-tx)" title={trackTitle}>{hasTrack ? trackTitle : $t("player.no_track")}</span>
        {#if audioTags?.artist}<span class="truncate text-sm font-medium text-(--lc-acc)">{audioTags.artist}</span>{/if}
        {#if hasTrack && (format || qualite)}
          <div class="mt-1.25 flex items-center gap-2 min-w-0 font-mono text-[10px] text-(--lc-tx2)">
            {#if format}<span class="px-1.75 py-0.5 rounded-[5px] font-medium tracking-[0.05em] bg-(--lc-fmt-bg) text-(--lc-fmt-tx)">{format}</span>{/if}
            {#if qualite}<span class="whitespace-nowrap">{qualite}</span>{/if}
            {#if chaine}
              <span class="flex items-center gap-1.25 min-w-0 whitespace-nowrap"><span class="w-1.25 h-1.25 shrink-0 rounded-full {chaine.point}"></span><span class="truncate">{$t(chaine.cle)}</span></span>
            {/if}
          </div>
        {/if}
      </div>
      {#if hasTrack}
        <button type="button" class="self-start w-8 h-8 shrink-0 flex items-center justify-center rounded-full cursor-pointer transition-colors hover:bg-(--lc-survol)
                                    {aime ? 'text-[#e2566a] dark:text-[#ff7a88]' : 'text-(--lc-ic) hover:text-(--lc-tx)'}"
                title={$t("player.like")} aria-label={$t("player.like")} aria-pressed={aime} onclick={() => pathFile && liked.toggle(pathFile)}>
          <Icon icon={aime ? "material-symbols:favorite-rounded" : "material-symbols-light:favorite-outline-rounded"} width="20" />
        </button>
      {/if}
    </div>

    <!-- Commandes et progression -->
    <div class="flex items-center gap-3 px-4 pt-3">
      <div class="flex items-center gap-0.5 shrink-0">
        <button type="button" class={ib} title={$t("player.prev")} aria-label={$t("player.prev")} onclick={() => playerService.prevTrack()}>
          <Icon icon="material-symbols:skip-previous-rounded" width="28" />
        </button>
        <button type="button" class="lecteur-lecture relative w-11.5 h-11.5 flex items-center justify-center rounded-full cursor-pointer transition-transform hover:scale-105 active:scale-95 bg-(--lc-play) text-(--lc-play-tx)"
                title={enAttente ? $t("player.preparing") : isPlaying ? $t("player.pause") : $t("player.play")} aria-label={isPlaying ? $t("player.pause") : $t("player.play")}
                aria-busy={enAttente} onclick={() => playerService.handleTogglePlay()}>
          <Icon icon={isPlaying ? "material-symbols:pause-rounded" : "material-symbols:play-arrow-rounded"} width="28" />
          <AnneauAttente actif={enAttente} />
        </button>
        <button type="button" class={ib} title={$t("player.next")} aria-label={$t("player.next")} onclick={() => playerService.nextTrack()}>
          <Icon icon="material-symbols:skip-next-rounded" width="28" />
        </button>
      </div>
      <div class="flex-1 min-w-0 flex flex-col gap-0.5">
        <PlayerProgressBar position={percent} onseek={(p) => duration && playerService.seekTo((p / 100) * duration)} />
        <div class="flex justify-between font-mono text-[10px] tabular-nums text-(--lc-mu)">
          <span>{minutesSecondes(jsPosition)}</span><span>{minutesSecondes(duration)}</span>
        </div>
      </div>
      <MiniVolume buttonClass={ib} />
    </div>

    <!-- Onglets File / Paroles -->
    <div class="grid grid-cols-2 gap-1 mx-3 mt-3.5 mb-3 p-0.75 rounded-[11px] border bg-(--lc-seg) border-(--lc-seg-bd)" role="tablist">
      {#each [{ id: "queue", icone: "material-symbols-light:queue-music-rounded", libelle: $t("mini.queue_tab") }, { id: "lyrics", icone: "material-symbols-light:lyrics-outline-rounded", libelle: $t("mini.lyrics") }] as o (o.id)}
        {@const ouvert = onglet === o.id}
        <button type="button" role="tab" aria-selected={ouvert}
                class="h-8 flex items-center justify-center gap-1.75 rounded-lg text-[13px] font-semibold cursor-pointer transition-colors
                       {ouvert ? 'bg-(--lc-seg-on) text-(--lc-tx) shadow-[0_1px_2px_rgba(0,0,0,0.08),inset_0_1px_0_rgba(255,255,255,0.06)]' : 'text-(--lc-tx2) hover:text-(--lc-tx)'}"
                onclick={() => basculer(o.id as "queue" | "lyrics")}>
          <Icon icon={o.icone} width="18" class={ouvert ? "text-(--lc-acc)" : ""} />
          {o.libelle}
          {#if o.id === "queue" && aVenir > 0}
            <span class="px-1.5 py-px rounded-lg font-mono text-[10px] bg-(--lc-survol) text-(--lc-tx2)">{aVenir}</span>
          {/if}
        </button>
      {/each}
    </div>
  </div>

  <!-- Panneau déroulé -->
  {#if onglet}
    <div class="relative flex-1 min-h-0 flex flex-col border-t border-(--lc-trait) bg-(--lc-panneau)" in:fade={{ duration: 160, delay: 60 }}>
      {#if onglet === "queue"}
        <div class="flex-1 min-h-0 overflow-y-auto scrollbar-app pb-2">
          {#if enCours}
            <p class={entetePanneau}>{$t("mini.now_playing")}</p>
            <div class="flex items-center gap-3 mx-1.5 py-1.5 pl-2 pr-2.5 rounded-[10px] bg-(--lc-encours)">
              <span class="w-3.5 shrink-0"></span>
              {@render vignette(enCours.cover)}
              <span class="min-w-0 flex-1">
                <span class="block truncate text-sm font-semibold text-(--lc-acc)">{enCours.title}</span>
                <span class="block truncate text-xs text-(--lc-mu)">{enCours.artist ?? ""}</span>
              </span>
              <span class="mini-eq flex items-end gap-0.5 h-3 shrink-0" data-pause={isPlaying ? undefined : ""}><i></i><i></i><i></i></span>
            </div>
          {/if}

          <div class={entetePanneau}>
            <span>{suite.length === 1 ? $t("mini.next_one") : $t("mini.next_count").replace("{n}", String(suite.length))}</span>
            {#if suite.length > 0 && enCours}
              <button type="button" class="normal-case tracking-normal text-[11px] font-semibold cursor-pointer text-(--lc-mu) hover:text-(--lc-tx)"
                      title={$t("mini.clear_next_hint")} onclick={() => queueState.clearUpcoming()}>{$t("mini.clear_next")}</button>
            {/if}
          </div>

          {#if suite.length === 0}
            <p class="px-4 py-4 text-xs text-(--lc-mu)">{$t("mini.queue_empty")}</p>
          {:else}
            {#each suite as track, i (track.queueId)}
              <div
                data-qi={i}
                role="button"
                tabindex="0"
                class="group flex items-center gap-3 mx-1.5 py-1.5 pl-2 pr-2.5 rounded-[10px] cursor-pointer transition-colors hover:bg-(--lc-survol)
                       {depart === i ? 'opacity-50' : ''} {depart !== null && survol === i && depart !== i ? 'shadow-[inset_0_2px_0_var(--lc-acc)]' : ''}"
                onclick={() => jouer(track, i)}
                onkeydown={(e) => { if (e.key === "Enter") jouer(track, i); }}
              >
                <span class="w-3.5 shrink-0 flex justify-center text-(--lc-mu2) opacity-0 group-hover:opacity-100 cursor-grab active:cursor-grabbing touch-none"
                      title={$t("mini.move")} role="presentation" onpointerdown={(e) => saisir(e, i)} onpointermove={deplacer} onpointerup={lacher}
                      onclick={(e) => e.stopPropagation()}>
                  <Icon icon="material-symbols-light:drag-indicator" width="16" />
                </span>
                {@render vignette(track.cover)}
                <span class="min-w-0 flex-1">
                  <span class="block truncate text-sm font-semibold text-(--lc-tx)">{track.title}</span>
                  <span class="block truncate text-xs text-(--lc-mu)">{track.artist ?? ""}</span>
                </span>
                {#if track.duration}
                  <span class="font-mono text-[11px] tabular-nums text-(--lc-mu) group-hover:hidden">{minutesSecondes(track.duration)}</span>
                {/if}
                <button type="button" class="hidden group-hover:flex w-6.5 h-6.5 items-center justify-center rounded-md cursor-pointer text-(--lc-mu) hover:bg-(--lc-survol) hover:text-(--lc-tx)"
                        title={$t("mini.remove")} aria-label={$t("mini.remove")} onclick={(e) => { e.stopPropagation(); queueState.removeTrack(track.queueId); }}>
                  <Icon icon="material-symbols-light:close-rounded" width="16" />
                </button>
              </div>
            {/each}
          {/if}
        </div>
      {:else}
        <div bind:this={boite} class="mini-paroles relative flex-1 min-h-0 overflow-y-auto scrollbar-app">
          {#if etat === "loading"}
            <p class="py-24 text-center text-xs text-(--lc-mu)">{$t("mini.lyrics_loading")}</p>
          {:else if lignes.length > 0}
            <div class="h-27.5"></div>
            {#each lignes as ligne, i (i)}
              <button
                type="button"
                bind:this={elLignes[i]}
                class="block w-full text-left px-5 py-1.25 text-[17px] font-bold leading-[1.35] text-pretty origin-left cursor-pointer
                       transition-[color,transform] duration-250 hover:text-(--lc-tx2)
                       {i === active ? 'text-(--lc-tx) scale-104 first-letter:text-(--lc-acc)' : i < active ? 'text-(--lc-ly-passe)' : 'text-(--lc-ly)'}"
                title={$t("mini.jump_line")}
                onclick={() => playerService.seekTo(ligne.timeMs / 1000)}
              >{ligne.text || "♪"}</button>
            {/each}
            <div class="h-27.5"></div>
          {:else if texteBrut}
            <p class="px-5 py-4 text-sm leading-relaxed whitespace-pre-wrap text-(--lc-tx2)">{texteBrut}</p>
          {:else}
            <p class="py-24 text-center text-xs text-(--lc-mu)">{$t("mini.lyrics_empty")}</p>
          {/if}
        </div>
        {#if etat === "ready"}
          <span class="shrink-0 h-6.5 flex items-center px-4 text-[10px] text-(--lc-mu2) border-t border-(--lc-trait)">
            {lignes.length > 0 ? $t("mini.lyrics_synced") : $t("mini.lyrics_plain")}{#if source && SOURCES[source]} · {$t(SOURCES[source])}{/if}
          </span>
        {/if}
      {/if}
    </div>
  {/if}
</div>
