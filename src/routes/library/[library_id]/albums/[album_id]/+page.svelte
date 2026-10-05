<script lang="ts">
// Page d'un album : en-tête teinté par la pochette, titres en grille ou en tableau, puis l'artiste et ses voisins.
import { page } from "$app/state";
import { goto } from "$app/navigation";
import Icon from "@iconify/svelte";
import { untrack } from "svelte";
import { invoke } from "@tauri-apps/api/core";
import { t, currentLocale } from "#lib/i18n";
import { profilSelector } from "#lib/stores/profil/profil.store";
import { selectionStore } from "#lib/stores/ui/selection.store";
import { viewMode } from "#lib/stores/ui/viewMode.store";
import { settingsStore } from "#lib/stores/settings/settings.store";
import { dataCache } from "#lib/stores/cache/dataCache.store";
import { liked } from "#lib/stores/playlist/like.store";
import { toasts } from "#lib/stores/ui/toast.store";
import { libraryContentStore } from "#lib/stores/library/libraryContent.store";
import { ordreAlbums } from "#lib/stores/library/albumOrder.store";
import { loadAlbum, loadTracksByAlbum } from "#lib/services/library/library.service";
import { handleTracksPlay } from "#lib/actions/queue/QueueAction";
import { clesAffichees, resoudreColonnes, lireLargeurs, resetTagCache, trierPistes, type SortDir } from "#lib/config/trackColumns";
import { dureeEcoute } from "#lib/helper/tools/dateTools";
import { tailleLisible } from "#lib/helper/tools/sizeTools";
import { meilleureQualite } from "#lib/helper/tools/audioFormatTools";
import { echantillonAleatoire } from "#lib/helper/tools/randomTools";
import { lireLocal, ecrireLocal } from "#lib/helper/tools/stockage";
import { cleTri, comparerNaturel, ordreDisque } from "#lib/helper/library/cleTri";
import { lireTagsFichier } from "#lib/helper/library/tagsFichier";
import CoverImg from "#lib/components/ui/image/CoverImg.svelte";
import ImgZoom from "#lib/components/ui/tools/ImgZoom.svelte";
import QualityBadge from "#lib/components/ui/text/QualityBadge.svelte";
import DetailPage from "#lib/components/ui/layout/DetailPage.svelte";
import PageState from "#lib/components/ui/layout/PageState.svelte";
import PlayButton from "#lib/components/ui/button/PlayButton.svelte";
import RoundButton from "#lib/components/ui/button/RoundButton.svelte";
import ToolbarButton from "#lib/components/ui/button/ToolbarButton.svelte";
import ChipLink from "#lib/components/ui/link/ChipLink.svelte";
import PathChip from "#lib/components/ui/link/PathChip.svelte";
import SegmentedControl from "#lib/components/ui/input/SegmentedControl.svelte";
import ViewModeSwitch from "#lib/components/ui/input/ViewModeSwitch.svelte";
import SelectionToggle from "#lib/components/ui/selection/SelectionToggle.svelte";
import CarouselSection from "#lib/components/ui/carousel/CarouselSection.svelte";
import CollectionContextMenu from "#lib/components/ui/contextmenu/CollectionContextMenu.svelte";
import AlbumTrackRow from "#lib/components/library/album/AlbumTrackRow.svelte";
import LibraryTrackTable from "#lib/components/library/track/LibraryTrackTable.svelte";
import TrackColumnsButton from "#lib/components/library/track/TrackColumnsButton.svelte";
import AlbumListItem from "#lib/components/library/album/AlbumListItem.svelte";
import AlbumListRow from "#lib/components/library/album/AlbumListRow.svelte";
import ArtistListItem from "#lib/components/library/artist/ArtistListItem.svelte";
import ArtistListRow from "#lib/components/library/artist/ArtistListRow.svelte";
import type { Library } from "#lib/types/db/library/Library";
import type { AlbumDetailView } from "#lib/types/ui/library/album/AlbumDetailView";
import type { AlbumListView } from "#lib/types/ui/library/album/AlbumListView";
import type { ArtistListView } from "#lib/types/ui/library/artist/ArtistListView";
import type { TrackListView } from "#lib/types/ui/library/track/TrackListView";

const libraryId = $derived(Number(page.params.library_id));
const albumId = $derived(page.params.album_id as string);
const profil = $derived($profilSelector.profilSelected);

let album = $state<AlbumDetailView | null>(null);
let tracks = $state<TrackListView[]>([]);
let isLoading = $state(true);
let erreur = $state(false);
let defilement = $state<HTMLDivElement | null>(null);

// ─── Chargement : album et titres ensemble, l'ancien reste affiché jusque-là ───
let tour = 0;

$effect(() => {
  const id = libraryId;
  const aid = albumId;
  const p = profil;
  if (!$profilSelector.initialized) return;
  if (!p || !id || !aid) {
    erreur = true;
    isLoading = false;
    return;
  }
  // Hors suivi : `charger` lit `album`, l'effet se relancerait à chaque chargement.
  const n = ++tour;
  untrack(() => charger(id, aid, p.id, n));
});

async function charger(libId: number, aid: string, profilId: number, n: number) {
  erreur = false;
  isLoading = album === null;
  try {
    const lib = await invoke<Library>("get_library", { libraryId: libId });
    if (n !== tour) return;
    if (!lib || lib.profil_id !== profilId) {
      goto("/");
      return;
    }
    // Périmées, les fiches s'affichent tout de suite puis se rechargent.
    const [a, pistes] = await Promise.all([
      dataCache.lire(`album:${aid}`, () => loadAlbum(aid), (v) => { if (n === tour) album = v; }),
      dataCache.lire(`album-tracks:${aid}`, () => loadTracksByAlbum(libId, aid), (v) => { if (n === tour) tracks = v; }),
    ]);
    if (n !== tour) return;
    if (!a) {
      erreur = true;
      return;
    }
    const autre = album?.id !== a.id;
    album = a;
    tracks = pistes ?? [];
    if (autre) {
      tri = null;
      sens = "asc";
      defilement?.scrollTo({ top: 0 });
    }
  } catch (e) {
    if (n === tour) erreur = true;
    console.error("[album] chargement :", e);
  } finally {
    if (n === tour) isLoading = false;
  }
}

async function pistesPourMenu() {
  return album ? ((await loadTracksByAlbum(libraryId, album.id)) ?? []) : [];
}

// Les tags analysés appartiennent aux pistes chargées : un autre album vide le cache.
$effect(() => {
  void tracks;
  resetTagCache();
});

// ─── Chiffres de l'album ───
const nombre = (n: number) => n.toLocaleString($currentLocale);
const duree = $derived(tracks.reduce((s, x) => s + (x.duration ?? 0), 0) || album?.total_duration || 0);
const poids = $derived(tracks.reduce((s, x) => s + (x.file_size ?? x.size ?? 0), 0));
const nbTitres = $derived(tracks.length || album?.total_tracks || 0);
const titresLibelle = $derived(`${nombre(nbTitres)} ${$t(nbTitres === 1 ? "library_head.tracks_one" : "library_head.tracks_n")}`);
const qualite = $derived(meilleureQualite(tracks));

// Genres des titres (l'album n'en porte qu'un), chacun mène à sa page.
const genres = $derived.by(() => {
  const vus = new Set<string>();
  for (const x of tracks) if (x.genre?.trim()) vus.add(x.genre.trim());
  if (vus.size === 0 && album?.genre) vus.add(album.genre);
  return [...vus].slice(0, 4);
});

// Date de sortie complète et label, lus dans les tags de la première piste qui les porte.
const fiche = $derived.by(() => {
  let date: string | null = null;
  let label: string | null = null;
  for (const x of tracks) {
    const tg = lireTagsFichier(x.tags);
    if (!tg) continue;
    const { champs, perso } = tg;
    label ??= (champs.publisher as string) || perso.get("LABEL") || perso.get("ORGANIZATION") || perso.get("PUBLISHER") || null;
    date ??= [perso.get("RELEASEDATE"), perso.get("DATE"), champs.year as string, perso.get("ORIGINALDATE")].find((v) => v && /^\d{4}-\d{2}-\d{2}/.test(v)) ?? null;
    if (label && date) break;
  }
  return { date, label };
});
const sortie = $derived.by(() => {
  if (fiche.date) {
    const d = new Date(fiche.date.slice(0, 10) + "T12:00:00");
    if (!isNaN(d.getTime())) return $t("album_view.released_on").replace("{d}", d.toLocaleDateString($currentLocale, { day: "numeric", month: "long", year: "numeric" }));
  }
  return album?.year ? $t("album_view.released_in").replace("{d}", String(album.year)) : null;
});

// Dossier commun des fichiers (un album en CD1/CD2 remonte d'un cran).
const dossier = $derived.by(() => {
  const parent = (p: string) => p.replace(/[\\/][^\\/]*$/, "");
  const dirs = tracks.map((x) => parent(x.path));
  if (!dirs.length) return null;
  let p = dirs[0];
  for (const d of dirs) {
    while (p && d !== p && !d.startsWith(p + "\\") && !d.startsWith(p + "/")) {
      const q = parent(p);
      p = q === p ? "" : q;
    }
  }
  return p || null;
});

// ─── Favoris : l'album l'est quand tous ses titres le sont ───
const tousAimes = $derived(tracks.length > 0 && tracks.every((x) => $liked.paths.has(x.path)));
async function basculerFavori() {
  const aimer = !tousAimes;
  try {
    await liked.toggleAll(tracks.map((x) => x.path), aimer);
    toasts.push({ type: "success", title: $t(aimer ? "album_view.fav_added" : "album_view.fav_removed"), message: album?.title ?? "" });
  } catch (e) {
    console.error("[album] favoris :", e);
  }
}

// ─── Tri des titres : un seul état, partagé par le sélecteur et l'en-tête du tableau ───
let tri = $state<string | null>(null);
let sens = $state<SortDir>("asc");
// Sans numéro dans les tags, l'ordre naturel des noms de fichier (« 2-13 » avant « 10-13 »).
const pistesAlbum = $derived(tracks.every((x) => x.track_number != null) ? tracks : [...tracks].sort(ordreDisque));
const pistesTriees = $derived(trierPistes(pistesAlbum, tri, sens));

function trierPar(cle: string) {
  if (cle === "n") {
    tri = null;
    sens = "asc";
  } else if (tri === cle) sens = sens === "asc" ? "desc" : "asc";
  else {
    tri = cle;
    sens = "asc";
  }
}
const flecheSens = (cle: string) => (tri === cle ? (sens === "asc" ? "material-symbols:arrow-upward-rounded" : "material-symbols:arrow-downward-rounded") : null);

// Dans l'ordre du disque, un album en plusieurs disques se lit disque par disque.
const disques = $derived.by(() => {
  if (tri !== null) return [{ n: 0, pistes: pistesTriees }];
  const m = new Map<number, TrackListView[]>();
  for (const x of pistesTriees) {
    const d = x.disc_number || 1;
    if (!m.has(d)) m.set(d, []);
    m.get(d)!.push(x);
  }
  return m.size > 1 ? [...m].map(([n, pistes]) => ({ n, pistes })) : [{ n: 0, pistes: pistesTriees }];
});

// L'ordre affiché, pour Maj + clic (le tableau déclare le sien).
$effect(() => {
  if ($viewMode !== "list") selectionStore.setOrder(pistesTriees.map((x) => ({ id: x.id, track: x })));
});

// Jaquettes de la vue grille, retenues d'une visite à l'autre.
let jaquettes = $state(lireLocal("album:jaquettes", "1") !== "0");
function basculerJaquettes() {
  jaquettes = !jaquettes;
  ecrireLocal("album:jaquettes", jaquettes ? "1" : "0");
}

// Colonnes du tableau : les mêmes que l'onglet Morceaux.
const colonnes = $derived(resoudreColonnes(clesAffichees($settingsStore.track_columns)).filter((c) => c.key !== "artist"));
const largeurs = $derived(lireLargeurs($settingsStore.track_column_widths));

// ─── Autour de l'album ───
const tousAlbums = $derived($libraryContentStore.albums.filter((a) => a.library_id === libraryId));
const tousArtistes = $derived($libraryContentStore.artists);
const artiste = $derived(album?.artist_id ? tousArtistes.find((a) => a.id === album!.artist_id) ?? null : null);
const parTitre = (x: AlbumListView, y: AlbumListView) => comparerNaturel.compare(cleTri(x.title), cleTri(y.title));

let triAutres = $state<"year" | "title">("year");
const autresAlbums = $derived.by(() => {
  const a = album;
  if (!a?.artist_id) return [];
  const liste = tousAlbums.filter((x) => x.artist_id === a.artist_id && x.id !== a.id);
  return triAutres === "year" ? liste.sort((x, y) => (y.year ?? 0) - (x.year ?? 0) || parTitre(x, y)) : liste.sort(parTitre);
});

// Dix au hasard, tirés une fois par album : deux albums du même genre ne proposent pas la même sélection.
let similaires = $state<AlbumListView[]>([]);
let artistesSimilaires = $state<ArtistListView[]>([]);
let dernierTirage = "";
$effect(() => {
  const a = album;
  const als = tousAlbums;
  const ars = tousArtistes;
  const cle = `${a?.id}:${als.length}:${ars.length}`;
  if (!a || cle === dernierTirage) return;
  dernierTirage = cle;
  const genre = a.genre?.toLowerCase();
  if (!genre) {
    similaires = [];
    artistesSimilaires = [];
    return;
  }
  const memeGenre = als.filter((x) => x.genre?.toLowerCase() === genre && x.artist_id !== a.artist_id);
  similaires = echantillonAleatoire(memeGenre, 10);
  const ids = new Set(memeGenre.map((x) => x.artist_id).filter(Boolean));
  artistesSimilaires = echantillonAleatoire(ars.filter((x) => ids.has(x.id)), 10);
});

// ─── Précédent / suivant : l'ordre de la vue Albums, sinon l'ordre alphabétique ───
const ordre = $derived.by(() => {
  const o = $ordreAlbums;
  if (o && o.libraryId === libraryId && album && o.ids.includes(album.id)) return o.ids;
  return [...tousAlbums].sort(parTitre).map((x) => x.id);
});
const rang = $derived(album ? ordre.indexOf(album.id) : -1);
const precedent = $derived(rang > 0 ? ordre[rang - 1] : null);
const suivant = $derived(rang >= 0 && rang < ordre.length - 1 ? ordre[rang + 1] : null);

let albumMenu = $state<{ x: number; y: number } | null>(null);
const grille = $derived($viewMode !== "list");
</script>

<!-- Cartes et lignes des sections (déclarées hors de DetailPage : sinon ce seraient ses props). -->
{#snippet carteAlbum(a: AlbumListView)}<AlbumListItem {libraryId} album={a} />{/snippet}
{#snippet ligneAlbum(a: AlbumListView)}<AlbumListRow {libraryId} album={a} />{/snippet}
{#snippet carteArtiste(a: ArtistListView)}<ArtistListItem {libraryId} artist={a} />{/snippet}
{#snippet ligneArtiste(a: ArtistListView)}<ArtistListRow {libraryId} artist={a} />{/snippet}
{#snippet triAutresSeg()}
  <SegmentedControl variant="discret" label={$t("album_view.sort_year")} value={triAutres} onchange={(v) => (triAutres = v as "year" | "title")}
                    options={[{ value: "year", label: $t("album_view.sort_year") }, { value: "title", label: $t("album_view.sort_title") }]} />
{/snippet}

{#if isLoading && !album}
  <PageState chargement message={$t("album_view.loading")} />

{:else if erreur && !album}
  <PageState icon="material-symbols:album-outline" message={$t("album_view.error")} lien={{ href: `/library/${libraryId}/albums`, label: $t("album_view.back") }} />

{:else if album}
<DetailPage bind:defilement image={album.cover_url} retourHref={`/library/${libraryId}/albums`} retourLabel={$t("album_view.back")}
            playLabel={$t("album_view.play")} onplay={() => handleTracksPlay(pistesAlbum)}>
  {#snippet mini()}
    <span class="w-8.5 h-8.5 shrink-0 rounded-[7px] overflow-hidden bg-(--rg-s2)">
      {#if album?.cover_url}<CoverImg path={album.cover_url} alt="" size="1x" class="w-full h-full object-cover" />{/if}
    </span>
    <b class="truncate text-[15px] text-(--rg-tx)">{album?.title}</b>
    <span class="shrink-0 text-[13px] text-(--rg-mu) max-sm:hidden">{album?.artist ?? ""}</span>
  {/snippet}
  {#snippet barre()}
    <ViewModeSwitch class="bg-(--rg-carte)/80" />
    <div class="flex gap-0.5">
      {#each [{ id: precedent, i: "material-symbols:chevron-left-rounded", l: "album_view.prev" }, { id: suivant, i: "material-symbols:chevron-right-rounded", l: "album_view.next" }] as o (o.l)}
        <button type="button" disabled={!o.id} title={$t(o.l)} aria-label={$t(o.l)}
                class="w-8 h-8 flex items-center justify-center rounded-lg cursor-pointer transition-colors text-(--rg-mu) hover:bg-(--rg-s2) hover:text-(--rg-tx) disabled:opacity-30 disabled:cursor-default disabled:hover:bg-transparent"
                onclick={() => o.id && goto(`/library/${libraryId}/albums/${o.id}`)}>
          <Icon icon={o.i} width="22" />
        </button>
      {/each}
    </div>
  {/snippet}

  <!-- ─── En-tête ─── -->
  <div class="relative flex flex-wrap items-end gap-8 pt-3 pb-7">
    <div class="relative w-58 @max-[760px]:w-45 aspect-square shrink-0 rounded-[14px] overflow-hidden shadow-[0_24px_60px_rgba(0,0,0,0.3)] dark:shadow-[0_24px_60px_rgba(0,0,0,0.55)]">
      {#if album.cover_url}
        <ImgZoom path={album.cover_url} alt={album.title}>
          <CoverImg path={album.cover_url} alt={album.title} class="w-full h-full object-cover" />
        </ImgZoom>
      {:else}
        <div class="w-full h-full flex items-center justify-center text-(--rg-mu2) shadow-[inset_0_0_0_1px_var(--rg-bd)]
                    bg-[repeating-linear-gradient(135deg,var(--rg-s2)_0_10px,var(--rg-carte)_10px_20px)]">
          <Icon icon="material-symbols:album-outline" width="64" />
        </div>
      {/if}
      <span class="absolute inset-0 rounded-[14px] ring-1 ring-inset ring-black/5 dark:ring-white/6 pointer-events-none"></span>
    </div>

    <div class="flex-[1_1_320px] min-w-0 flex flex-col gap-3">
      <div class="flex flex-wrap items-center gap-2.5 text-xs font-bold tracking-widest uppercase text-(--rg-tx2)">
        {$t(`album_view.type_${["single", "ep", "compilation"].includes(album.album_type) ? album.album_type : "album"}`)}
        {#if qualite}<QualityBadge palier={qualite.palier} texte={qualite.texte} class="px-2 py-0.75" />{/if}
      </div>

      <h1 class="font-extrabold leading-none tracking-[-0.03em] text-balance line-clamp-3 text-(--rg-tx)
                 {album.title.length > 40 ? 'text-[44px] @max-[760px]:text-[30px]' : 'text-[56px] @max-[760px]:text-[38px]'}" title={album.title}>{album.title}</h1>

      <div class="flex flex-wrap items-center gap-x-3 gap-y-2 text-[15px] text-(--rg-mu)">
        {#if album.artist_id}
          <a href={`/library/${libraryId}/artists/${album.artist_id}`} class="flex items-center gap-2 min-w-0 font-bold text-(--rg-tx) hover:underline underline-offset-2">
            <span class="w-6.5 h-6.5 shrink-0 rounded-full overflow-hidden flex items-center justify-center bg-(--rg-s2) text-(--rg-mu)">
              {#if artiste?.thumbnail_path}
                <CoverImg path={artiste.thumbnail_path} alt="" size="1x" class="w-full h-full object-cover" />
              {:else}
                <Icon icon="material-symbols:person-rounded" width="16" />
              {/if}
            </span>
            <span class="truncate">{album.artist ?? $t("album_view.unknown_artist")}</span>
          </a>
        {:else}
          <span class="font-bold text-(--rg-tx)">{album.artist ?? $t("album_view.unknown_artist")}</span>
        {/if}
        {#if album.year}
          <span class="text-(--rg-mu2)">·</span>
          <a href={`/library/${libraryId}/albums?annee=${album.year}`} class="hover:text-(--rg-tx) hover:underline underline-offset-2"
             title={$t("album_view.year_title").replace("{y}", String(album.year))}>{album.year}</a>
        {/if}
        <span class="text-(--rg-mu2)">·</span>
        <span>{titresLibelle}</span>
        {#if duree}<span class="text-(--rg-mu2)">·</span><span>{dureeEcoute(duree)}</span>{/if}
        {#if poids}<span class="text-(--rg-mu2)">·</span><span>{tailleLisible(poids, $currentLocale)}</span>{/if}
      </div>

      <div class="flex flex-wrap items-center gap-2.5 mt-2">
        <PlayButton label={$t("album_view.play")} onclick={() => handleTracksPlay(pistesAlbum)} />
        <RoundButton icon="material-symbols:shuffle-rounded" title={$t("album_view.shuffle")} onclick={() => handleTracksPlay(pistesAlbum, true)} />
        <RoundButton icon={tousAimes ? "material-symbols:favorite-rounded" : "material-symbols:favorite-outline-rounded"} pressed={tousAimes}
                     title={$t(tousAimes ? "album_view.fav_remove" : "album_view.fav_add")} onclick={basculerFavori} />
        <RoundButton icon="material-symbols:more-horiz" title={$t("album_view.more")}
                     onclick={(e) => { const r = (e.currentTarget as HTMLElement).getBoundingClientRect(); albumMenu = { x: r.left, y: r.bottom + 6 }; }} />
        {#if genres.length}
          <div class="flex flex-wrap gap-1.5 ml-1.5 @max-[760px]:ml-0">
            {#each genres as g (g)}<ChipLink href={`/library/${libraryId}/genres/${encodeURIComponent(g)}`} label={g} title={$t("album_view.genre_title")} />{/each}
          </div>
        {/if}
      </div>
    </div>
  </div>

  <!-- ─── Titres ─── -->
  <div class="flex flex-wrap items-center gap-2.5 pt-1 pb-3.5">
    <span class="mr-auto text-[13px] text-(--rg-mu)"><b class="font-semibold text-(--rg-tx2)">{nombre(nbTitres)}</b> {$t(nbTitres === 1 ? "library_head.tracks_one" : "library_head.tracks_n")}{#if duree}{" · "}{dureeEcoute(duree)}{/if}</span>

    {#if tracks.length > 1}
      <SegmentedControl variant="discret" label={$t("common.sort")} value={tri ?? "n"} onchange={trierPar}
                        options={[
                          { value: "n", label: $t("album_view.sort_number") },
                          { value: "title", label: $t("album_view.sort_title"), after: flecheSens("title") },
                          { value: "duration", label: $t("album_view.sort_duration"), after: flecheSens("duration") },
                        ]} />
    {/if}
    {#if grille}
      <ToolbarButton icon="material-symbols:image-outline-rounded" label={$t("album_view.covers")} title={$t("album_view.covers_title")} pressed={jaquettes} onclick={basculerJaquettes} />
    {:else}
      <TrackColumnsButton {libraryId} avecLibelle />
    {/if}
    <SelectionToggle avecLibelle />
  </div>

  {#if tracks.length === 0}
    <p class="py-10 text-center text-sm text-(--rg-mu)">{$t("album_view.no_tracks")}</p>
  {:else if grille}
    {#each disques as d (d.n)}
      {#if d.n}
        <p class="flex items-center gap-2 pt-4 pb-2 px-3 text-xs font-bold uppercase tracking-[0.08em] text-(--rg-mu2) first:pt-0">
          <Icon icon="material-symbols:album-outline" width="16" />{$t("album_view.disc").replace("{n}", String(d.n))}
        </p>
      {/if}
      <!-- Deux colonnes lues de haut en bas, une seule quand la place manque. -->
      <div class="grid grid-cols-2 grid-flow-col gap-x-7 gap-y-0.5 grid-rows-[repeat(var(--rangs),auto)]
                  @max-[900px]:grid-cols-1 @max-[900px]:grid-flow-row @max-[900px]:grid-rows-none"
           style="--rangs: {Math.ceil(d.pistes.length / 2)}">
        {#each d.pistes as track, i (track.id)}
          <AlbumTrackRow {libraryId} {track} tracks={pistesTriees} jaquette={jaquettes} artisteAlbum={album.artist} position={tri === null ? i + 1 : null} />
        {/each}
      </div>
    {/each}
  {:else}
    <LibraryTrackTable {libraryId} tracks={pistesTriees} columns={colonnes} {largeurs} barreHorizontale sortKey={tri} sortDir={sens} colle={60} positions={tri === null}
                       onsort={(cle, dir) => { tri = cle; sens = dir; }} />
  {/if}

  <!-- ─── Fiche : sortie, label, dossier ─── -->
  {#if sortie || fiche.label || dossier}
    <div class="flex flex-wrap items-center gap-x-6 gap-y-2 pt-5 px-3 text-[13px] text-(--rg-mu)">
      {#if sortie}<span>{sortie}</span>{/if}
      {#if fiche.label}<span>{$t("album_view.label")} <b class="font-semibold text-(--rg-tx2)">{fiche.label}</b></span>{/if}
      {#if dossier}<PathChip chemin={dossier} aReveler={tracks[0]?.path ?? dossier} />{/if}
    </div>
  {/if}

  {#if album.notes}
    <section class="mt-12">
      <h2 class="mb-2 text-[22px] font-extrabold tracking-[-0.01em] text-(--rg-tx)">{$t("album_view.about")}</h2>
      <p class="max-w-3xl text-sm leading-relaxed text-(--rg-mu)">{album.notes}</p>
    </section>
  {/if}

  {#if autresAlbums.length > 0}
    <CarouselSection titre={album.artist ? $t("album_view.other_albums").replace("{a}", album.artist) : $t("album_view.other_albums_same")}
                     sousTitre={`${nombre(autresAlbums.length)} ${$t(autresAlbums.length === 1 ? "library_head.albums_one" : "library_head.albums_n")}`}
                     {grille} items={autresAlbums} cle={(a) => a.id} carte={carteAlbum} ligne={ligneAlbum} extra={triAutresSeg} reinit={triAutres} />
  {/if}
  {#if similaires.length > 0}
    <CarouselSection titre={$t("album_view.similar_albums")} sousTitre={$t("album_view.same_genre").replace("{g}", album.genre ?? "")}
                     {grille} items={similaires} cle={(a) => a.id} carte={carteAlbum} ligne={ligneAlbum} />
  {/if}
  {#if artistesSimilaires.length > 0}
    <CarouselSection titre={$t("album_view.similar_artists")} sousTitre={$t("album_view.same_genre").replace("{g}", album.genre ?? "")}
                     {grille} items={artistesSimilaires} cle={(a) => a.id} carte={carteArtiste} ligne={ligneArtiste} />
  {/if}
</DetailPage>

{#if albumMenu}
  <CollectionContextMenu
    title={album.title}
    type="album"
    loadTracks={pistesPourMenu}
    x={albumMenu.x}
    y={albumMenu.y}
    onclose={() => (albumMenu = null)}
    onedittags={() => goto(`/library/${libraryId}/tags?album=${album?.id}`)}
    oncover={() => { if (album) loadAlbum(album.id).then((r) => { if (r) { album = r; dataCache.set(`album:${r.id}`, r); } }); }}
    albumId={album.id}
    artistName={album.artist}
    {libraryId}
  />
{/if}
{/if}
