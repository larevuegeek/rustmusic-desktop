<script lang="ts">
import EmptyResult from "#lib/components/ui/feedback/EmptyResult.svelte";
import SearchField from "#lib/components/ui/input/SearchField.svelte";
import TextButton from "#lib/components/ui/button/TextButton.svelte";
import MenuSelect from "#lib/components/ui/menu/MenuSelect.svelte";
import SelectionToggle from "#lib/components/ui/selection/SelectionToggle.svelte";
import SegmentedControl from "#lib/components/ui/input/SegmentedControl.svelte";
import TrackColumnsButton from "#lib/components/library/track/TrackColumnsButton.svelte";
import { page } from "$app/state";
import { onDestroy, tick } from "svelte";
import Icon from "@iconify/svelte";
import { invoke } from "@tauri-apps/api/core";
import { t, currentLocale } from "#lib/i18n";
import { libraryHeader } from "#lib/stores/library/libraryHeader";
import { settingsStore } from "#lib/stores/settings/settings.store";
import { handlePlayTrack } from "#lib/actions/player/PlayerAction";
import { versFileDAttente } from "#lib/mapper/queue/mapQueueTrack";
import { clesAffichees, resoudreColonnes, lireLargeurs, resetTagCache, type SortDir } from "#lib/config/trackColumns";
import LibraryImportingLoader from "#lib/components/library/common/loader/LibraryImportingLoader.svelte";
import { importAffiche, importsTermines } from "#lib/stores/library/importProgress.store";
import LibraryTrackTable from "#lib/components/library/track/LibraryTrackTable.svelte";
import FilterChip from "#lib/components/ui/input/FilterChip.svelte";
import Menu from "#lib/components/ui/menu/Menu.svelte";
import MenuItem from "#lib/components/ui/menu/MenuItem.svelte";
import AlphabetNav from "#lib/components/ui/alphabet/AlphabetNav.svelte";
import type { TrackListView } from "#lib/types/ui/library/track/TrackListView";
import type { GenreView } from "#lib/types/ui/library/genre/GenreView";

const libraryId = $derived(Number(page.params.library_id));

// ─── Préférences retenues : tri et densité ───
type Tri = "title" | "artist" | "album" | "rating" | "duration" | "date";
const TRIS: { cle: Tri; libelle: string; icone: string }[] = [
  { cle: "title", libelle: "tracks_view.sort_title", icone: "material-symbols:sort-by-alpha-rounded" },
  { cle: "artist", libelle: "tracks_view.sort_artist", icone: "material-symbols:person-outline-rounded" },
  { cle: "album", libelle: "tracks_view.sort_album", icone: "material-symbols:album-outline-rounded" },
  { cle: "rating", libelle: "tracks_view.sort_rating", icone: "material-symbols:star-outline-rounded" },
  { cle: "duration", libelle: "tracks_view.sort_duration", icone: "material-symbols:schedule-outline-rounded" },
  { cle: "date", libelle: "tracks_view.sort_added", icone: "material-symbols:calendar-month-outline-rounded" },
];
function lire<T>(cle: string, defaut: T): T {
  try { return (JSON.parse(localStorage.getItem(cle) ?? "null") as T) ?? defaut; } catch { return defaut; }
}
const pref = lire<{ tri?: string; sens?: SortDir }>("morceaux:tri", {});
// Le tri s'applique aussi aux colonnes non listées (tags) : on garde la clé telle quelle.
let tri = $state<string>(pref.tri ?? "title");
let sens = $state<SortDir>(pref.sens ?? "asc");
let detaille = $state(lire<string>("morceaux:densite", "compact") === "detaille");
$effect(() => {
  try {
    localStorage.setItem("morceaux:tri", JSON.stringify({ tri, sens }));
    localStorage.setItem("morceaux:densite", JSON.stringify(detaille ? "detaille" : "compact"));
  } catch {}
});

// ─── Filtres (appliqués par la base : la liste arrive page par page) ───
let recherche = $state("");
let rechercheEnvoyee = $state("");
let minuterie: ReturnType<typeof setTimeout> | null = null;
$effect(() => {
  const q = recherche.trim();
  if (minuterie) clearTimeout(minuterie);
  minuterie = setTimeout(() => (rechercheEnvoyee = q.length >= 2 ? q : ""), 300);
});
let hiRes = $state(false);
let sansPerte = $state(false);
let favoris = $state(false);
let sansPochette = $state(false);
let genre = $state<string | null>(null);

const filtresActifs = $derived(!!recherche.trim() || hiRes || sansPerte || favoris || sansPochette || !!genre);
function effacerFiltres() {
  recherche = "";
  hiRes = sansPerte = favoris = sansPochette = false;
  genre = null;
}

let menuGenre = $state(false);

// Genres de la bibliothèque, chargés à la première ouverture du menu.
let genres = $state<GenreView[] | null>(null);
async function ouvrirGenres() {
  menuGenre = !menuGenre;
  if (genres === null) {
    try {
      genres = (await invoke<GenreView[]>("get_genres", { libraryId })).sort((a, b) => b.total_tracks - a.total_tracks);
    } catch {
      genres = [];
    }
  }
}

// ─── Colonnes : celles de l'utilisateur, habillées pour la maquette ───
const cles = $derived(clesAffichees($settingsStore.track_columns));
// L'artiste passe sous le titre ; en détaillé, album, année et qualité passent sur la ligne d'infos.
const DANS_LE_DETAIL = new Set(["album", "quality", "year", "audio_format", "bits_per_sample", "sample_rate"]);
const colonnes = $derived(resoudreColonnes(cles).filter((c) => c.key !== "artist" && !(detaille && DANS_LE_DETAIL.has(c.key))));
const largeurs = $derived(lireLargeurs($settingsStore.track_column_widths));

// ─── Chargement page par page ───
const PAGE = 100;
// La liste chargée est une fenêtre [debut, debut + tracks.length) : la navigation A–Z la déplace.
let tracks = $state<TrackListView[]>([]);
let debut = $state(0);
let total = $state(0);
let chargement = $state(true);
let suite = $state(false);
let defilement = $state<HTMLDivElement | null>(null);
let jeton = 0;

function filtres() {
  return {
    libraryId,
    sortDir: sens,
    filter: rechercheEnvoyee || null,
    missingCover: sansPochette,
    quality: hiRes ? "hires" : sansPerte ? "lossless" : null,
    favorites: favoris,
    genre,
  };
}
function parametres(offset: number, limit: number) {
  return { ...filtres(), offset, limit, sortBy: tri };
}

async function lirePage(offset: number, limit: number) {
  return invoke<{ tracks: TrackListView[]; total: number }>("get_tracks_paginated", parametres(offset, limit));
}

/** Recharge une fenêtre qui commence à `depart` ; `ancre` = index absolu à amener en haut. */
async function charger(depart = 0, ancre: number | null = null) {
  const moi = ++jeton;
  chargement = true;
  try {
    const r = await lirePage(depart, PAGE);
    if (moi !== jeton) return;
    resetTagCache();
    tracks = r.tracks;
    debut = depart;
    total = r.total;
    await tick();
    if (ancre === null) defilement?.scrollTo({ top: 0 });
    else montrer(ancre);
  } catch (e) {
    console.error("[morceaux] chargement :", e);
  } finally {
    if (moi === jeton) chargement = false;
  }
}

// Au-delà, on lâche l'autre bout : le document reste léger après des milliers de titres.
const FENETRE = 500;

/** Applique `maj` sans que la première ligne visible ne bouge à l'écran. */
async function sansSaut(maj: () => void) {
  const el = defilement;
  const haut = el?.getBoundingClientRect().top ?? 0;
  const repere = el ? [...el.querySelectorAll<HTMLElement>("[data-piste]")].find((r) => r.getBoundingClientRect().bottom > haut) : undefined;
  const avant = repere?.getBoundingClientRect().top ?? 0;
  maj();
  await tick();
  if (el && repere?.isConnected) el.scrollTop += repere.getBoundingClientRect().top - avant;
}

async function chargerApres() {
  const moi = jeton;
  suite = true;
  try {
    const r = await lirePage(debut + tracks.length, PAGE);
    if (moi !== jeton) return;
    const tout = [...tracks, ...r.tracks];
    const trop = Math.max(0, tout.length - FENETRE);
    await sansSaut(() => { tracks = tout.slice(trop); debut += trop; });
  } catch (e) {
    console.error("[morceaux] suite :", e);
  } finally {
    suite = false;
  }
}

// Au-dessus de la fenêtre : on insère sans faire sauter ce qu'on regarde.
async function chargerAvant() {
  const moi = jeton;
  const depart = Math.max(0, debut - PAGE);
  suite = true;
  try {
    const r = await lirePage(depart, debut - depart);
    if (moi !== jeton) return;
    await sansSaut(() => { tracks = [...r.tracks, ...tracks].slice(0, FENETRE); debut = depart; });
  } catch (e) {
    console.error("[morceaux] précédents :", e);
  } finally {
    suite = false;
  }
}

function defiler(e: Event) {
  const el = e.currentTarget as HTMLDivElement;
  if (suite || chargement) return;
  if (debut + tracks.length < total && el.scrollHeight - el.scrollTop - el.clientHeight < 400) chargerApres();
  else if (debut > 0 && el.scrollTop < 400) chargerAvant();
}

// ─── Navigation A–Z : la base dit où commence chaque lettre dans le tri en cours ───
let lettres = $state<Map<string, number>>(new Map());
let jetonLettres = 0;
async function chargerLettres() {
  // Filtres changés pendant la requête : ces positions ne valent plus.
  const moi = ++jetonLettres;
  if (tri !== "title") { lettres = new Map(); return; }
  try {
    const l = await invoke<{ letter: string; offset: number }[]>("get_track_letter_offsets", filtres());
    if (moi === jetonLettres) lettres = new Map(l.map((x) => [x.letter, x.offset]));
  } catch {
    if (moi === jetonLettres) lettres = new Map();
  }
}

/** Amène la ligne d'index absolu `i` en haut de la zone (sous l'en-tête collant). */
function montrer(i: number) {
  const piste = tracks[i - debut];
  if (piste) defilement?.querySelector(`[data-piste="${CSS.escape(piste.id)}"]`)?.scrollIntoView({ block: "start" });
}

function allerLettre(l: string) {
  const i = lettres.get(l);
  if (i === undefined) return;
  if (i >= debut && i < debut + tracks.length) montrer(i);
  // Un peu de contexte au-dessus : remonter d'une ligne recharge la suite sans à-coup.
  else charger(Math.max(0, i - 20), i);
}

// Tout changement de bibliothèque, de tri, de filtre ou un import validé repart du début.
$effect(() => {
  void [libraryId, tri, sens, rechercheEnvoyee, hiRes, sansPerte, favoris, sansPochette, genre, $importsTermines];
  charger(0);
  chargerLettres();
});

function trierPar(cle: string, dir: SortDir) {
  tri = cle;
  sens = dir;
}

// « Tout lire » (en-tête) : la liste telle qu'affichée, filtres et tri compris, plafonnée
// pour ne pas charger la file et ses panneaux de dizaines de milliers de lignes.
const PLAFOND_LECTURE = 1000;
async function toutLire() {
  try {
    const r = await invoke<{ tracks: TrackListView[] }>("get_tracks_paginated", parametres(0, PLAFOND_LECTURE));
    if (r.tracks.length) await handlePlayTrack(r.tracks[0].path, versFileDAttente(r.tracks));
  } catch (e) {
    console.error("[morceaux] tout lire :", e);
  }
}

// Dans un effet (après le montage) : la page quittée a déjà remis l'en-tête à zéro.
$effect(() => {
  libraryHeader.update(() => ({ action: { cle: "library_head.play_all", icone: "material-symbols:play-arrow-rounded", lancer: toutLire } }));
});
onDestroy(() => libraryHeader.update((h) => ({ ...h, action: null })));

const nombre = (n: number) => n.toLocaleString($currentLocale);
</script>

{#if $importAffiche}
  <LibraryImportingLoader />

{:else}
  <div class="flex flex-col h-full">
    <!-- ─── Outils : une ligne si la place le permet ; sinon recherche + vue en haut, filtres dessous ─── -->
    <div class="@container shrink-0 pl-4 md:pl-8 pr-4 md:pr-14 py-3">
      <div class="flex flex-wrap items-center gap-2.5">
        <SearchField bind:value={recherche} class="order-1 w-65 @max-6xl:w-auto @max-6xl:flex-1 min-w-36" placeholder={$t("tracks_view.search")} clearLabel={$t("tracks_view.clear_filters")} />

        <div class="order-2 @max-6xl:order-4 @max-6xl:basis-full flex flex-wrap items-center gap-2.5">
          <!-- Très étroit : l'icône seule, le libellé reste en info-bulle. -->
          <FilterChip icon="material-symbols:high-quality-outline-rounded" pressed={hiRes} title={$t("tracks_view.hires")} onclick={() => (hiRes = !hiRes)}><span class="@max-xl:hidden">{$t("tracks_view.hires")}</span></FilterChip>
          <FilterChip icon="material-symbols:graphic-eq-rounded" pressed={sansPerte} title={$t("tracks_view.lossless")} onclick={() => (sansPerte = !sansPerte)}><span class="@max-xl:hidden">{$t("tracks_view.lossless")}</span></FilterChip>
          <FilterChip icon="material-symbols:favorite-outline-rounded" pressed={favoris} title={$t("tracks_view.favorites")} onclick={() => (favoris = !favoris)}><span class="@max-xl:hidden">{$t("tracks_view.favorites")}</span></FilterChip>
          <FilterChip icon="material-symbols:hide-image-outline-rounded" pressed={sansPochette} title={$t("tracks_view.no_cover")} onclick={() => (sansPochette = !sansPochette)}><span class="@max-xl:hidden">{$t("tracks_view.no_cover")}</span></FilterChip>
          <div class="relative">
            <FilterChip icon="material-symbols:sell-outline-rounded" pressed={!!genre} menu onclick={ouvrirGenres}>
              <span class="max-w-40 truncate">{genre ?? $t("tracks_view.genre")}</span>
            </FilterChip>
            <Menu bind:open={menuGenre} align="left" class="w-60 max-h-80 overflow-y-auto scrollbar-app">
              <MenuItem actif={!genre} onclick={() => { genre = null; menuGenre = false; }}>{$t("tracks_view.all_genres")}</MenuItem>
              {#if genres === null}
                <p class="px-2.5 py-2 text-xs text-(--rg-mu)">{$t("common.loading")}</p>
              {:else}
                {#each genres as g (g.name)}
                  <MenuItem actif={genre === g.name} onclick={() => { genre = g.name; menuGenre = false; }}>
                    {g.name}
                    {#snippet fin()}<span class="font-mono text-[11px] text-(--rg-mu2)">{nombre(g.total_tracks)}</span>{/snippet}
                  </MenuItem>
                {/each}
              {/if}
            </Menu>
          </div>
          {#if filtresActifs}
            <TextButton icon="material-symbols:filter-alt-off-outline-rounded" label={$t("tracks_view.clear_filters")} labelClass="@max-3xl:hidden" onclick={effacerFiltres} />
          {/if}
        </div>

        <span class="order-3 flex-1 @max-6xl:hidden"></span>

        <div class="order-3 flex items-center gap-2.5">
          <MenuSelect value={tri} options={TRIS.map((x) => ({ value: x.cle, label: $t(x.libelle), icon: x.icone }))}
                      onchange={(v) => { tri = v; sens = "asc"; }} labelClass="@max-xl:hidden" menuClass="w-55"
                      label={tri.replace(/^tag:(custom:)?/, "")} icon="material-symbols:sort-rounded"
                      indicator={sens === "asc" ? "material-symbols:arrow-upward-rounded" : "material-symbols:arrow-downward-rounded"}>
            {#snippet fin(fermer)}
              <MenuItem icon="material-symbols:swap-vert-rounded" onclick={() => { sens = sens === "asc" ? "desc" : "asc"; fermer(); }}>{$t("tracks_view.reverse")}</MenuItem>
            {/snippet}
          </MenuSelect>

          <SegmentedControl variant="discret" label={$t("tracks_view.detailed")} value={detaille ? "detaille" : "compact"} onchange={(v) => (detaille = v === "detaille")}
                            options={[
                              { value: "compact", label: $t("tracks_view.compact"), icon: "material-symbols:density-small-rounded", iconOnly: true },
                              { value: "detaille", label: $t("tracks_view.detailed"), icon: "material-symbols:density-medium-rounded", iconOnly: true },
                            ]} />

          <TrackColumnsButton {libraryId} />

          <SelectionToggle />
        </div>
      </div>
    </div>

    <!-- ─── Morceaux, chargés au fil du défilement ─── -->
    <div class="flex-1 relative min-h-0">
      <div class="absolute inset-0 overflow-y-auto scrollbar-app pl-4 md:pl-8 pr-10 md:pr-14 pb-16" bind:this={defilement} onscroll={defiler}>
        {#if chargement && tracks.length === 0}
          <div class="flex items-center justify-center py-20 text-(--rg-mu)">
            <Icon icon="material-symbols:progress-activity" width="26" class="animate-spin" />
          </div>
        {:else if tracks.length === 0}
          <EmptyResult message={$t("tracks_view.no_match")} effacerLabel={$t("tracks_view.clear_filters")} oneffacer={filtresActifs ? effacerFiltres : null} />
        {:else}
          {#if debut > 0 && suite}
            <div class="flex justify-center py-3 text-(--rg-mu)"><Icon icon="material-symbols:progress-activity" width="20" class="animate-spin" /></div>
          {/if}
          <LibraryTrackTable {libraryId} {tracks} columns={colonnes} {largeurs} {detaille} sortKey={tri} sortDir={sens} onsort={trierPar} />
          {#if suite}
            <div class="flex justify-center py-4 text-(--rg-mu)"><Icon icon="material-symbols:progress-activity" width="20" class="animate-spin" /></div>
          {/if}
          <p class="py-3 text-center text-[11px] tabular-nums text-(--rg-mu2)">
            {$t("tracks_view.loaded").replace("{n}", nombre(debut + tracks.length)).replace("{total}", nombre(total))}
          </p>
        {/if}
      </div>

      {#if tri === "title" && lettres.size > 0}
        <AlphabetNav availableLetters={new Set(lettres.keys())} onletter={allerLettre} toujours />
      {/if}
    </div>
  </div>

{/if}
