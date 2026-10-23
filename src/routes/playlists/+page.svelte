<script lang="ts">
// Toutes les playlists du profil : la barre latérale n'en montre que les épinglées.
import { couverturesPlaylists } from "#lib/stores/playlist/covers.store";
import { sourceFilter } from "#lib/stores/source/sourceFilter.store";
import { profilSelector } from "#lib/stores/profil/profil.store";
import Icon from "@iconify/svelte";
import { t, currentLocale } from "#lib/i18n";
import { playlistStore } from "#lib/stores/playlist/playlist.store";
import { settingsStore } from "#lib/stores/settings/settings.store";
import { liked, likedCount } from "#lib/stores/playlist/like.store";
import { recentCount } from "#lib/stores/recent/recent.store";
import { popinStore } from "#lib/stores/ui/popin.store";
import { viewMode } from "#lib/stores/ui/viewMode.store";
import { readLocal, writeLocal } from "#lib/helper/tools/storage";
import { sortKey, comparerNaturel } from "#lib/helper/library/sortKey";
import SearchField from "#lib/components/ui/input/SearchField.svelte";
import SegmentedControl from "#lib/components/ui/input/SegmentedControl.svelte";
import MenuSelect from "#lib/components/ui/menu/MenuSelect.svelte";
import ViewModeSwitch from "#lib/components/ui/input/ViewModeSwitch.svelte";
import EmptyResult from "#lib/components/ui/feedback/EmptyResult.svelte";
import PlaylistCard from "#lib/components/playlist/PlaylistCard.svelte";
import SelectionToggle from "#lib/components/ui/selection/SelectionToggle.svelte";
import { selectionStore } from "#lib/stores/ui/selection.store";
import { groupKey, toggleGroupSelection, toggleTracksGroup } from "#lib/helper/tools/selectionGroups";
import { invoke } from "@tauri-apps/api/core";
import { get } from "svelte/store";
import type { TrackListView } from "#lib/types/ui/library/track/TrackListView";
import type { RecentFileListView } from "#lib/types/ui/recent/RecentFileListView";
import PlaylistContextMenu from "#lib/components/ui/contextmenu/PlaylistContextMenu.svelte";
import AddPlaylistPopin from "#lib/components/playlist/popin/AddPlaylistPopin.svelte";
import SmartPlaylistPopin from "#lib/components/playlist/smart/SmartPlaylistPopin.svelte";
import type { Playlist } from "#lib/types/db/playlist/Playlist";

// Les mix vivent sur leur page : ce ne sont pas des playlists.
const playlists = $derived($playlistStore.playlists.filter((p) => !p.is_mix));

// ─── Filtres et tri (retenus d'une visite à l'autre) ───
let recherche = $state("");
type Sorte = "toutes" | "classiques" | "auto";
let sorte = $state<Sorte>("toutes");
type Tri = "recent" | "name" | "size" | "sidebar";
const CLE_TRI = "playlists:tri";
const triLu = readLocal(CLE_TRI, "recent");
let tri = $state<Tri>(["recent", "name", "size", "sidebar"].includes(triLu) ? (triLu as Tri) : "recent");
$effect(() => writeLocal(CLE_TRI, tri));

const byName = (x: Playlist, y: Playlist) => comparerNaturel.compare(sortKey(x.name), sortKey(y.name));
const TRI_FN: Record<Tri, (x: Playlist, y: Playlist) => number> = {
  recent: (x, y) => y.created_at.localeCompare(x.created_at) || byName(x, y),
  name: byName,
  size: (x, y) => y.track_count - x.track_count || byName(x, y),
  sidebar: (x, y) => x.position - y.position || y.id - x.id,
};

const filtresActifs = $derived(!!recherche.trim() || sorte !== "toutes");
const visibles = $derived.by(() => {
  const q = recherche.trim().toLowerCase();
  return playlists
    .filter((p) => (!q || `${p.name} ${p.description ?? ""}`.toLowerCase().includes(q))
      && (sorte === "toutes" || (sorte === "auto") === p.is_smart))
    .sort(TRI_FN[tri]);
});
// Source serveur : celles du serveur à part de celles gardées dans RustMusic.
const groupes = $derived.by((): { titre: string | null; liste: Playlist[] }[] => {
  const app = visibles.filter((p) => p.server_id == null);
  const serveur = visibles.filter((p) => p.server_id != null);
  if (!app.length || !serveur.length) return [{ titre: null, liste: visibles }];
  return [{ titre: $t("servers.playlists_app"), liste: app }, { titre: $t("servers.playlists_server"), liste: serveur }];
});
// Les raccourcis bâtis dans l'application, en tête quand rien n'est filtré.
const montrerLikes = $derived($settingsStore.show_favorites !== "false");
const raccourcis = $derived(!filtresActifs);

const epinglees = $derived(playlists.filter((p) => p.pinned).length);
const formatNumber = (n: number) => n.toLocaleString($currentLocale);
const trackCount = (n: number) => (n === 1 ? $t("home.track_one") : $t("home.tracks_n").replace("{n}", formatNumber(n)));
function subtitle(p: Playlist) {
  const genre = p.is_smart ? $t("sidebar.playlist_auto") : $t("sidebar.playlist");
  if (p.track_count === 0) return p.is_smart ? genre : `${genre} · ${$t("sidebar.empty")}`;
  return `${genre} · ${trackCount(p.track_count)}`;
}

const grille = $derived($viewMode !== "list");
let menu = $state<{ x: number; y: number; playlist: Playlist } | null>(null);

function createPlaylist() {
  popinStore.open($t("nav.playlists"), AddPlaylistPopin, {});
}
function createSmartPlaylist() {
  popinStore.open($t("playlist.new_smart"), SmartPlaylistPopin, {}, { size: "xl", icon: "lucide:sparkles", flush: true });
}

// ─── Sélection : une playlist cochée apporte tous ses titres ───
const LIKED_KEY = "liked";
const RECENT_KEY = "recent";
const playlistKey = (id: number) => groupKey("playlist", 0, String(id));
const tracksByPaths = (paths: string[]) => invoke<TrackListView[]>("get_tracks_view_by_paths", { paths });
const selectLiked = () => toggleTracksGroup(LIKED_KEY, () => tracksByPaths([...get(liked).paths]));
const selectRecent = () =>
  toggleTracksGroup(RECENT_KEY, async () => {
    const files = await invoke<RecentFileListView[]>("get_recent_files", { profilId: $profilSelector.profilSelected?.id ?? 0, sourceId: get(sourceFilter) });
    return tracksByPaths(files.map((f) => f.path).filter((x): x is string => !!x));
  });
</script>

<div class="biblio-maquette flex flex-col h-full">
  <!-- ─── En-tête ─── -->
  <div class="shrink-0 flex flex-wrap items-center gap-x-5 gap-y-4 px-4 md:pl-8 md:pr-14 pt-6 pb-4">
    <div class="w-16 h-16 shrink-0 rounded-2xl border flex items-center justify-center text-(--rg-g)
                bg-(--rg-gbg) border-(--rg-gbd) dark:bg-linear-145 dark:from-[#15402a] dark:to-[#0e2519] max-sm:w-12 max-sm:h-12">
      <Icon icon="material-symbols:queue-music-rounded" width="30" />
    </div>
    <div class="flex-[1_1_240px] min-w-60">
      <p class="text-xs font-bold uppercase tracking-widest text-(--rg-mu)">{$t("playlists_view.kicker")}</p>
      <h1 class="mt-0.5 mb-1.5 text-[34px] max-sm:text-2xl font-extrabold tracking-[-0.02em] leading-tight text-(--rg-tx)">{$t("nav.playlists")}</h1>
      <div class="flex flex-wrap items-baseline gap-x-4.5 gap-y-1.5 text-sm text-(--rg-mu)">
        <span><b class="font-bold text-(--rg-tx)">{formatNumber(playlists.length)}</b> {$t(playlists.length === 1 ? "playlists_view.count_one" : "playlists_view.count_n")}</span>
        <span><b class="font-bold text-(--rg-tx)">{formatNumber(epinglees)}</b> {$t(epinglees === 1 ? "playlists_view.pinned_one" : "playlists_view.pinned_n")}</span>
      </div>
    </div>
    <div class="flex flex-wrap items-center gap-2.5">
      <button type="button" onclick={createSmartPlaylist}
              class="h-10 px-4 inline-flex items-center gap-2 rounded-xl border cursor-pointer text-sm font-semibold transition-colors
                     border-(--rg-bd2) text-(--rg-tx) hover:border-(--rg-gbd) hover:text-(--rg-gtx)">
        <Icon icon="material-symbols:auto-awesome-outline-rounded" width="18" />{$t("playlists_view.new_smart")}
      </button>
      <button type="button" onclick={createPlaylist}
              class="h-10 px-4 inline-flex items-center gap-2 rounded-xl cursor-pointer text-sm font-bold bg-(--rg-g) text-(--rg-on-g) hover:brightness-110 transition">
        <Icon icon="material-symbols:add-rounded" width="20" />{$t("playlist.new")}
      </button>
    </div>
  </div>

  <!-- ─── Outils ─── -->
  <div class="@container shrink-0 pl-4 md:pl-8 pr-4 md:pr-14 py-3 border-t border-(--rg-line)">
    <div class="flex flex-wrap items-center gap-2.5">
      <SearchField bind:value={recherche} class="w-65 @max-4xl:flex-1 min-w-36"
                   placeholder={$t("playlists_view.filter").replace("{n}", formatNumber(playlists.length))} clearLabel={$t("genres_view.clear_filters")} />
      <SegmentedControl variant="discret" label={$t("playlists_view.kind")} value={sorte} onchange={(v) => (sorte = v as Sorte)}
                        options={[
                          { value: "toutes", label: $t("playlists_view.kind_all") },
                          { value: "classiques", label: $t("playlists_view.kind_classic") },
                          { value: "auto", label: $t("playlists_view.kind_smart") },
                        ]} />
      <span class="flex-1"></span>
      <MenuSelect value={tri} onchange={(v) => (tri = v as Tri)} labelClass="@max-xl:hidden" menuClass="w-60"
                  options={[
                    { value: "recent", label: $t("playlists_view.sort_recent"), icon: "material-symbols:schedule-outline-rounded" },
                    { value: "name", label: $t("playlists_view.sort_name"), icon: "material-symbols:sort-by-alpha-rounded" },
                    { value: "size", label: $t("playlists_view.sort_size"), icon: "material-symbols:format-list-numbered-rounded" },
                    { value: "sidebar", label: $t("playlists_view.sort_sidebar"), icon: "material-symbols:side-navigation-rounded" },
                  ]} />
      <SelectionToggle />
      <ViewModeSwitch />
    </div>
  </div>

  <!-- ─── Les playlists ─── -->
  <div class="flex-1 min-h-0 overflow-y-auto scrollbar-app pl-4 md:pl-8 pr-10 md:pr-14 pt-2 pb-16">
    {#if visibles.length === 0 && filtresActifs}
      <EmptyResult message={$t("playlists_view.no_match")} effacerLabel={$t("genres_view.clear_filters")} oneffacer={() => { recherche = ""; sorte = "toutes"; }} />
    {:else}
      <div class={grille ? "grid grid-cols-[repeat(auto-fill,minmax(176px,1fr))] gap-x-5 gap-y-6 pt-2" : "flex flex-col"}>
        {#if raccourcis}
          {#if montrerLikes}
            <PlaylistCard liste={!grille} href="/playlist/liked" nom={$t("playlist_page.liked_title")}
                          sousTitre={`${$t("sidebar.playlist")} · ${trackCount($likedCount)}`}
                          couleur="#f43f5e" icone="material-symbols:favorite-rounded" couvertures={$couverturesPlaylists.liked ?? []}
                          selected={$selectionStore.groupes.has(LIKED_KEY)} onselect={selectLiked} />
          {/if}
          <PlaylistCard liste={!grille} href="/playlist/recent" nom={$t("playlist_page.recent_title")}
                        sousTitre={`${$t("sidebar.history")} · ${trackCount($recentCount)}`}
                        couleur="#0ea5e9" icone="material-symbols:history-rounded" couvertures={$couverturesPlaylists.recent ?? []}
                        selected={$selectionStore.groupes.has(RECENT_KEY)} onselect={selectRecent} />
        {/if}
        {#each groupes as g (g.titre ?? "")}
          {#if g.titre}
            <h2 class="col-span-full mt-4 first:mt-0 flex items-center gap-2 text-[11px] font-bold uppercase tracking-[0.1em] text-(--rg-mu)">
              <Icon icon={g.liste[0]?.server_id != null ? "material-symbols:cloud-outline-rounded" : "material-symbols:hard-drive-outline"} width="15" />
              {g.titre} <span class="font-semibold">· {formatNumber(g.liste.length)}</span>
            </h2>
          {/if}
          {#each g.liste as p (p.id)}
            <PlaylistCard liste={!grille} href={`/playlist/${p.id}`} nom={p.name} sousTitre={subtitle(p)}
                          couleur={p.color} icone={p.icon} auto={p.is_smart} couvertures={$couverturesPlaylists[`playlist:${p.id}`] ?? []}
                          epinglee={p.pinned} onepingler={() => playlistStore.setPinned(p.id, !p.pinned)}
                          onmenu={(x, y) => (menu = { x, y, playlist: p })}
                          selected={$selectionStore.groupes.has(playlistKey(p.id))} onselect={() => toggleGroupSelection("playlist", 0, String(p.id))} />
          {/each}
        {/each}
      </div>
      {#if playlists.length === 0}
        <p class="mt-8 max-w-md text-sm leading-relaxed text-(--rg-mu)">{$t("playlists_view.empty_desc")}</p>
      {/if}
    {/if}
  </div>
</div>

{#if menu}
  <PlaylistContextMenu playlist={menu.playlist} x={menu.x} y={menu.y} onclose={() => (menu = null)} />
{/if}
