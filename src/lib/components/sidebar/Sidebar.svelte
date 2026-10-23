<script lang="ts">
import { couverturesPlaylists } from "#lib/stores/playlist/covers.store";
import Icon from "@iconify/svelte";
import { t, currentLocale } from "#lib/i18n";
import { goto } from "$app/navigation";
import { page } from "$app/state";
import { onMount } from "svelte";
import { liked, likedCount } from "#lib/stores/playlist/like.store";
import { popinStore } from "#lib/stores/ui/popin.store";
import AddLibraryPopin from "#lib/components/library/common/popin/AddLibraryPopin.svelte";
import AddPlaylistPopin from "#lib/components/playlist/popin/AddPlaylistPopin.svelte";
import SmartPlaylistPopin from "#lib/components/playlist/smart/SmartPlaylistPopin.svelte";
import PlaylistZoneMenu from "#lib/components/ui/contextmenu/PlaylistZoneMenu.svelte";
import LibraryZoneMenu from "#lib/components/ui/contextmenu/LibraryZoneMenu.svelte";
import OpenZoneMenu from "#lib/components/ui/contextmenu/OpenZoneMenu.svelte";
import LibrarySelector from "#lib/components/library/common/LibrarySelector.svelte";
import { libraryStore } from "#lib/stores/library/library.store";
import { playlistStore } from "#lib/stores/playlist/playlist.store";
import { pinsStore } from "#lib/stores/library/pins.store";
import { recentCount } from "#lib/stores/recent/recent.store";
import { podcastStore } from "#lib/stores/podcast/podcast.store";
import { profilSelector } from "#lib/stores/profil/profil.store";
import { sidebarStore } from "#lib/stores/ui/sidebar.store";
import { settingsStore } from "#lib/stores/settings/settings.store";
import { handleClickOpenDirectory, handleClickOpenFile } from "#lib/actions/player/PlayerAction";
import PlaylistItem from "./PlaylistItem.svelte";
import SidebarButton from "./SidebarButton.svelte";
import SidebarPins from "./SidebarPins.svelte";
import SidebarRow from "./SidebarRow.svelte";
import SidebarNav from "./SidebarNav.svelte";
import SidebarFooter from "./SidebarFooter.svelte";
import SidebarTitle from "./SidebarTitle.svelte";
import SidebarThumbnail from "./SidebarThumbnail.svelte";
import SourceSwitcher from "./SourceSwitcher.svelte";
import { currentSource } from "#lib/stores/source/source.store";
import { sidebarOrganize } from "#lib/stores/ui/sidebarOrganize.store";
import { radioStore } from "#lib/stores/radio/radio.store";
import RadioLogo from "#lib/components/radio/RadioLogo.svelte";

const pathname = $derived(page.url.pathname);
const replie = $derived($settingsStore.sidebar_collapsed === 'true');
const libraryId = $derived($libraryStore.librarySelected?.id ?? null);

// `!== 'false'` : sans réglage en base, le défaut est « visible ».
// Sans cœur, plus rien ne remplit les likés : la liste suit le bouton favori.
const favorisActifs = $derived($settingsStore.show_favorites !== 'false');
const montrerLikes = $derived(favorisActifs && $settingsStore.show_liked_in_playlists !== 'false');
const montrerRecents = $derived($settingsStore.show_recent_in_playlists !== 'false');
// Toute la section (Apparence → Barre latérale) : playlists épinglées, likés, récents.
const montrerPlaylists = $derived($settingsStore.show_sidebar_playlists !== 'false');
// Source serveur : rien à importer ni à créer depuis l'ordinateur.
const sourceServeur = $derived(typeof $currentSource === 'number' && $currentSource > 0);
const montrerBoutonsOuvrir = $derived($settingsStore.show_open_buttons === 'true' && !sourceServeur);
const podcastsActifs = $derived($settingsStore.podcasts === 'true');
const radiosActives = $derived($settingsStore.radios === 'true');
const profilId = $derived($profilSelector.profilSelected?.id ?? null);
$effect(() => {
  if (podcastsActifs && profilId != null) podcastStore.ensureLoaded(profilId);
  if (radiosActives && profilId != null) radioStore.ensureLoaded(profilId);
});
const podcastsNouveaux = $derived($podcastStore.podcasts.reduce((n, p) => n + p.new_count, 0));
const podcastsCouvertures = $derived($podcastStore.podcasts.map((p) => p.cover_path).filter((c): c is string => !!c));
// Podcasts favoris : chacun sa ligne sous « Podcasts ».
const podcastsFavoris = $derived($podcastStore.podcasts.filter((p) => p.pinned).sort((a, b) => a.title.localeCompare(b.title)));
const newCountLabel = (n: number) => `${n.toLocaleString($currentLocale)} ${$t(n === 1 ? 'podcasts.new_one' : 'podcasts.new_n')}`;

// ─── Ordre des sections, et mode « Organiser » ───
type Section = 'playlists' | 'online' | 'pins';
const TOUTES: Section[] = ['playlists', 'online', 'pins'];
const ordre = $derived.by((): Section[] => {
  let lu: unknown = [];
  try { lu = JSON.parse($settingsStore.sidebar_order || '[]'); } catch { /* réglage abîmé : ordre d'origine */ }
  const choisi = (Array.isArray(lu) ? lu : []).filter((s): s is Section => TOUTES.includes(s as Section));
  return [...new Set([...choisi, ...TOUTES])];
});
const visible = (s: Section) => s === 'playlists' ? montrerPlaylists : s === 'online' ? podcastsActifs || radiosActives : libraryId != null;
const sections = $derived(ordre.filter(visible));
const sectionTitle = (s: Section) => s === 'playlists' ? $t('nav.playlists') : s === 'online' ? $t('podcasts.kicker') : $t('sidebar.pinned');
const epinglees = $derived($playlistStore.playlists.filter((p) => p.pinned && !p.is_mix));

/** Lignes de la section Playlists : `playlist:<id>`, `liked`, `recent`, dans l'ordre choisi. */
const elementsPlaylists = $derived.by((): string[] => {
  const defaut = [...epinglees.map((p) => `playlist:${p.id}`), ...(montrerLikes ? ['liked'] : []), ...(montrerRecents ? ['recent'] : [])];
  let lu: unknown = [];
  try { lu = JSON.parse($settingsStore.sidebar_playlists_order || '[]'); } catch { /* réglage abîmé : ordre d'origine */ }
  const rang = new Map((Array.isArray(lu) ? lu : []).map((k, i) => [String(k), i]));
  // Les inconnus (playlist épinglée depuis) gardent leur place d'origine, après les rangés.
  return defaut
    .map((k, i) => ({ k, r: rang.get(k) ?? 1000 + i }))
    .sort((a, b) => a.r - b.r)
    .map((x) => x.k);
});
const findPinnedPlaylist = (k: string) => epinglees.find((p) => `playlist:${p.id}` === k) ?? null;

let edition = $state(false);
// « Essayer » des nouveautés : ouvre l'organisation.
$effect(() => {
  if (!$sidebarOrganize) return;
  sidebarOrganize.set(false);
  if (!replie) edition = true;
});
$effect(() => { if (replie) edition = false; });

/** Place la section `de` à l'endroit de `vers` ; les sections masquées gardent leur rang. */
function moveSection(de: Section, vers: Section) {
  if (de === vers) return;
  const liste = ordre.filter((s) => s !== de);
  const cible = liste.indexOf(vers) + (ordre.indexOf(de) < ordre.indexOf(vers) ? 1 : 0);
  liste.splice(cible, 0, de);
  settingsStore.set('sidebar_order', JSON.stringify(liste));
}
function shiftSection(i: number, sens: -1 | 1) {
  const vers = sections[i + sens];
  if (vers) moveSection(sections[i], vers);
}
function movePlaylist(de: number, vers: number) {
  if (de === vers) return;
  const cles = [...elementsPlaylists];
  const [x] = cles.splice(de, 1);
  cles.splice(vers, 0, x);
  settingsStore.set('sidebar_playlists_order', JSON.stringify(cles));
  // Les playlists gardent aussi cet ordre en base (page Playlists, tri « comme la barre »).
  const ids = cles.filter((k) => k.startsWith('playlist:')).map((k) => Number(k.slice(9)));
  if (ids.join() !== epinglees.map((p) => p.id).join()) playlistStore.reorder(ids);
}

// Glisser-déposer par la poignée (elle capture le pointeur), comme dans la file d'attente.
let glisse = $state<{ liste: 'sections' | 'playlists'; depart: number; survol: number } | null>(null);
function grab(e: PointerEvent, liste: 'sections' | 'playlists', i: number) {
  if (e.button !== 0) return;
  e.preventDefault();
  e.stopPropagation();
  glisse = { liste, depart: i, survol: i };
  (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
}
function drag(e: PointerEvent) {
  if (!glisse) return;
  const attr = glisse.liste === 'sections' ? 'data-sec-i' : 'data-pl-i';
  const el = document.elementFromPoint(e.clientX, e.clientY)?.closest(`[${attr}]`) as HTMLElement | null;
  if (el) glisse = { ...glisse, survol: Number(el.getAttribute(attr)) };
}
function drop() {
  if (glisse) {
    if (glisse.liste === 'sections') moveSection(sections[glisse.depart], sections[glisse.survol]);
    else movePlaylist(glisse.depart, glisse.survol);
  }
  glisse = null;
}

// Pastille sur le « … » quand un raccourci est masqué : c'est là qu'on le retrouve.
const raccourcisMasques = $derived(
  (favorisActifs && $settingsStore.show_liked_in_playlists === 'false' ? 1 : 0) +
  ($settingsStore.show_recent_in_playlists === 'false' ? 1 : 0)
);

let menuZone = $state<{ x: number; y: number } | null>(null);
let menuBibliotheque = $state<{ x: number; y: number } | null>(null);
let menuOuvrir = $state<{ x: number; y: number } | null>(null);

const openSubMenu = (e: MouseEvent) => {
  const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
  return { x: r.right, y: r.bottom + 4 };
};

function genreLabel(genre: string, n: number): string {
  if (n === 0) return `${genre} · ${$t('sidebar.empty')}`;
  const titres = n === 1 ? $t('home.track_one') : $t('home.tracks_n').replace('{n}', n.toLocaleString($currentLocale));
  return `${genre} · ${titres}`;
}

function nav(path: string) {
  goto(path);
  sidebarStore.close();
}

$effect(() => { pinsStore.load(libraryId); });

onMount(() => {
  liked.refresh();
  playlistStore.init();
});
</script>

{#snippet secPlaylists()}
    {#if !replie}
      <SidebarTitle titre={$t('nav.playlists')} aide={$t('playlists_view.all')} onclick={() => nav('/playlists')}>
        {#snippet actions()}
          <SidebarButton
            icon="material-symbols:auto-awesome-outline-rounded"
            label={$t('playlist.new_smart')}
            taille={16}
            onclick={() => popinStore.open($t('playlist.new_smart'), SmartPlaylistPopin, {},
              { size: 'xl', icon: 'lucide:sparkles', flush: true })}
          />
          <SidebarButton
            icon="material-symbols:add-rounded"
            label={$t('playlist.new')}
            taille={16}
            onclick={() => popinStore.open($t('nav.playlists'), AddPlaylistPopin, {})}
          />
          <SidebarButton
            icon="material-symbols:more-horiz"
            label={$t('playlist.options')}
            taille={16}
            onclick={(e) => menuZone = openSubMenu(e)}
          >
            {#if raccourcisMasques > 0}
              <span class="absolute top-1 right-1 w-1.5 h-1.5 rounded-full bg-(--sb-g)"></span>
            {/if}
          </SidebarButton>
        {/snippet}
      </SidebarTitle>
    {/if}

    {#each elementsPlaylists as k (k)}{@render lignePlaylist(k, false)}{/each}
{/snippet}

{#snippet lignePlaylist(k: string, edite: boolean)}
  {#if k === 'liked'}
    <SidebarRow
      titre={$t('playlist_page.liked_title')}
      sousTitre={genreLabel($t('sidebar.playlist'), $likedCount)}
      replie={replie && !edite}
      actif={!edite && pathname === '/playlist/liked'}
      onclick={() => nav('/playlist/liked')}
    >
      {#snippet vignette()}
        <SidebarThumbnail couvertures={$couverturesPlaylists.liked ?? []} couleur="#f43f5e" icone="material-symbols:favorite-rounded" />
      {/snippet}
    </SidebarRow>
  {:else if k === 'recent'}
    <SidebarRow
      titre={$t('playlist_page.recent_title')}
      sousTitre={genreLabel($t('sidebar.history'), $recentCount)}
      replie={replie && !edite}
      actif={!edite && pathname === '/playlist/recent'}
      onclick={() => nav('/playlist/recent')}
    >
      {#snippet vignette()}
        <SidebarThumbnail couvertures={$couverturesPlaylists.recent ?? []} couleur="#0ea5e9" icone="material-symbols:history-rounded" />
      {/snippet}
    </SidebarRow>
  {:else}
    {@const playlist = findPinnedPlaylist(k)}
    {#if playlist}
      <PlaylistItem {playlist} replie={replie && !edite} couvertures={$couverturesPlaylists[`playlist:${playlist.id}`] ?? []} />
    {/if}
  {/if}
{/snippet}

{#snippet secOnline()}
      {#if !replie}
        <SidebarTitle titre={$t('podcasts.kicker')} aide={$t('podcasts.kicker')} onclick={() => nav(podcastsActifs ? '/podcasts' : '/radios')} />
      {/if}
      {#if podcastsActifs}
      <SidebarRow
        titre={$t('podcasts.title')}
        sousTitre={podcastsNouveaux > 0
          ? newCountLabel(podcastsNouveaux)
          : `${$podcastStore.podcasts.length.toLocaleString($currentLocale)} ${$t($podcastStore.podcasts.length === 1 ? 'podcasts.count_one' : 'podcasts.count_n')}`}
        {replie}
        actif={pathname.startsWith('/podcasts') && !podcastsFavoris.some((p) => pathname === `/podcasts/${p.id}`)}
        onclick={() => nav('/podcasts')}
      >
        {#snippet vignette()}
          <SidebarThumbnail couvertures={podcastsCouvertures} couleur="#8b5cf6" icone="material-symbols:podcasts-rounded" />
        {/snippet}
      </SidebarRow>
      {#each podcastsFavoris as p (p.id)}
        <SidebarRow
          titre={p.title}
          sousTitre={p.new_count > 0
            ? newCountLabel(p.new_count)
            : (p.author ?? $t('podcasts.kicker_podcast'))}
          {replie}
          actif={pathname === `/podcasts/${p.id}`}
          onclick={() => nav(`/podcasts/${p.id}`)}
        >
          {#snippet vignette()}
            <SidebarThumbnail couvertures={p.cover_path ? [p.cover_path] : []} couleur="#8b5cf6" icone="material-symbols:podcasts-rounded" />
          {/snippet}
        </SidebarRow>
      {/each}
      {/if}
      {#if radiosActives}
        <SidebarRow
          titre={$t('radios.title')}
          sousTitre={`${$radioStore.radios.length.toLocaleString($currentLocale)} ${$t($radioStore.radios.length === 1 ? 'radios.favorites_one' : 'radios.favorites_n')}`}
          {replie}
          actif={pathname.startsWith('/radios')}
          onclick={() => nav('/radios')}
        >
          {#snippet vignette()}
            <!-- Le logo de la dernière radio écoutée ; à défaut, le poste. -->
            {#if $radioStore.recent[0]}
              <RadioLogo path={$radioStore.recent[0].logo_path} favicon={$radioStore.recent[0].favicon_url} icone={18} class="w-full h-full rounded-[inherit]" />
            {:else}
              <SidebarThumbnail couvertures={[]} couleur="#f43f5e" icone="material-symbols:radio-rounded" />
            {/if}
          {/snippet}
        </SidebarRow>
      {/if}
{/snippet}

{#snippet secPins()}
  {#if libraryId != null}
    <SidebarPins {libraryId} {replie} />
  {/if}
{/snippet}

<aside
  class="sidebar h-full shrink-0 flex flex-col gap-3 overflow-y-auto overflow-x-hidden smart-scroll
         bg-(--sb-bg) border-r border-(--sb-sep) transition-[width] duration-200
         {replie ? 'w-19 items-center pt-3 pb-3' : 'w-72 px-4 pt-4 pb-3'}"
>
  <!-- ─── Haut : ouvrir, navigation, bibliothèque. Le logo est dans la barre de titre. ─── -->
  <div class="shrink-0 flex flex-col gap-3 {replie ? 'items-center' : ''}">
    <SourceSwitcher {replie} />
    <!-- Affiché par défaut ; se masque depuis son « … » ou Réglages → Apparence. -->
    {#if montrerBoutonsOuvrir && !replie}
      <div class="flex items-center gap-2">
        {#each [
          { icone: 'material-symbols:audio-file-outline-rounded', label: $t('nav.import_file'), action: handleClickOpenFile },
          { icone: 'material-symbols:create-new-folder-outline-rounded', label: $t('nav.import_folder'), action: handleClickOpenDirectory },
        ] as bouton (bouton.icone)}
          <button
            type="button"
            class="flex-1 h-9 flex items-center justify-center gap-1.5 rounded-lg cursor-pointer transition-colors
                   text-xs font-semibold bg-(--sb-s1) border border-(--sb-bd) text-(--sb-tx2)
                   hover:border-(--sb-bd2) hover:text-(--sb-tx)"
            onclick={() => bouton.action()}
          >
            <Icon icon={bouton.icone} width="17" />
            {bouton.label}
          </button>
        {/each}
        <SidebarButton icon="material-symbols:more-horiz" label={$t('nav.open')} onclick={(e) => menuOuvrir = openSubMenu(e)} />
      </div>
      <!-- Part avec la section. -->
      <div class="h-px bg-linear-to-r from-transparent via-(--sb-mu)/40 to-transparent" role="presentation"></div>
    {/if}

    <SidebarNav {replie} />

    {#if !$libraryStore.isLoading && $libraryStore.libraries.length === 0}
      <button
        type="button"
        class="flex items-center gap-3 rounded-xl cursor-pointer transition-colors
               bg-(--sb-s1) border border-dashed border-(--sb-bd2) hover:border-(--sb-g)
               {replie ? 'p-1.75' : 'w-full py-2 pr-2.5 pl-2'}"
        onclick={() => popinStore.open($t('library.create_library'), AddLibraryPopin, {})}
        title={$t('library.create_library')}
      >
        <span class="w-9 h-9 rounded-[9px] shrink-0 flex items-center justify-center bg-(--sb-gbg) text-(--sb-g)">
          <Icon icon="material-symbols:add-rounded" width="22" />
        </span>
        {#if !replie}
          <span class="flex flex-col gap-px min-w-0 text-left">
            <b class="text-sm font-bold truncate text-(--sb-tx)">{$t('library.no_library')}</b>
            <small class="text-xs truncate text-(--sb-g) font-semibold">{$t('library.create')}</small>
          </span>
        {/if}
      </button>
    {:else}
      <div class="flex gap-2">
        <LibrarySelector {replie} />
        <!-- Ses réglages vivent dans le « … » de Fichier / Dossier ; masquée, il reprend le relais. -->
        {#if !replie && !montrerBoutonsOuvrir && !sourceServeur}
          <button
            type="button"
            class="w-9 shrink-0 rounded-xl flex items-center justify-center cursor-pointer transition-colors
                   bg-(--sb-s1) border border-(--sb-bd) text-(--sb-tx2) hover:border-(--sb-bd2) hover:text-(--sb-tx)"
            onclick={(e) => menuBibliotheque = openSubMenu(e)}
            title={$t('sidebar.library_options')}
            aria-label={$t('sidebar.library_options')}
          >
            <Icon icon="material-symbols:more-vert" width="20" />
          </button>
        {/if}
      </div>
    {/if}

  </div>

  <!-- ─── Milieu : playlists et épinglés, seule zone qui défile. Repliée, sans
       barre de défilement : elle mangerait la colonne. ─── -->
  <div
    class="flex flex-col border-t border-(--sb-sep)
           {replie
             ? 'flex-1 min-h-0 w-full items-center gap-2 py-2 overflow-y-auto overflow-x-hidden [scrollbar-width:none]'
             : 'flex-[1_0_140px] min-h-35 gap-0.5 -mx-2 px-2 pb-8 overflow-y-auto overflow-x-hidden smart-scroll sb-liste-fondu'}"
  >
    {#if edition}
      <!-- ─── Organiser : sections et playlists épinglées, glissées par leur poignée ─── -->
      <div class="sticky top-0 z-10 -mx-2 px-2 pt-2 pb-1 bg-(--sb-bg)">
        <div class="flex items-center gap-2 py-2 pl-2.5 pr-1.5 rounded-xl bg-(--sb-gbg) border border-(--sb-gbd)">
          <Icon icon="material-symbols:swap-vert-rounded" width="18" class="shrink-0 text-(--sb-g)" />
          <span class="flex-1 min-w-0 text-xs leading-snug text-(--sb-tx2)">{$t('sidebar.organize_hint')}</span>
          <button type="button" onclick={() => (edition = false)}
                  class="shrink-0 h-7 px-2.5 rounded-lg cursor-pointer text-xs font-bold bg-(--sb-g) text-white hover:brightness-110">
            {$t('sidebar.organize_done')}
          </button>
        </div>
      </div>
      {#each sections as s, i (s)}
        <div data-sec-i={i}
             class="mt-2 rounded-xl border p-1.5 transition-colors
                    {glisse?.liste === 'sections' && glisse.survol === i ? 'border-(--sb-g) bg-(--sb-gbg)' : 'border-dashed border-(--sb-bd2)'}
                    {glisse?.liste === 'sections' && glisse.depart === i ? 'opacity-60' : ''}">
          <div class="flex items-center gap-0.5">
            <span role="presentation" title={$t('sidebar.move')}
                  class="w-6 h-7.5 shrink-0 flex items-center justify-center cursor-grab active:cursor-grabbing touch-none text-(--sb-mu) hover:text-(--sb-tx)"
                  onpointerdown={(e) => grab(e, 'sections', i)} onpointermove={drag} onpointerup={drop} onpointercancel={() => (glisse = null)}>
              <Icon icon="material-symbols:drag-indicator" width="18" />
            </span>
            <span class="flex-1 min-w-0 truncate text-xs font-bold uppercase tracking-[.08em] text-(--sb-hd)">{sectionTitle(s)}</span>
            {#if i > 0}<SidebarButton icon="material-symbols:keyboard-arrow-up-rounded" label={$t('sidebar.move_up')} taille={18} onclick={() => shiftSection(i, -1)} />{/if}
            {#if i < sections.length - 1}<SidebarButton icon="material-symbols:keyboard-arrow-down-rounded" label={$t('sidebar.move_down')} taille={18} onclick={() => shiftSection(i, 1)} />{/if}
          </div>
          {#if s === 'playlists'}
            {#each elementsPlaylists as k, j (k)}
              <div data-pl-i={j}
                   class="flex items-center gap-0.5 rounded-lg transition-colors
                          {glisse?.liste === 'playlists' && glisse.survol === j ? 'bg-(--sb-gbg)' : ''}
                          {glisse?.liste === 'playlists' && glisse.depart === j ? 'opacity-60' : ''}">
                <span role="presentation" title={$t('sidebar.move')}
                      class="w-6 h-10 shrink-0 flex items-center justify-center cursor-grab active:cursor-grabbing touch-none text-(--sb-mu) hover:text-(--sb-tx)"
                      onpointerdown={(e) => grab(e, 'playlists', j)} onpointermove={drag} onpointerup={drop} onpointercancel={() => (glisse = null)}>
                  <Icon icon="material-symbols:drag-indicator" width="18" />
                </span>
                <!-- Inerte pendant l'organisation : ni navigation ni menu par mégarde. -->
                <div class="flex-1 min-w-0 pointer-events-none">{@render lignePlaylist(k, true)}</div>
              </div>
            {:else}
              <p class="px-2 py-1.5 text-xs text-(--sb-mu)">{$t('sidebar.organize_no_playlist')}</p>
            {/each}
          {/if}
        </div>
      {/each}
    {:else}
      {#each sections as s (s)}
        {#if s === 'playlists'}{@render secPlaylists()}{:else if s === 'online'}{@render secOnline()}{:else}{@render secPins()}{/if}
      {/each}
    {/if}
  </div>

  <!-- ─── Pied : état du scan et profil ─── -->
  <SidebarFooter {replie} />
</aside>

{#if menuZone}
  <PlaylistZoneMenu x={menuZone.x} y={menuZone.y} onclose={() => menuZone = null} onorganize={() => (edition = true)} />
{/if}

{#if menuBibliotheque}
  <LibraryZoneMenu x={menuBibliotheque.x} y={menuBibliotheque.y} onclose={() => menuBibliotheque = null} />
{/if}

{#if menuOuvrir}
  <OpenZoneMenu x={menuOuvrir.x} y={menuOuvrir.y} onclose={() => menuOuvrir = null} />
{/if}
