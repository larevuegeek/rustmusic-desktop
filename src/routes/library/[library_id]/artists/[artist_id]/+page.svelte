<script lang="ts">
// Page d'un artiste : portrait et chiffres, discographie, tous ses titres regroupés par album, puis ses voisins.
import { page } from "$app/state";
import { goto } from "$app/navigation";
import Icon from "@iconify/svelte";
import { tick, untrack } from "svelte";
import { invoke } from "@tauri-apps/api/core";
import { t, currentLocale } from "$lib/i18n";
import { profilSelector } from "$lib/stores/profil/profil.store";
import { selectionStore } from "$lib/stores/ui/selection.store";
import { viewMode } from "$lib/stores/ui/viewMode.store";
import { settingsStore } from "$lib/stores/settings/settings.store";
import { liked } from "$lib/stores/playlist/like.store";
import { toasts } from "$lib/stores/ui/toast.store";
import { loadArtist } from "$lib/services/library/library.service";
import { handleTracksPlay } from "$lib/actions/queue/QueueAction";
import { clesAffichees, resoudreColonnes, lireLargeurs, resetTagCache, trierPistes, type SortDir } from "$lib/config/trackColumns";
import { dureeEcoute } from "$lib/helper/tools/dateTools";
import { meilleureQualite } from "$lib/helper/tools/audioFormatTools";
import { lireLocal, ecrireLocal } from "$lib/helper/tools/stockage";
import { cleTri, comparerNaturel, ordreDisque } from "$lib/helper/library/cleTri";
import CoverImg from "$lib/components/ui/image/CoverImg.svelte";
import ImgZoom from "$lib/components/ui/tools/ImgZoom.svelte";
import QualityBadge from "$lib/components/ui/text/QualityBadge.svelte";
import DetailPage from "$lib/components/ui/layout/DetailPage.svelte";
import PageState from "$lib/components/ui/layout/PageState.svelte";
import PlayButton from "$lib/components/ui/button/PlayButton.svelte";
import RoundButton from "$lib/components/ui/button/RoundButton.svelte";
import ToolbarButton from "$lib/components/ui/button/ToolbarButton.svelte";
import ChipLink from "$lib/components/ui/link/ChipLink.svelte";
import SegmentedControl from "$lib/components/ui/input/SegmentedControl.svelte";
import ViewModeSwitch from "$lib/components/ui/input/ViewModeSwitch.svelte";
import SelectionToggle from "$lib/components/ui/selection/SelectionToggle.svelte";
import CarouselSection from "$lib/components/ui/carousel/CarouselSection.svelte";
import TrackColumnsButton from "$lib/components/library/track/TrackColumnsButton.svelte";
import LibraryItemMenu from "$lib/components/ui/contextmenu/LibraryItemMenu.svelte";
import AlbumTrackRow from "$lib/components/library/album/AlbumTrackRow.svelte";
import LibraryTrackTable from "$lib/components/library/track/LibraryTrackTable.svelte";
import AlbumListItem from "$lib/components/library/album/AlbumListItem.svelte";
import AlbumListRow from "$lib/components/library/album/AlbumListRow.svelte";
import ArtistListItem from "$lib/components/library/artist/ArtistListItem.svelte";
import ArtistListRow from "$lib/components/library/artist/ArtistListRow.svelte";
import type { Library } from "$lib/types/db/library/Library";
import type { ArtistDetailView } from "$lib/types/ui/library/artist/ArtistDetailView";
import type { ArtistListView } from "$lib/types/ui/library/artist/ArtistListView";
import type { AlbumListView } from "$lib/types/ui/library/album/AlbumListView";
import type { TrackListView } from "$lib/types/ui/library/track/TrackListView";

const libraryId = $derived(Number(page.params.library_id));
const artistId = $derived(String(page.params.artist_id));
const profil = $derived($profilSelector.profilSelected);

let artist = $state<ArtistDetailView | null>(null);
let portrait = $state<string | null>(null);
let tracks = $state<TrackListView[]>([]);
let albums = $state<AlbumListView[]>([]);
let similaires = $state<ArtistListView[]>([]);
let chargeTitres = $state(false);
let chargeAlbums = $state(false);
let erreur = $state(false);
let defilement = $state<HTMLDivElement | null>(null);

// ─── Chargement : l'en-tête d'abord, puis titres, albums et voisins en parallèle ───
let tour = 0;
let demande = "";

$effect(() => {
  const id = libraryId;
  const aid = artistId;
  const p = profil;
  if (!$profilSelector.initialized) return;
  if (!p || !id || !aid) {
    erreur = true;
    return;
  }
  // Hors suivi : `charger` réécrit l'état qu'il lit.
  const n = ++tour;
  untrack(() => charger(id, aid, p.id, n));
});

async function charger(libId: number, aid: string, profilId: number, n: number) {
  erreur = false;
  // Autre artiste : on vide (l'id d'adresse peut différer de `artist.id`).
  if (demande !== aid) {
    demande = aid;
    artist = null;
    portrait = null;
    tracks = [];
    albums = [];
    similaires = [];
  }
  chargeTitres = chargeAlbums = false;
  try {
    const [lib, a] = await Promise.all([invoke<Library>("get_library", { libraryId: libId }), loadArtist(aid)]);
    if (n !== tour) return;
    if (!lib || lib.profil_id !== profilId) {
      goto("/");
      return;
    }
    if (!a) {
      erreur = true;
      return;
    }
    artist = a;
    portrait = a.thumbnail_path ?? null;
    defilement?.scrollTo({ top: 0 });
  } catch (e) {
    if (n === tour) erreur = true;
    console.error("[artiste] chargement :", e);
    return;
  }

  // Portrait depuis Deezer : seul appel réseau automatique, mis en cache en base (absence comprise).
  if ($settingsStore.auto_download_artist_images !== "false" && artist?.name) {
    invoke<string | null>("fetch_artist_image", { artistId: artist.id, artistName: artist.name })
      .then((url) => { if (n === tour && url) portrait = url; })
      .catch(() => {});
  }

  // L'adresse peut porter l'id de bibliothèque de l'artiste : les requêtes veulent l'id d'artiste.
  const ida = artist!.id;
  invoke<TrackListView[]>("get_tracks_by_artist", { libraryId: libId, artistId: ida })
    .then((r) => {
      if (n !== tour) return;
      tracks = r ?? [];
      ouvrirPremier();
    })
    .catch((e) => console.error("[artiste] titres :", e))
    .finally(() => { if (n === tour) chargeTitres = true; });

  invoke<AlbumListView[]>("get_albums_by_artist", { libraryId: libId, artistId: ida })
    .then((r) => { if (n === tour) albums = r ?? []; })
    .catch((e) => console.error("[artiste] albums :", e))
    .finally(() => { if (n === tour) chargeAlbums = true; });

  invoke<ArtistListView[]>("get_similar_artists", { libraryId: libId, artistId: ida, limit: 12 })
    .then((r) => { if (n === tour) similaires = (r ?? []).filter((x) => !COMPILATIONS.test(x.name)).slice(0, 10); })
    .catch((e) => console.error("[artiste] similaires :", e));
}

// « Various Artists » n'est pas un artiste voisin.
const COMPILATIONS = /^(various artists?|va|artistes? divers|compilations?)$/i;

$effect(() => {
  void tracks;
  resetTagCache();
});

// Teinte de la lueur : le portrait, à défaut la première pochette.
const imageTeinte = $derived(portrait ?? albums.find((a) => a.cover_url)?.cover_url ?? null);

// ─── Chiffres ───
const nombre = (n: number) => n.toLocaleString($currentLocale);
const albumsPropres = $derived(albums.filter((a) => !a.participation));
const participations = $derived(albums.filter((a) => a.participation));
const duree = $derived(tracks.reduce((s, x) => s + (x.duration ?? 0), 0) || artist?.total_duration || 0);
const nbAlbums = $derived(chargeAlbums ? albumsPropres.length : (artist?.total_albums ?? 0));
const nbTitres = $derived(chargeTitres ? tracks.length : (artist?.total_tracks ?? 0));
const periode = $derived.by(() => {
  const ans = albumsPropres.map((a) => a.year ?? 0).filter((y) => y > 0);
  if (!ans.length) return null;
  const [min, max] = [Math.min(...ans), Math.max(...ans)];
  return min === max ? String(min) : `${min} – ${max}`;
});
const plusieurs = (n: number, un: string, n_: string) => `${$t(n === 1 ? un : n_)}`;

// Trois genres les plus présents dans ses titres.
const genres = $derived.by(() => {
  const n = new Map<string, number>();
  for (const x of tracks) if (x.genre?.trim()) n.set(x.genre.trim(), (n.get(x.genre.trim()) ?? 0) + 1);
  return [...n.entries()].sort((a, b) => b[1] - a[1]).slice(0, 3).map(([g]) => g);
});

// ─── Favoris : l'artiste l'est quand tous ses titres le sont ───
const tousAimes = $derived(tracks.length > 0 && tracks.every((x) => $liked.paths.has(x.path)));
async function basculerFavori() {
  const aimer = !tousAimes;
  try {
    await liked.toggleAll(tracks.map((x) => x.path), aimer);
    toasts.push({ type: "success", title: $t(aimer ? "album_view.fav_added" : "album_view.fav_removed"), message: artist?.name ?? "" });
  } catch (e) {
    console.error("[artiste] favoris :", e);
  }
}

// ─── Tous les titres : regroupés par album (année ou nom), sinon à plat ───
let tri = $state<string>("year");
let sens = $state<SortDir>("desc");
const groupe = $derived(tri === "year" || tri === "album");

type Groupe = { cle: string; album: AlbumListView | null; titre: string; annee: number | null; cover: string | null; pistes: TrackListView[] };
const albumsParId = $derived(new Map(albums.map((a) => [a.id, a])));
const parTitre = (x: string, y: string) => comparerNaturel.compare(cleTri(x), cleTri(y));

const groupes = $derived.by((): Groupe[] => {
  if (!groupe) return [];
  const m = new Map<string, TrackListView[]>();
  for (const x of tracks) {
    const cle = x.album_id ?? `?${x.album ?? ""}`;
    if (!m.has(cle)) m.set(cle, []);
    m.get(cle)!.push(x);
  }
  const liste = [...m].map(([cle, pistes]): Groupe => {
    const a = albumsParId.get(cle) ?? null;
    const p = pistes[0];
    return {
      cle,
      album: a,
      titre: a?.title ?? p.album ?? $t("artist_view.unknown_album"),
      annee: a?.year ?? (Number(p.year) || null),
      cover: a?.cover_url ?? p.thumbnail_path ?? null,
      pistes: pistes.sort(ordreDisque),
    };
  });
  const s = sens === "asc" ? 1 : -1;
  return tri === "year"
    ? liste.sort((x, y) => s * ((x.annee ?? 0) - (y.annee ?? 0)) || parTitre(x.titre, y.titre))
    : liste.sort((x, y) => s * parTitre(x.titre, y.titre));
});
const aPlat = $derived(groupe ? [] : trierPistes(tracks, tri, sens));
// La file de lecture : l'ordre affiché, albums repliés compris.
const ordreLecture = $derived(groupe ? groupes.flatMap((g) => g.pistes) : aPlat);

let ouverts = $state<Set<string>>(new Set());
function ouvrirPremier() {
  const premier = groupes[0]?.cle;
  ouverts = new Set(premier ? [premier] : []);
}
function basculer(cle: string) {
  const s = new Set(ouverts);
  if (!s.delete(cle)) s.add(cle);
  ouverts = s;
}
const toutOuvert = $derived(groupes.length > 0 && groupes.every((g) => ouverts.has(g.cle)));
function toutBasculer() {
  ouverts = toutOuvert ? new Set() : new Set(groupes.map((g) => g.cle));
}

// Un nouvel ordre repart du premier album ouvert, comme la maquette.
function trierPar(cle: string) {
  if (tri === cle) sens = sens === "asc" ? "desc" : "asc";
  else {
    tri = cle;
    sens = cle === "year" ? "desc" : "asc";
  }
  ouvrirPremier();
}

// L'ordre affiché, pour Maj + clic (le tableau déclare le sien).
const visibles = $derived(groupe ? groupes.flatMap((g) => (ouverts.has(g.cle) ? g.pistes : [])) : aPlat);
$effect(() => {
  if ($viewMode !== "list") selectionStore.setOrder(visibles.map((x) => ({ id: x.id, track: x })));
});

// Clic sur une carte de la discographie : son album s'ouvre plus bas.
async function allerA(id: string) {
  if (!groupe) {
    tri = "year";
    sens = "desc";
  }
  if (!groupes.some((g) => g.cle === id)) {
    goto(`/library/${libraryId}/albums/${id}`);
    return;
  }
  ouverts = new Set([...ouverts, id]);
  await tick();
  const el = defilement?.querySelector<HTMLElement>(`[data-groupe="${id}"]`);
  if (!el || !defilement) return;
  const haut = el.getBoundingClientRect().top - defilement.getBoundingClientRect().top + defilement.scrollTop - 70;
  defilement.scrollTo({ top: haut, behavior: "smooth" });
  el.animate([{ boxShadow: "inset 0 0 0 2px var(--rg-g)" }, { boxShadow: "inset 0 0 0 2px transparent" }], { duration: 1400, easing: "ease-out" });
}

// Jaquettes par titre : utiles à plat, superflues sous l'en-tête d'album.
let jaquettes = $state(lireLocal("artiste:jaquettes", "0") === "1");
function basculerJaquettes() {
  jaquettes = !jaquettes;
  ecrireLocal("artiste:jaquettes", jaquettes ? "1" : "0");
}

const colonnes = $derived(resoudreColonnes(clesAffichees($settingsStore.track_columns)).filter((c) => c.key !== "artist"));
const largeurs = $derived(lireLargeurs($settingsStore.track_column_widths));

// Discographie : même ordre que les titres (nom, sinon année récente d'abord).
const discoParNom = $derived(tri === "album" || tri === "title");
const discographie = $derived(
  discoParNom
    ? [...albumsPropres].sort((x, y) => parTitre(x.title, y.title))
    : [...albumsPropres].sort((x, y) => (y.year ?? 0) - (x.year ?? 0) || parTitre(x.title, y.title)),
);

let artistMenu = $state<{ x: number; y: number } | null>(null);
const grille = $derived($viewMode !== "list");

const flecheSens = (cle: string) => (tri === cle ? (sens === "asc" ? "material-symbols:arrow-upward-rounded" : "material-symbols:arrow-downward-rounded") : null);
</script>

<!-- Cartes et lignes des sections (hors de DetailPage : sinon ce seraient ses props). -->
{#snippet carteDisco(a: AlbumListView)}<AlbumListItem {libraryId} album={a} onopen={() => allerA(a.id)} />{/snippet}
{#snippet ligneDisco(a: AlbumListView)}<AlbumListRow {libraryId} album={a} onopen={() => allerA(a.id)} />{/snippet}
{#snippet carteAlbum(a: AlbumListView)}<AlbumListItem {libraryId} album={a} />{/snippet}
{#snippet ligneAlbum(a: AlbumListView)}<AlbumListRow {libraryId} album={a} />{/snippet}
{#snippet carteArtiste(a: ArtistListView)}<ArtistListItem {libraryId} artist={a} />{/snippet}
{#snippet ligneArtiste(a: ArtistListView)}<ArtistListRow {libraryId} artist={a} />{/snippet}

<!-- En-tête d'album repliable, collé sous la barre tant qu'il est ouvert. -->
{#snippet enteteGroupe(cle: string)}
  {@const g = groupes.find((x) => x.cle === cle)}
  {#if g}
    {@const ouvert = ouverts.has(g.cle)}
    {@const q = g.album ? meilleureQualite(g.pistes) : null}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <!-- Le tableau défile de côté : l'en-tête d'album, lui, reste en place. -->
    <div data-groupe={g.cle} role="button" tabindex="0" aria-expanded={ouvert}
         style="transform: translateX(var(--tx, 0px)); max-width: calc(var(--vue, 100%) + 1rem)"
         class="group/gh -mx-2 pl-2 pr-3 py-2.5 flex items-center gap-3.5 rounded-xl cursor-pointer select-none transition-colors hover:bg-(--rg-carte)
                {ouvert ? 'sticky top-15 z-9 mb-1.5 border-b border-(--rg-bd) bg-(--c-fond) dark:bg-zinc-950' : ''}"
         onclick={() => basculer(g.cle)}
         onkeydown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); basculer(g.cle); } }}>
      <span class="w-8 h-8 shrink-0 flex items-center justify-center rounded-lg text-(--rg-mu) group-hover/gh:text-(--rg-tx) transition-transform duration-200 {ouvert ? '' : '-rotate-90'}">
        <Icon icon="material-symbols:expand-more-rounded" width="22" />
      </span>
      <span class="w-13 h-13 shrink-0 rounded-[9px] overflow-hidden bg-(--rg-s2) shadow-[0_6px_16px_rgba(0,0,0,0.2)] dark:shadow-[0_6px_16px_rgba(0,0,0,0.4)]">
        {#if g.cover}<CoverImg path={g.cover} alt="" size="1x" class="w-full h-full object-cover" />{/if}
      </span>
      <div class="flex-1 min-w-0 flex flex-col gap-0.75">
        <b class="truncate text-[17px] font-extrabold tracking-[-0.01em] text-(--rg-tx)" title={g.titre}>{g.titre}</b>
        <span class="flex items-center gap-2 whitespace-nowrap overflow-hidden text-[13px] text-(--rg-mu)">
          {#if g.annee}<span>{g.annee}</span><span>·</span>{/if}
          <span>{nombre(g.pistes.length)} {plusieurs(g.pistes.length, "library_head.tracks_one", "library_head.tracks_n")}</span>
          <span>·</span>
          <span>{dureeEcoute(g.pistes.reduce((s, x) => s + (x.duration ?? 0), 0))}</span>
          {#if q}<QualityBadge palier={q.palier} texte={q.texte} class="px-1.5 py-0.5 max-sm:hidden" />{/if}
        </span>
      </div>
      {#if g.album}
        <a href={`/library/${libraryId}/albums/${g.album.id}`} onclick={(e) => e.stopPropagation()}
           class="h-8.5 pl-3 pr-2 shrink-0 flex items-center gap-0.5 rounded-[9px] text-[13px] font-semibold transition-colors text-(--rg-mu) hover:bg-(--rg-s2) hover:text-(--rg-tx) @max-[760px]:hidden">
          {$t("artist_view.open_album")}<Icon icon="material-symbols:chevron-right-rounded" width="20" />
        </a>
      {/if}
      <PlayButton size="md" muted={!ouvert} title={$t("artist_view.play_album")} onclick={(e) => { e.stopPropagation(); handleTracksPlay(g.pistes); }} />
    </div>
  {/if}
{/snippet}

{#snippet grilleTitres(pistes: TrackListView[], aPlat: boolean)}
  <!-- Deux colonnes lues de haut en bas, une seule quand la place manque. -->
  <div class="grid grid-cols-2 grid-flow-col gap-x-7 gap-y-0.5 grid-rows-[repeat(var(--rangs),auto)]
              @max-[900px]:grid-cols-1 @max-[900px]:grid-flow-row @max-[900px]:grid-rows-none"
       style="--rangs: {Math.ceil(pistes.length / 2)}">
    {#each pistes as track, i (track.id)}
      <AlbumTrackRow {libraryId} {track} tracks={ordreLecture} jaquette={jaquettes} artisteAlbum={artist?.name ?? null}
                     position={i + 1} numero={aPlat ? i + 1 : null}
                     sousTitre={aPlat ? [track.album, track.year].filter(Boolean).join(" · ") : null} />
    {/each}
  </div>
{/snippet}

{#if !artist && !erreur}
  <PageState chargement message={$t("artist_view.loading")} />

{:else if !artist}
  <PageState icon="material-symbols:person-outline-rounded" message={$t("artist_view.error")} lien={{ href: `/library/${libraryId}/artists`, label: $t("artist_view.back") }} />

{:else}
<DetailPage bind:defilement image={imageTeinte} retourHref={`/library/${libraryId}/artists`} retourLabel={$t("artist_view.back")}
            playLabel={$t("artist_view.play_all")} onplay={() => handleTracksPlay(ordreLecture)}>
  {#snippet mini()}
    <span class="w-8.5 h-8.5 shrink-0 rounded-full overflow-hidden flex items-center justify-center bg-(--rg-s2) text-(--rg-mu)">
      {#if portrait}<CoverImg path={portrait} alt="" size="1x" class="w-full h-full object-cover" />{:else}<Icon icon="material-symbols:person-rounded" width="18" />{/if}
    </span>
    <b class="truncate text-[15px] text-(--rg-tx)">{artist?.name}</b>
  {/snippet}
  {#snippet barre()}<ViewModeSwitch class="bg-(--rg-carte)/80" />{/snippet}

    <!-- ─── En-tête ─── -->
    <div class="relative flex flex-wrap items-end gap-8 pt-3 pb-7">
      <div class="relative w-55 @max-[760px]:w-40 aspect-square shrink-0 rounded-full overflow-hidden shadow-[0_24px_60px_rgba(0,0,0,0.3)] dark:shadow-[0_24px_60px_rgba(0,0,0,0.55)]">
        {#if portrait}
          <ImgZoom path={portrait} alt={artist.name}>
            <CoverImg path={portrait} alt={artist.name} class="w-full h-full object-cover" />
          </ImgZoom>
        {:else}
          <div class="w-full h-full flex items-center justify-center text-(--rg-mu) bg-(--rg-s2)">
            <Icon icon="material-symbols:person-rounded" width="96" />
          </div>
        {/if}
        <span class="absolute inset-0 rounded-full ring-1 ring-inset ring-black/5 dark:ring-white/6 pointer-events-none"></span>
      </div>

      <div class="flex-[1_1_320px] min-w-0 flex flex-col gap-3">
        <div class="text-xs font-bold tracking-[0.1em] uppercase text-(--rg-tx2)">{$t("artist_view.kicker")}</div>
        <h1 class="font-extrabold leading-none tracking-[-0.03em] text-balance line-clamp-2 text-(--rg-tx)
                   {artist.name.length > 24 ? 'text-[56px] @max-[760px]:text-[34px]' : 'text-[72px] @max-[760px]:text-[44px]'}" title={artist.name}>{artist.name}</h1>

        <div class="flex flex-wrap items-center gap-x-3 gap-y-2 text-[15px] text-(--rg-mu)">
          {#if nbAlbums}
            <span><b class="font-bold text-(--rg-tx)">{nombre(nbAlbums)}</b> {plusieurs(nbAlbums, "library_head.albums_one", "library_head.albums_n")}</span>
            <span class="text-(--rg-mu2)">·</span>
          {/if}
          <span><b class="font-bold text-(--rg-tx)">{nombre(nbTitres)}</b> {plusieurs(nbTitres, "library_head.tracks_one", "library_head.tracks_n")}</span>
          {#if duree}
            <span class="text-(--rg-mu2)">·</span>
            <span>{dureeEcoute(duree)}</span>
          {/if}
          {#if periode}
            <span class="text-(--rg-mu2)">·</span>
            <span>{periode}</span>
          {/if}
        </div>

        <div class="flex flex-wrap items-center gap-2.5 mt-2">
          <PlayButton label={$t("artist_view.play_all")} disabled={!tracks.length} onclick={() => handleTracksPlay(ordreLecture)} />
          <RoundButton icon="material-symbols:shuffle-rounded" title={$t("album_view.shuffle")} disabled={!tracks.length} onclick={() => handleTracksPlay(tracks, true)} />
          <RoundButton icon={tousAimes ? "material-symbols:favorite-rounded" : "material-symbols:favorite-outline-rounded"} pressed={tousAimes} disabled={!tracks.length}
                       title={$t(tousAimes ? "album_view.fav_remove" : "album_view.fav_add")} onclick={basculerFavori} />
          <RoundButton icon="material-symbols:more-horiz" title={$t("album_view.more")}
                       onclick={(e) => { const r = (e.currentTarget as HTMLElement).getBoundingClientRect(); artistMenu = { x: r.left, y: r.bottom + 6 }; }} />
          {#if genres.length}
            <div class="flex flex-wrap gap-1.5 ml-1.5 @max-[760px]:ml-0">
              {#each genres as g (g)}<ChipLink href={`/library/${libraryId}/genres/${encodeURIComponent(g)}`} label={g} title={$t("album_view.genre_title")} />{/each}
            </div>
          {/if}
        </div>
      </div>
    </div>

    <!-- ─── Discographie (recréée quand l'ordre change : sinon le défilement suit la carte accrochée) ─── -->
    {#if albumsPropres.length > 0}
      <CarouselSection class="mt-3" titre={$t("artist_view.discography")}
                       sousTitre={`${nombre(albumsPropres.length)} ${plusieurs(albumsPropres.length, "library_head.albums_one", "library_head.albums_n")}`}
                       {grille} items={discographie} cle={(a) => a.id} carte={carteDisco} ligne={ligneDisco} reinit={discoParNom} />
    {/if}

    <!-- ─── Tous les titres ─── -->
    <section class="mt-12">
      <div class="flex flex-col gap-1 mb-2">
        <h2 class="text-[22px] font-extrabold tracking-[-0.01em] text-(--rg-tx)">{$t("artist_view.all_tracks")}</h2>
        <span class="text-[13px] text-(--rg-mu)">{$t(groupe ? "artist_view.grouped" : tri === "title" ? "artist_view.flat_title" : "artist_view.flat")}</span>
      </div>

      <div class="flex flex-wrap items-center gap-2.5 pt-1 pb-3.5">
        <span class="mr-auto text-[13px] text-(--rg-mu)"><b class="font-semibold text-(--rg-tx2)">{nombre(nbTitres)}</b> {plusieurs(nbTitres, "library_head.tracks_one", "library_head.tracks_n")}{#if duree}{" · "}{dureeEcoute(duree)}{/if}</span>

        <SegmentedControl variant="discret" label={$t("common.sort")} value={tri} onchange={trierPar}
                          options={[
                            { value: "year", label: $t("artist_view.sort_year"), after: flecheSens("year") },
                            { value: "album", label: $t("artist_view.sort_album"), after: flecheSens("album") },
                            { value: "title", label: $t("artist_view.sort_title"), after: flecheSens("title") },
                          ]} />
        {#if groupe && groupes.length > 1}
          <ToolbarButton icon={toutOuvert ? "material-symbols:unfold-less-rounded" : "material-symbols:unfold-more-rounded"}
                         label={$t(toutOuvert ? "artist_view.collapse_all" : "artist_view.expand_all")} onclick={toutBasculer} />
        {/if}
        {#if grille}
          <ToolbarButton icon="material-symbols:image-outline-rounded" label={$t("album_view.covers")} title={$t("album_view.covers_title")} pressed={jaquettes} onclick={basculerJaquettes} />
        {:else}
          <TrackColumnsButton {libraryId} avecLibelle />
        {/if}
        <SelectionToggle avecLibelle />
      </div>

      {#if !chargeTitres}
        <div class="flex justify-center py-10 text-(--rg-mu)"><Icon icon="material-symbols:progress-activity" width="22" class="animate-spin" /></div>
      {:else if tracks.length === 0}
        <p class="py-10 text-center text-sm text-(--rg-mu)">{$t("artist_view.no_tracks")}</p>
      {:else if grille}
        {#if groupe}
          {#each groupes as g, k (g.cle)}
            <div class={k > 0 ? (ouverts.has(groupes[k - 1].cle) ? "mt-7" : "mt-1") : "mt-2"}>
              {@render enteteGroupe(g.cle)}
              {#if ouverts.has(g.cle)}{@render grilleTitres(g.pistes, false)}{/if}
            </div>
          {/each}
        {:else}
          {@render grilleTitres(aPlat, true)}
        {/if}
      {:else}
        <LibraryTrackTable {libraryId} tracks={ordreLecture} columns={colonnes} {largeurs} barreHorizontale sortKey={groupe ? null : tri} sortDir={sens}
                           colle={groupe ? null : 60} positions={groupe}
                           groupes={groupe ? groupes.map((g) => ({ cle: g.cle, pistes: g.pistes, ouvert: ouverts.has(g.cle) })) : null}
                           {enteteGroupe}
                           onsort={(cle, dir) => { tri = cle; sens = dir; ouvrirPremier(); }} />
      {/if}
    </section>

    {#if participations.length > 0}
      <CarouselSection titre={$t("artist_view.appears_on")} sousTitre={$t("artist_view.appears_on_desc")}
                       {grille} items={participations} cle={(a) => a.id} carte={carteAlbum} ligne={ligneAlbum} />
    {/if}
    {#if similaires.length > 0}
      <CarouselSection titre={$t("album_view.similar_artists")}
                       sousTitre={genres[0] ? $t("album_view.same_genre").replace("{g}", genres[0]) : $t("artist_view.same_genre")}
                       {grille} items={similaires} cle={(a) => a.id} carte={carteArtiste} ligne={ligneArtiste} />
    {/if}
</DetailPage>

{#if artistMenu}
  <LibraryItemMenu kind="artist" id={artist.id} title={artist.name} {libraryId}
                   x={artistMenu.x} y={artistMenu.y} onclose={() => (artistMenu = null)} />
{/if}

{/if}
