<script lang="ts">
// Fiche d'un morceau : en-tête teinté par la pochette, audio et tags, album et artiste, fichier, le reste de l'album.
import { page } from "$app/state";
import { goto } from "$app/navigation";
import Icon from "@iconify/svelte";
import { untrack } from "svelte";
import { invoke } from "@tauri-apps/api/core";
import { t, currentLocale } from "$lib/i18n";
import { profilSelector } from "$lib/stores/profil/profil.store";
import { liked } from "$lib/stores/playlist/like.store";
import { toasts } from "$lib/stores/ui/toast.store";
import { popinStore } from "$lib/stores/ui/popin.store";
import { queueState } from "$lib/stores/queue/queueState.store";
import { dataCache } from "$lib/stores/cache/dataCache.store";
import { libraryContentStore } from "$lib/stores/library/libraryContent.store";
import { artistImageReadyStore } from "$lib/stores/library/artistImageReady.store";
import { settingsStore } from "$lib/stores/settings/settings.store";
import { loadTrack, loadTracksByAlbum } from "$lib/services/library/library.service";
import { handlePlayTrack } from "$lib/actions/player/PlayerAction";
import { versFileDAttente } from "$lib/mapper/queue/mapQueueTrack";
import { toQueueTrack } from "$lib/helper/tools/queueTools";
import { dureeEcoute, ilYA, minutesSecondes } from "$lib/helper/tools/dateTools";
import { tailleLisible } from "$lib/helper/tools/sizeTools";
import { dsdLabel, formatDsdRate, isDsdFormat, meilleureQualite, palierPiste } from "$lib/helper/tools/audioFormatTools";
import { copierChemin, revelerDansDossier } from "$lib/helper/tools/fileTools";
import { ordreDisque } from "$lib/helper/library/cleTri";
import { lireTagsFichier } from "$lib/helper/library/tagsFichier";
import DetailPage from "$lib/components/ui/layout/DetailPage.svelte";
import PageState from "$lib/components/ui/layout/PageState.svelte";
import PlayButton from "$lib/components/ui/button/PlayButton.svelte";
import RoundButton from "$lib/components/ui/button/RoundButton.svelte";
import CoverImg from "$lib/components/ui/image/CoverImg.svelte";
import ImgZoom from "$lib/components/ui/tools/ImgZoom.svelte";
import QualityBadge from "$lib/components/ui/text/QualityBadge.svelte";
import StarRating from "$lib/components/ui/rating/StarRating.svelte";
import TrackContextMenu from "$lib/components/ui/contextmenu/TrackContextMenu.svelte";
import EditTagsPopin from "$lib/components/library/common/popin/EditTagsPopin.svelte";
import AlbumTrackRow from "$lib/components/library/album/AlbumTrackRow.svelte";
import type { Library } from "$lib/types/db/library/Library";
import type { TrackDetailView } from "$lib/types/ui/library/track/TrackDetailView";
import type { TrackListView } from "$lib/types/ui/library/track/TrackListView";

/** Artistes crédités ; `library_artist_id` vide quand l'artiste n'est pas dans cette bibliothèque. */
type Credit = { artist_id: string; library_artist_id: string | null; name: string };

const libraryId = $derived(Number(page.params.library_id));
const trackId = $derived(page.params.track_id as string);
const profil = $derived($profilSelector.profilSelected);

let track = $state<TrackDetailView | null>(null);
let credits = $state<Credit[]>([]);
let pistesAlbum = $state<TrackListView[]>([]);
let erreur = $state(false);
let defilement = $state<HTMLDivElement | null>(null);
let menu = $state<{ x: number; y: number } | null>(null);


// ─── Chargement : le morceau, ses crédits, puis les titres de son album ───
let tour = 0;

$effect(() => {
  const id = libraryId;
  const tid = trackId;
  const p = profil;
  if (!$profilSelector.initialized) return;
  if (!p || !id || !tid) {
    erreur = true;
    return;
  }
  // Hors suivi : `charger` réécrit l'état qu'il lit.
  const n = ++tour;
  untrack(() => charger(id, tid, p.id, n));
});

async function charger(libId: number, tid: string, profilId: number, n: number) {
  erreur = false;
  try {
    const [lib, m, c] = await Promise.all([
      invoke<Library>("get_library", { libraryId: libId }),
      loadTrack(tid),
      invoke<Credit[]>("get_track_artists", { trackId: tid }).catch(() => [] as Credit[]),
    ]);
    if (n !== tour) return;
    if (!lib || lib.profil_id !== profilId) {
      goto("/");
      return;
    }
    if (!m) {
      erreur = true;
      return;
    }
    const autreAlbum = m.album_id !== track?.album_id;
    track = m;
    credits = c ?? [];
    defilement?.scrollTo({ top: 0 });
    if (!m.album_id) {
      pistesAlbum = [];
      return;
    }
    if (autreAlbum) pistesAlbum = [];
    const r = await dataCache.lire(`album-tracks:${m.album_id}`, () => loadTracksByAlbum(libId, m.album_id!), (v) => { if (n === tour) pistesAlbum = v; });
    if (n === tour) pistesAlbum = r ?? [];
  } catch (e) {
    if (n === tour) erreur = true;
    console.error("[morceau] chargement :", e);
  }
}

// ─── L'album dans l'ordre du disque (numéros absents : ordre naturel des fichiers) ───
const ordreAlbum = $derived([...pistesAlbum].sort(ordreDisque));
const rang = $derived(track ? ordreAlbum.findIndex((x) => x.id === track!.id) : -1);
const precedent = $derived(rang > 0 ? ordreAlbum[rang - 1].id : null);
const suivant = $derived(rang >= 0 && rang < ordreAlbum.length - 1 ? ordreAlbum[rang + 1].id : null);
// La version « liste » du morceau porte les tags complets et l'id d'artiste.
const courante = $derived(rang >= 0 ? ordreAlbum[rang] : null);

const album = $derived(track?.album_id ? ($libraryContentStore.albums.find((a) => a.id === track!.album_id) ?? null) : null);
const artisteId = $derived(courante?.artist_id ?? credits[0]?.artist_id ?? null);
const artiste = $derived(artisteId ? ($libraryContentStore.artists.find((a) => a.id === artisteId) ?? null) : null);

// Sans pochette embarquée dans le fichier, celle de l'album (Deezer, choisie à la main) : l'en-tête n'est plus vide.
const pochette = $derived(track?.thumbnail_path ?? album?.cover_url ?? courante?.thumbnail_path ?? null);

// Portrait : connu, arrivé pendant la visite, sinon demandé à Deezer comme sur la fiche artiste.
let portraitCharge = $state<{ id: string; url: string | null } | null>(null);
const portrait = $derived(artisteId
  ? (artistImageReadyStore.get(artisteId, $artistImageReadyStore) ?? artiste?.thumbnail_path ?? (portraitCharge?.id === artisteId ? portraitCharge.url : null))
  : null);
$effect(() => {
  const ida = artisteId;
  const nom = artiste?.name ?? credits[0]?.name ?? track?.artist ?? null;
  if (!ida || !nom || untrack(() => portrait) || $settingsStore.auto_download_artist_images === "false") return;
  untrack(() => {
    if (portraitCharge?.id === ida) return;
    portraitCharge = { id: ida, url: null };
    invoke<string | null>("fetch_artist_image", { artistId: ida, artistName: nom })
      .then((url) => { if (portraitCharge?.id === ida) portraitCharge = { id: ida, url }; })
      .catch(() => {});
  });
});

// Tags complets : compositeur, totaux de pistes et de disques, label.
const tags = $derived.by(() => {
  const vide = { compositeur: null as string | null, totalPistes: null as number | null, totalDisques: null as number | null, label: null as string | null };
  const tg = lireTagsFichier(courante?.tags);
  if (!tg) return vide;
  const { champs: d, perso } = tg;
  const entier = (v: unknown) => (Number(v) > 0 ? Number(v) : null);
  return {
    compositeur: (d.composer as string) || perso.get("COMPOSER") || null,
    totalPistes: entier(d.total_tracks) ?? entier(perso.get("TRACKTOTAL")) ?? entier(perso.get("TOTALTRACKS")),
    totalDisques: entier(d.total_discs) ?? entier(perso.get("DISCTOTAL")) ?? entier(perso.get("TOTALDISCS")),
    label: (d.publisher as string) || perso.get("LABEL") || perso.get("ORGANIZATION") || null,
  };
});
const numero = $derived(track?.track_number ?? (rang >= 0 ? rang + 1 : null));
const totalPistes = $derived(tags.totalPistes ?? (pistesAlbum.length || null));

// ─── Affichage ───
const nombre = (n: number) => n.toLocaleString($currentLocale);
const dateLongue = (d: string | null | undefined) => {
  if (!d) return null;
  const x = new Date(d.includes("T") ? d : d.replace(" ", "T") + "Z");
  return isNaN(x.getTime()) ? null : x.toLocaleDateString($currentLocale, { day: "numeric", month: "long", year: "numeric" });
};
const khz = (r: number) => `${(r / 1000).toLocaleString($currentLocale, { maximumFractionDigits: 1 })} kHz`;
const aime = $derived(track ? $liked.paths.has(track.path) : false);
const qualite = $derived(track ? meilleureQualite([track]) : null);
const dsd = $derived(!!track && (track.bits_per_sample === 1 || isDsdFormat(track.audio_format)));

// Résumé de la qualité : intitulé, détail et palier.
const resume = $derived.by(() => {
  if (!track) return null;
  const f = (track.audio_format ?? track.extension ?? "").toUpperCase();
  const palier = palierPiste(track.audio_format, track.bits_per_sample, track.sample_rate);
  if (dsd) return { palier, titre: $t("track_view.q_dsd"), detail: `${dsdLabel(track.sample_rate)} · ${formatDsdRate(track.sample_rate)}`, niveau: "HI-RES" };
  const reso = track.bits_per_sample && track.sample_rate ? `${track.bits_per_sample} bit / ${khz(track.sample_rate)}` : "";
  if (palier === "hires") return { palier, titre: $t("track_view.q_hires"), detail: `${f} ${$t("track_view.lossless_word")} · ${reso}`, niveau: "HI-RES" };
  if (palier === "lossless") {
    const cd = track.bits_per_sample === 16 && track.sample_rate === 44100;
    return { palier, titre: $t(cd ? "track_view.q_cd" : "track_view.q_lossless"), detail: [`${f} ${$t("track_view.lossless_word")}`, reso].filter(Boolean).join(" · "), niveau: "LOSSLESS" };
  }
  return { palier, titre: $t("track_view.q_lossy"), detail: [`${f} ${$t("track_view.lossy_word")}`, track.bitrate ? `${nombre(track.bitrate)} kb/s` : ""].filter(Boolean).join(" · "), niveau: "LOSSY" };
});

const canaux = (c: number | null) => (c === 1 ? $t("track_view.mono") : c === 2 ? $t("track_view.stereo") : c === 6 ? "5.1" : c === 8 ? "7.1" : c ? `${c} ch` : "—");
const audio = $derived(track ? [
  { l: "track_view.duration", v: track.duration ? minutesSecondes(track.duration) : "—" },
  { l: "track_view.format", v: (track.audio_format ?? track.extension ?? "—").toUpperCase() },
  { l: "track_view.bitrate", v: track.bitrate ? `${nombre(track.bitrate)} kb/s` : "—" },
  { l: "track_view.sample_rate", v: track.sample_rate ? (dsd ? formatDsdRate(track.sample_rate) : khz(track.sample_rate)) : "—" },
  { l: "track_view.resolution", v: track.bits_per_sample ? `${track.bits_per_sample} bit` : "—" },
  { l: "track_view.channels", v: canaux(track.channels) },
  { l: "track_view.size", v: tailleLisible(track.file_size ?? track.size ?? 0, $currentLocale) || "—" },
  { l: "track_view.plays", v: nombre(track.play_count ?? 0), discret: !track.play_count },
] : []);

const ecoute = $derived.by(() => {
  if (!track?.play_count) return $t("track_view.never_played");
  const fois = $t(track.play_count === 1 ? "track_view.played_once" : "track_view.played_n").replace("{n}", nombre(track.play_count));
  return track.last_played_at ? `${fois} · ${ilYA(track.last_played_at, $currentLocale)}` : fois;
});

// Dossier et nom du fichier, le second mis en avant.
const chemin = $derived.by(() => {
  const p = track?.path ?? "";
  const i = Math.max(p.lastIndexOf("\\"), p.lastIndexOf("/"));
  return { dossier: p.slice(0, i + 1), fichier: p.slice(i + 1) };
});

// ─── Actions ───
function lire() {
  if (!track) return;
  handlePlayTrack(track.path, ordreAlbum.length ? versFileDAttente(ordreAlbum) : undefined);
}

function lireEnsuite() {
  if (!track) return;
  queueState.addTrack(toQueueTrack(track));
  toasts.push({ type: "success", title: $t("track_view.queued_next"), message: track.title });
}

function modifierTags() {
  if (!track) return;
  popinStore.open($t("tags.edit"), EditTagsPopin, {
    path: track.path,
    onsaved: () => toasts.push({ type: "success", title: $t("tags.saved"), message: "" }),
  }, { size: "xl", flush: true, icon: "material-symbols:sell-outline-rounded" });
}

const copier = () => track && copierChemin(track.path, chemin.fichier);
const ouvrirDossier = () => track && revelerDansDossier(track.path);

const carteCls = "min-w-0 rounded-2xl border px-5.5 py-5 bg-(--rg-carte) border-(--rg-bd)";
const titreCarte = "mb-4 flex items-center gap-2.5 text-xs font-bold tracking-widest uppercase text-(--rg-mu)";
const ligneDl = "flex justify-between gap-3 py-2.75 min-w-0 border-b border-(--rg-line)";
const lien = "text-(--rg-tx) hover:underline underline-offset-2";
const petitBouton = "h-8 pl-2 pr-2.5 shrink-0 flex items-center gap-1.5 rounded-lg text-[12.5px] font-semibold whitespace-nowrap cursor-pointer transition-colors text-(--rg-tx2) hover:bg-(--rg-s2) hover:text-(--rg-tx)";
</script>

{#if !track && !erreur}
  <PageState chargement message={$t("track_view.loading")} />

{:else if !track}
  <PageState icon="material-symbols:music-note-rounded" message={$t("track_view.error")} lien={{ href: `/library/${libraryId}/tracks`, label: $t("track_view.back") }} />

{:else}
<DetailPage bind:defilement image={pochette} retourHref={`/library/${libraryId}/tracks`} retourLabel={$t("track_view.back")}
            playLabel={$t("track_view.play")} onplay={lire}>
  {#snippet mini()}
    <span class="w-8.5 h-8.5 shrink-0 rounded-[7px] overflow-hidden bg-(--rg-s2)">
      {#if pochette}<CoverImg path={pochette} alt="" size="1x" class="w-full h-full object-cover" />{/if}
    </span>
    <b class="truncate text-[15px] text-(--rg-tx)">{track?.title}</b>
    <span class="shrink-0 text-[13px] text-(--rg-mu) max-sm:hidden">{track?.artist ?? ""}</span>
  {/snippet}
  {#snippet barre()}
    <div class="flex gap-0.5">
      {#each [{ id: precedent, i: "material-symbols:chevron-left-rounded", l: "track_view.prev" }, { id: suivant, i: "material-symbols:chevron-right-rounded", l: "track_view.next" }] as o (o.l)}
        <button type="button" disabled={!o.id} title={$t(o.l)} aria-label={$t(o.l)}
                class="w-8 h-8 flex items-center justify-center rounded-lg cursor-pointer transition-colors text-(--rg-mu) hover:bg-(--rg-s2) hover:text-(--rg-tx) disabled:opacity-30 disabled:cursor-default disabled:hover:bg-transparent"
                onclick={() => o.id && goto(`/library/${libraryId}/tracks/${o.id}`)}>
          <Icon icon={o.i} width="22" />
        </button>
      {/each}
    </div>
  {/snippet}

    <!-- ─── En-tête ─── -->
    <div class="relative flex flex-wrap items-end gap-8 pt-3 pb-7">
      <div class="relative w-58 @max-[760px]:w-45 aspect-square shrink-0 rounded-[14px] overflow-hidden shadow-[0_24px_60px_rgba(0,0,0,0.3)] dark:shadow-[0_24px_60px_rgba(0,0,0,0.55)]">
        {#if pochette}
          <ImgZoom path={pochette} alt={track.title}>
            <CoverImg path={pochette} alt={track.title} class="w-full h-full object-cover" />
          </ImgZoom>
        {:else}
          <div class="w-full h-full flex items-center justify-center text-(--rg-mu2) shadow-[inset_0_0_0_1px_var(--rg-bd)]
                      bg-[repeating-linear-gradient(135deg,var(--rg-s2)_0_10px,var(--rg-carte)_10px_20px)]">
            <Icon icon="material-symbols:music-note-rounded" width="64" />
          </div>
        {/if}
        <span class="absolute inset-0 rounded-[14px] ring-1 ring-inset ring-black/5 dark:ring-white/6 pointer-events-none"></span>
      </div>

      <div class="flex-[1_1_320px] min-w-0 flex flex-col gap-2.5">
        <div class="flex flex-wrap items-center gap-2.5 text-xs font-bold tracking-widest uppercase text-(--rg-tx2)">
          {$t("track_view.kicker")}
          {#if qualite}<QualityBadge palier={qualite.palier} texte={qualite.texte} class="px-2 py-0.75" />{/if}
        </div>

        <h1 class="font-extrabold leading-none tracking-[-0.03em] text-balance line-clamp-3 text-(--rg-tx)
                   {track.title.length > 40 ? 'text-[44px] @max-[760px]:text-[30px]' : 'text-[56px] @max-[760px]:text-[38px]'}" title={track.title}>{track.title}</h1>

        <div class="flex flex-wrap items-center gap-x-3 gap-y-2 text-[15px] text-(--rg-mu)">
          {#if credits.length}
            <span class="flex flex-wrap items-center gap-x-1">
              {#each credits as c, i (c.artist_id)}
                {#if i > 0}<span class="text-(--rg-mu2)">,</span>{/if}
                {#if c.library_artist_id}
                  <a href={`/library/${libraryId}/artists/${c.artist_id}`} class="font-bold {lien}">{c.name}</a>
                {:else}
                  <span class="font-bold text-(--rg-tx)">{c.name}</span>
                {/if}
              {/each}
            </span>
          {:else if track.artist}
            {#if artisteId}<a href={`/library/${libraryId}/artists/${artisteId}`} class="font-bold {lien}">{track.artist}</a>{:else}<span class="font-bold text-(--rg-tx)">{track.artist}</span>{/if}
          {/if}
          {#if track.album}
            <span class="text-(--rg-mu2)">·</span>
            {#if track.album_id}<a href={`/library/${libraryId}/albums/${track.album_id}`} class="font-bold {lien}">{track.album}</a>{:else}<span>{track.album}</span>{/if}
          {/if}
          {#if Number(track.year)}
            <span class="text-(--rg-mu2)">·</span>
            <a href={`/library/${libraryId}/albums?annee=${Number(track.year)}`} class="hover:text-(--rg-tx) hover:underline underline-offset-2"
               title={$t("album_view.year_title").replace("{y}", String(Number(track.year)))}>{Number(track.year)}</a>
          {/if}
          {#if track.duration}
            <span class="text-(--rg-mu2)">·</span>
            <span class="tabular-nums">{minutesSecondes(track.duration)}</span>
          {/if}
        </div>

        <div class="flex flex-wrap gap-x-4 gap-y-1.5 text-[13px] text-(--rg-mu)">
          {#if numero}
            <span class="whitespace-nowrap">{$t("track_view.track_label")} <b class="font-semibold text-(--rg-tx2)">{numero}</b>{#if totalPistes}{" "}{$t("track_view.of_total").replace("{total}", String(totalPistes))}{/if}</span>
          {/if}
          <span class="whitespace-nowrap">{ecoute}</span>
          {#if dateLongue(track.created_at)}
            <span class="whitespace-nowrap">{$t("track_view.added")} <b class="font-semibold text-(--rg-tx2)">{dateLongue(track.created_at)}</b></span>
          {/if}
        </div>

        <div class="flex flex-wrap items-center gap-2.5 mt-2">
          <PlayButton label={$t("track_view.play")} onclick={lire} />
          <RoundButton icon="material-symbols:queue-music-rounded" title={$t("track_view.play_next")} onclick={lireEnsuite} />
          <RoundButton icon={aime ? "material-symbols:favorite-rounded" : "material-symbols:favorite-outline-rounded"} pressed={aime}
                       title={$t("tracks_view.like")} onclick={() => track && liked.toggle(track.path)} />
          <div class="h-12 px-3.5 flex items-center rounded-full border border-(--rg-bd2) bg-white/40 dark:bg-black/25" title={$t("track_view.rate")}>
            <StarRating trackId={track.id} value={track.rating} size={22} ton="or" onchange={(r) => { if (track) track.rating = r; }} />
          </div>
          <RoundButton icon="material-symbols:more-horiz" title={$t("album_view.more")}
                       onclick={(e) => { const r = (e.currentTarget as HTMLElement).getBoundingClientRect(); menu = { x: r.left, y: r.bottom + 6 }; }} />
        </div>
      </div>
    </div>

    <!-- ─── Audio et tags ─── -->
    <div class="grid grid-cols-[minmax(0,1.15fr)_minmax(0,1fr)] gap-5 mt-2 @max-[900px]:grid-cols-1">
      <section class={carteCls}>
        <h3 class={titreCarte}>{$t("track_view.audio")}</h3>
        {#if resume}
          <div class="flex items-center gap-4 pt-1 pb-4.5 mb-1.5 border-b border-(--rg-bd)">
            <span class="w-12 h-12 shrink-0 rounded-xl border flex items-center justify-center bg-(--rg-s2) border-(--rg-bd2) text-(--rg-tx2)">
              <Icon icon="material-symbols:graphic-eq-rounded" width="24" />
            </span>
            <div class="min-w-0">
              <b class="block text-[22px] font-extrabold tracking-[-0.01em] text-(--rg-tx)">{resume.titre}</b>
              <small class="block mt-0.75 text-[13px] text-(--rg-mu)">{resume.detail}</small>
            </div>
            <QualityBadge palier={resume.palier} texte={resume.niveau} mono={false} class="ml-auto px-2.5 py-1" />
          </div>
        {/if}
        <dl class="grid grid-cols-2 gap-x-7 @max-[900px]:grid-cols-1">
          {#each audio as x, i (x.l)}
            <div class="{ligneDl} {i >= audio.length - 2 ? '@min-[900px]:border-b-0' : ''} {i === audio.length - 1 ? 'border-b-0' : ''}">
              <dt class="text-[13.5px] whitespace-nowrap text-(--rg-mu)">{$t(x.l)}</dt>
              <dd class="text-sm font-semibold text-right tabular-nums truncate {x.discret ? 'text-(--rg-mu) font-medium' : 'text-(--rg-tx)'}">{x.v}</dd>
            </div>
          {/each}
        </dl>
      </section>

      <section class={carteCls}>
        <h3 class={titreCarte}>
          {$t("track_view.tags")}<span class="flex-1"></span>
          <button type="button" class="h-7.5 pl-2 pr-2.5 flex items-center gap-1.5 rounded-lg border text-[12.5px] font-semibold tracking-normal normal-case cursor-pointer transition-colors border-(--rg-bd2) text-(--rg-tx2) hover:bg-(--rg-s2) hover:text-(--rg-tx)"
                  onclick={modifierTags}>
            <Icon icon="material-symbols:edit-outline-rounded" width="16" />{$t("track_view.edit")}
          </button>
        </h3>
        <dl class="[&>div:last-child]:border-b-0">
          {#snippet ligne(libelle: string)}
            <dt class="text-[13.5px] whitespace-nowrap text-(--rg-mu)">{libelle}</dt>
          {/snippet}
          <div class={ligneDl}>
            {@render ligne($t("track_view.album_artist"))}
            <dd class="text-sm font-semibold text-right truncate text-(--rg-tx)">
              {#if track.album_artist && album?.artist_id}<a href={`/library/${libraryId}/artists/${album.artist_id}`} class={lien}>{track.album_artist}</a>{:else}{track.album_artist ?? "—"}{/if}
            </dd>
          </div>
          <div class={ligneDl}>
            {@render ligne($t("track_view.genre"))}
            <dd class="text-sm font-semibold text-right truncate text-(--rg-tx)">
              {#if track.genre}<a href={`/library/${libraryId}/genres/${encodeURIComponent(track.genre)}`} class={lien}>{track.genre}</a>{:else}—{/if}
            </dd>
          </div>
          <div class={ligneDl}>
            {@render ligne($t("track_view.year"))}
            <dd class="text-sm font-semibold text-right tabular-nums text-(--rg-tx)">{track.year || "—"}</dd>
          </div>
          <div class={ligneDl}>
            {@render ligne($t("track_view.track"))}
            <dd class="text-sm font-semibold text-right tabular-nums text-(--rg-tx)">{track.track_number ?? "—"}{#if tags.totalPistes}<span class="font-medium text-(--rg-mu)">{" / "}{tags.totalPistes}</span>{/if}</dd>
          </div>
          <div class={ligneDl}>
            {@render ligne($t("track_view.disc"))}
            <dd class="text-sm font-semibold text-right tabular-nums text-(--rg-tx)">{track.disc_number || 1}{#if tags.totalDisques}<span class="font-medium text-(--rg-mu)">{" / "}{tags.totalDisques}</span>{/if}</dd>
          </div>
          {#if tags.compositeur}
            <div class={ligneDl}>
              {@render ligne($t("track_view.composer"))}
              <dd class="text-sm font-semibold text-right truncate text-(--rg-tx)" title={tags.compositeur}>{tags.compositeur}</dd>
            </div>
          {/if}
          {#if tags.label}
            <div class={ligneDl}>
              {@render ligne($t("album_view.label"))}
              <dd class="text-sm font-semibold text-right truncate text-(--rg-tx)" title={tags.label}>{tags.label}</dd>
            </div>
          {/if}
        </dl>
      </section>
    </div>

    <!-- ─── Album et artiste ─── -->
    {#if track.album_id || artisteId}
      <div class="grid grid-cols-2 gap-5 mt-5 @max-[900px]:grid-cols-1">
        {#if track.album_id}
          <a href={`/library/${libraryId}/albums/${track.album_id}`}
             class="group flex items-center gap-4 min-w-0 py-3.5 pl-3.5 pr-4 rounded-2xl border transition-colors bg-(--rg-carte) border-(--rg-bd) hover:border-(--rg-bd2) hover:bg-(--rg-hover)">
            <span class="w-16 h-16 shrink-0 rounded-[10px] overflow-hidden bg-(--rg-s2)">
              {#if album?.cover_url ?? track.thumbnail_path}<CoverImg path={album?.cover_url ?? track.thumbnail_path} alt="" size="1x" class="w-full h-full object-cover" />{/if}
            </span>
            <span class="flex-1 min-w-0">
              <small class="block text-[11px] font-bold tracking-widest uppercase text-(--rg-mu)">{$t("track_view.album")}</small>
              <b class="block mt-0.75 truncate text-[17px] font-bold text-(--rg-tx)">{track.album ?? album?.title}</b>
              <span class="block mt-0.5 text-[13px] text-(--rg-mu)">
                {[album?.year, `${nombre(album?.total_tracks ?? pistesAlbum.length)} ${$t((album?.total_tracks ?? pistesAlbum.length) === 1 ? "library_head.tracks_one" : "library_head.tracks_n")}`, album?.total_duration ? dureeEcoute(album.total_duration) : null].filter(Boolean).join(" · ")}
              </span>
            </span>
            <Icon icon="material-symbols:chevron-right-rounded" width="22" class="shrink-0 text-(--rg-mu2) group-hover:text-(--rg-tx)" />
          </a>
        {/if}
        {#if artisteId}
          <a href={`/library/${libraryId}/artists/${artisteId}`}
             class="group flex items-center gap-4 min-w-0 py-3.5 pl-3.5 pr-4 rounded-2xl border transition-colors bg-(--rg-carte) border-(--rg-bd) hover:border-(--rg-bd2) hover:bg-(--rg-hover)">
            <span class="w-16 h-16 shrink-0 rounded-full overflow-hidden flex items-center justify-center bg-(--rg-s2) text-(--rg-mu)">
              {#if portrait}<CoverImg path={portrait} alt="" size="1x" class="w-full h-full object-cover" />{:else}<Icon icon="material-symbols:person-rounded" width="30" />{/if}
            </span>
            <span class="flex-1 min-w-0">
              <small class="block text-[11px] font-bold tracking-widest uppercase text-(--rg-mu)">{$t("artist_view.kicker")}</small>
              <b class="block mt-0.75 truncate text-[17px] font-bold text-(--rg-tx)">{artiste?.name ?? track.artist}</b>
              {#if artiste}
                <span class="block mt-0.5 text-[13px] text-(--rg-mu)">
                  {nombre(artiste.total_albums)} {$t(artiste.total_albums === 1 ? "library_head.albums_one" : "library_head.albums_n")} · {nombre(artiste.total_tracks)} {$t(artiste.total_tracks === 1 ? "library_head.tracks_one" : "library_head.tracks_n")}
                </span>
              {/if}
            </span>
            <Icon icon="material-symbols:chevron-right-rounded" width="22" class="shrink-0 text-(--rg-mu2) group-hover:text-(--rg-tx)" />
          </a>
        {/if}
      </div>
    {/if}

    <!-- ─── Fichier ─── -->
    <section class="{carteCls} mt-5">
      <h3 class={titreCarte}>{$t("track_view.file")}</h3>
      <div class="flex items-center gap-2.5 py-3 pl-4 pr-3 rounded-xl border bg-(--c-fond) dark:bg-zinc-950 border-(--rg-bd)">
        <span class="flex-1 min-w-0 font-mono text-[12.5px] text-(--rg-mu) wrap-anywhere">{chemin.dossier}<span class="text-(--rg-tx)">{chemin.fichier}</span></span>
        <button type="button" class={petitBouton} onclick={copier}>
          <Icon icon="material-symbols:content-copy-outline-rounded" width="17" />{$t("track_view.copy")}
        </button>
      </div>
      <div class="flex flex-wrap items-center gap-x-4.5 gap-y-1.5 mt-3 text-[13px] text-(--rg-mu)">
        {#if dateLongue(track.created_at)}<span>{$t("track_view.added")} <b class="font-semibold text-(--rg-tx2)">{dateLongue(track.created_at)}</b></span>{/if}
        {#if dateLongue(track.last_scanned_at)}<span>{$t("track_view.scanned")} <b class="font-semibold text-(--rg-tx2)">{dateLongue(track.last_scanned_at)}</b></span>{/if}
        <span class="flex-1"></span>
        <button type="button" class={petitBouton} onclick={ouvrirDossier}>
          <Icon icon="material-symbols:folder-outline-rounded" width="17" />{$t("track_view.reveal")}
        </button>
      </div>
    </section>

    <!-- ─── Sur le même album ─── -->
    {#if ordreAlbum.length > 1}
      <section class="mt-12">
        <div class="flex flex-wrap items-end gap-3 mb-4">
          <div class="flex flex-col gap-1 min-w-0">
            <h2 class="text-[22px] font-extrabold tracking-[-0.01em] text-(--rg-tx)">{$t("track_view.same_album")}</h2>
            <span class="text-[13px] text-(--rg-mu)">{track.album} · {nombre(ordreAlbum.length)} {$t("library_head.tracks_n")}</span>
          </div>
          <span class="flex-1"></span>
          <a href={`/library/${libraryId}/albums/${track.album_id}`}
             class="h-8.5 pl-3 pr-2 flex items-center gap-0.5 rounded-[9px] text-sm font-semibold transition-colors text-(--rg-mu) hover:bg-(--rg-s2) hover:text-(--rg-tx)">
            {$t("track_view.see_album")}<Icon icon="material-symbols:chevron-right-rounded" width="20" />
          </a>
        </div>
        <div class="grid grid-cols-2 grid-flow-col gap-x-7 gap-y-0.5 grid-rows-[repeat(var(--rangs),auto)]
                    @max-[900px]:grid-cols-1 @max-[900px]:grid-flow-row @max-[900px]:grid-rows-none"
             style="--rangs: {Math.ceil(ordreAlbum.length / 2)}">
          {#each ordreAlbum as x, i (x.id)}
            <AlbumTrackRow {libraryId} track={x} tracks={ordreAlbum} jaquette={false} artisteAlbum={album?.artist ?? track.album_artist}
                           position={i + 1} marque={x.id === track.id} sousTitre={x.id === track.id ? $t("track_view.this_track") : null} />
          {/each}
        </div>
      </section>
    {/if}
</DetailPage>

{#if menu}
  <TrackContextMenu {track} x={menu.x} y={menu.y} {libraryId} showNavigation={true} onclose={() => (menu = null)} />
{/if}
{/if}
