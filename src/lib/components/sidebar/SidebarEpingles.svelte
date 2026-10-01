<script lang="ts">
import Icon from "@iconify/svelte";
import { goto } from "$app/navigation";
import { page } from "$app/state";
import { t } from "$lib/i18n";
import CoverImg from "$lib/components/ui/image/CoverImg.svelte";
import LibraryItemMenu from "$lib/components/ui/contextmenu/LibraryItemMenu.svelte";
import PinZoneMenu from "$lib/components/ui/contextmenu/PinZoneMenu.svelte";
import { pinsStore, type LibraryPin, type PinKind } from "$lib/stores/library/pins.store";
import { player } from "$lib/stores/player/player.store";
import { artistImageReadyStore } from "$lib/stores/library/artistImageReady.store";
import { sidebarStore } from "$lib/stores/ui/sidebar.store";
import { localiserUn, type TrackLocation } from "$lib/helper/library/trackLocation";
import { teinte } from "$lib/helper/tools/teinte";
import { settingsStore } from "$lib/stores/settings/settings.store";
import SidebarBouton from "./SidebarBouton.svelte";
import SidebarLigne from "./SidebarLigne.svelte";
import SidebarTitre from "./SidebarTitre.svelte";
import SidebarVignette from "./SidebarVignette.svelte";

let { libraryId, replie = false }: { libraryId: number; replie?: boolean } = $props();

// `!== 'false'` : sans réglage en base, les sections sont visibles.
const montrerAlbums = $derived($settingsStore.show_pinned_albums !== "false");
const montrerArtistes = $derived($settingsStore.show_pinned_artists !== "false");
const albums = $derived(montrerAlbums ? $pinsStore.pins.filter((p) => p.kind === "album") : []);
const artistes = $derived(montrerArtistes ? $pinsStore.pins.filter((p) => p.kind === "artist") : []);
// Une section n'apparaît qu'avec au moins une épingle ; sans aucune, la carte d'accueil.
const accueil = $derived(!replie && (montrerAlbums || montrerArtistes) && albums.length === 0 && artistes.length === 0);

function masquerEpingles() {
  settingsStore.set("show_pinned_albums", "false");
  settingsStore.set("show_pinned_artists", "false");
}
const pathname = $derived(page.url.pathname);

let menu = $state<{ pin: LibraryPin; x: number; y: number } | null>(null);
let menuSection = $state<{ kind: PinKind; x: number; y: number } | null>(null);

// Le morceau en cours : égaliseur, et « Épingler l'album en cours ». Dérivé
// d'abord : le lecteur publie sa position en continu, seul le chemin compte.
const cheminEnLecture = $derived($player.pathFile);
const tags = $derived($player.audioFile?.tags);
let lieu = $state<TrackLocation | null>(null);
$effect(() => {
  const chemin = cheminEnLecture;
  if (!chemin) { lieu = null; return; }
  localiserUn(chemin).then((l) => { if (cheminEnLecture === chemin) lieu = l; });
});

const albumEnLecture = $derived(lieu?.library_id === libraryId ? lieu.library_album_id : null);
const artisteEnLecture = $derived(lieu?.library_id === libraryId ? lieu.artist_id : null);

const lien = (pin: LibraryPin) => `/library/${libraryId}/${pin.kind === "album" ? "albums" : "artists"}/${pin.id}`;

function aller(pin: LibraryPin) {
  goto(lien(pin));
  sidebarStore.close();
}

function ouvrirSection(kind: PinKind, e: MouseEvent) {
  const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
  menuSection = { kind, x: r.right, y: r.bottom + 4 };
}
</script>

<!-- Rien d'épinglé : une carte d'accueil plutôt que deux sections vides. -->
{#if accueil}
  <div class="relative mt-3 p-3 rounded-xl bg-(--sb-s1) border border-(--sb-bd)">
    <button
      type="button"
      class="absolute top-1.5 right-1.5 w-6 h-6 rounded-md flex items-center justify-center cursor-pointer transition-colors
             text-(--sb-mu) hover:bg-(--sb-s2) hover:text-(--sb-tx)"
      onclick={masquerEpingles}
      title={`${$t("sidebar.hide_section")} — ${$t("sidebar.hide_section_note")}`}
      aria-label={$t("sidebar.hide_section")}
    >
      <Icon icon="material-symbols:close-rounded" width="16" />
    </button>

    <div class="flex items-center gap-2.5 pr-6">
      <span class="w-8 h-8 rounded-lg shrink-0 flex items-center justify-center bg-(--sb-gbg) text-(--sb-g)">
        <Icon icon="material-symbols:keep-outline-rounded" width="18" class="sb-icone" />
      </span>
      <b class="text-[13px] font-bold leading-tight text-(--sb-tx)">{$t("sidebar.pins_empty_title")}</b>
    </div>
    <p class="mt-2 text-xs leading-snug text-(--sb-mu)">{$t("sidebar.pins_empty_desc")}</p>

    <div class="mt-2.5 flex gap-1.5">
      {#each [
        { montrer: montrerAlbums, icone: "material-symbols:album-outline-rounded", label: $t("library.albums"), cle: "albums" },
        { montrer: montrerArtistes, icone: "material-symbols:mic-external-on-outline-rounded", label: $t("library.artists"), cle: "artists" },
      ].filter((b) => b.montrer) as b (b.cle)}
        <button
          type="button"
          class="h-7 px-3 flex items-center gap-1.5 rounded-full cursor-pointer transition-colors
                 text-xs font-semibold bg-(--sb-s2) text-(--sb-tx2) hover:text-(--sb-tx) hover:brightness-125"
          onclick={() => { goto(`/library/${libraryId}/${b.cle}`); sidebarStore.close(); }}
        >
          <Icon icon={b.icone} width="15" />
          {b.label}
        </button>
      {/each}
    </div>
  </div>
{/if}

<!-- ─── Albums ─── -->
{#if !replie && albums.length > 0}
  <SidebarTitre titre={$t("sidebar.pinned_albums")}>
    {#snippet actions()}
      <SidebarBouton icon="material-symbols:more-horiz" label={$t("sidebar.pinned_albums")} taille={16}
                     onclick={(e) => ouvrirSection("album", e)} />
    {/snippet}
  </SidebarTitre>
{/if}

{#each albums as pin (pin.id)}
  <SidebarLigne
    titre={pin.title}
    sousTitre={[$t("sidebar.album"), pin.subtitle].filter(Boolean).join(" · ")}
    {replie}
    actif={pathname === lien(pin)}
    enLecture={albumEnLecture === pin.id}
    onclick={() => aller(pin)}
    onmenu={(x, y) => menu = { pin, x, y }}
  >
    {#snippet vignette()}
      <SidebarVignette couvertures={pin.cover ? [pin.cover] : []} couleur="oklch(0.62 0.12 {teinte(pin.title)})"
                       icone="material-symbols:album-rounded" />
    {/snippet}
  </SidebarLigne>
{/each}

<!-- ─── Artistes ─── -->
{#if !replie && artistes.length > 0}
  <SidebarTitre titre={$t("sidebar.pinned_artists")}>
    {#snippet actions()}
      <SidebarBouton icon="material-symbols:more-horiz" label={$t("sidebar.pinned_artists")} taille={16}
                     onclick={(e) => ouvrirSection("artist", e)} />
    {/snippet}
  </SidebarTitre>
{/if}

{#if artistes.length > 0}
  <div class="flex {replie ? 'flex-col items-center gap-2' : 'flex-wrap gap-3 px-1 pb-3'}">
    {#each artistes as pin (pin.id)}
      {@const image = artistImageReadyStore.get(pin.id, $artistImageReadyStore) ?? pin.cover}
      {@const actif = pathname === lien(pin)}
      <button
        type="button"
        class="group flex flex-col items-center gap-1.5 text-xs text-center cursor-pointer {replie ? '' : 'w-16'}"
        onclick={() => aller(pin)}
        oncontextmenu={(e) => { e.preventDefault(); menu = { pin, x: e.clientX, y: e.clientY }; }}
        title={pin.title}
        aria-current={actif ? "page" : undefined}
      >
        <span class="rounded-full overflow-hidden shrink-0 {replie ? 'w-11 h-11' : 'w-13 h-13'}
                     {actif || artisteEnLecture === pin.id
                       ? 'ring-2 ring-(--sb-g)'
                       : 'ring-1 ring-(--sb-bd) group-hover:ring-2 group-hover:ring-(--sb-bd2)'}">
          {#if image}
            <CoverImg path={image} alt="" size="1x" class="w-full h-full object-cover" />
          {:else}
            <span class="sb-vignette text-base font-bold" style="--c: oklch(0.62 0.12 {teinte(pin.title)});">
              {pin.title.charAt(0).toUpperCase()}
            </span>
          {/if}
        </span>
        {#if !replie}
          <span class="w-full truncate {actif ? 'text-(--sb-g) font-semibold' : 'text-(--sb-tx2)'}">{pin.title}</span>
        {/if}
      </button>
    {/each}
  </div>
{/if}

{#if menu}
  <LibraryItemMenu
    kind={menu.pin.kind}
    id={menu.pin.id}
    title={menu.pin.title}
    {libraryId}
    x={menu.x}
    y={menu.y}
    onclose={() => menu = null}
  />
{/if}

{#if menuSection}
  <PinZoneMenu
    kind={menuSection.kind}
    {libraryId}
    enCours={menuSection.kind === "album"
      ? (albumEnLecture ? { id: albumEnLecture, titre: tags?.album ?? "" } : null)
      : (artisteEnLecture ? { id: artisteEnLecture, titre: tags?.album_artist ?? tags?.artist ?? "" } : null)}
    x={menuSection.x}
    y={menuSection.y}
    onclose={() => menuSection = null}
  />
{/if}
