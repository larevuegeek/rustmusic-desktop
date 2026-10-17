<script lang="ts" module>
  // Tâches en cours par bibliothèque : partagées par tous les menus « Gérer » (page, barre latérale).
  type Tache = "scan" | "covers" | "artists";
  const enCours = $state<Record<number, Partial<Record<Tache, boolean>>>>({});

  export function isLibraryBusy(id: number): boolean {
    const t = enCours[id];
    return !!(t?.scan || t?.covers || t?.artists);
  }
</script>

<script lang="ts">
  // Contenu du menu « Gérer » d'une bibliothèque : scan, dossiers, pochettes, nom, défaut, suppression.
  import { goto } from "$app/navigation";
  import Icon from "@iconify/svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { t, currentLocale } from "#lib/i18n";
  import { fetchCovers, fetchPortraits } from "#lib/actions/library/ImageAction";
  import { handleAddFiles } from "#lib/actions/library/LibraryAction";
  import { forgetLocations } from "#lib/helper/library/trackLocation";
  import { readableSize } from "#lib/helper/tools/sizeTools";
  import { libraryStore } from "#lib/stores/library/library.store";
  import { libraryContentStore } from "#lib/stores/library/libraryContent.store";
  import { popinStore } from "#lib/stores/ui/popin.store";
  import Dialog from "#lib/components/ui/dialog/Dialog.svelte";
  import AddLibraryPopin from "#lib/components/library/common/popin/AddLibraryPopin.svelte";
  import type { Library } from "#lib/types/db/library/Library";
  import type { LibraryDir } from "#lib/types/db/library/LibraryDir";

  let { library, dirs: fournis, onclose, onfolders, onchange }: {
    library: Library;
    /** Absents (barre latérale) : lus à l'ouverture. */
    dirs?: LibraryDir[];
    onclose: () => void;
    onfolders: () => void;
    onchange?: () => void;
  } = $props();

  const id = $derived(library.id as number);

  let lus = $state<LibraryDir[]>([]);
  $effect(() => {
    if (fournis) return;
    const cible = id;
    invoke<LibraryDir[]>("get_library_dirs", { libraryId: cible })
      .then((d) => { if (cible === id) lus = d; })
      .catch(() => {});
  });
  const dirs = $derived(fournis ?? lus);

  const taches = $derived(enCours[id] ?? {});
  const taille = $derived(dirs.reduce((s, d) => s + (d.total_size ?? 0), 0));
  const etat = $derived(
    dirs.some((d) => d.scan_status === "scanning") || taches.scan ? "scanning" : dirs.some((d) => d.scan_status === "error") ? "error" : "ok",
  );
  // Le contenu chargé est celui de la bibliothèque ouverte : ailleurs, pas de compte.
  const ouverte = $derived($libraryStore.librarySelected?.id === id);
  const sansPochette = $derived(ouverte ? $libraryContentStore.albums.filter((a) => !a.cover_url).length : 0);
  const formatNumber = (n: number) => n.toLocaleString($currentLocale);

  async function runTask(quoi: Tache, tache: () => Promise<void>) {
    const cible = id;
    if (enCours[cible]?.[quoi]) return;
    enCours[cible] = { ...enCours[cible], [quoi]: true };
    try {
      await tache();
    } catch (e) {
      console.error(`[bibliothèque] ${quoi} :`, e);
    } finally {
      enCours[cible] = { ...enCours[cible], [quoi]: false };
    }
  }

  // Chaque action fige sa bibliothèque avant `onclose` : dans la barre latérale,
  // fermer démonte ce menu et `library` n'est plus lisible.
  const rescan = () => runTask("scan", async () => {
    const cible = id, etaitOuverte = ouverte;
    onclose();
    await invoke("rescan_library", { libraryId: cible });
    // Un scan renumérote : les fiches mises en cache peuvent ne plus exister.
    forgetLocations();
    if (etaitOuverte) libraryContentStore.refresh();
    libraryStore.refresh();
    onchange?.();
  });

  const refreshCovers = () => runTask("covers", () => fetchCovers([id]));
  const portraits = () => runTask("artists", () => fetchPortraits());

  function rename() {
    const lib = library;
    onclose();
    popinStore.open($t("selector.rename_title"), AddLibraryPopin, { library: lib });
  }

  async function makeDefault() {
    const lib = library;
    onclose();
    try {
      await libraryStore.setDefault(lib);
    } catch (e) {
      console.error("[bibliothèque] défaut :", e);
    }
  }

  function remove() {
    const lib = library;
    onclose();
    popinStore.open($t("library_head.delete"), Dialog, {
      message: $t("library_head.delete_confirm").replace("{name}", lib.name),
      confirmText: $t("common.delete"),
      cancelText: $t("common.cancel"),
      variant: "danger",
      onConfirm: async () => {
        try {
          await libraryStore.removeLibrary(lib);
          goto("/library");
        } catch (e) {
          console.error("[bibliothèque] suppression :", e);
        }
      },
      onCancel: () => {},
    });
  }

  const tuile = "w-8.5 h-8.5 shrink-0 rounded-[9px] border flex items-center justify-center transition-colors";
</script>

<!-- Une action riche : tuile d'icône, titre, sous-titre, fin (flèche, compteur). -->
{#snippet action(icone: string, titre: string, sous: string, onclick: () => void, fin?: "fleche" | number, alerte = false, tourne = false)}
  <button type="button" role="menuitem" class="group/a w-full flex items-center gap-3 p-2 rounded-[10px] text-left cursor-pointer transition-colors hover:bg-(--rg-s2) dark:hover:bg-white/5" {onclick}>
    <span class="{tuile} bg-(--rg-s2) border-(--rg-bd) dark:bg-[#232a26] dark:border-[#2b332f] group-hover/a:bg-(--rg-gbg) group-hover/a:border-(--rg-gbd)">
      <Icon icon={icone} width="18" class="{tourne ? 'animate-spin' : ''} {alerte ? 'text-(--rg-am)' : 'text-(--rg-tx2) group-hover/a:text-(--rg-g)'}" />
    </span>
    <span class="flex-1 min-w-0">
      <span class="block text-[13px] font-semibold text-(--rg-tx)">{titre}</span>
      <span class="block mt-0.5 truncate text-[11.5px] text-(--rg-mu)">{sous}</span>
    </span>
    {#if fin === "fleche"}
      <Icon icon="material-symbols:arrow-forward-rounded" width="16" class="shrink-0 text-(--rg-mu2) transition-transform group-hover/a:translate-x-0.5 group-hover/a:text-(--rg-tx2)" />
    {:else if typeof fin === "number" && fin > 0}
      <span class="shrink-0 px-1.75 py-0.5 rounded-[9px] font-mono text-[11px] font-bold bg-(--rg-ambg) text-(--rg-am)">{formatNumber(fin)}</span>
    {/if}
  </button>
{/snippet}

<!-- La bibliothèque, son emplacement et son état -->
<div class="flex items-center gap-3 px-3.5 pt-3.5 pb-3 border-b border-(--rg-bd) dark:border-[#262d29] bg-linear-135 from-(--rg-g)/10 to-transparent to-70%">
  <span class="w-9.5 h-9.5 shrink-0 rounded-[10px] border flex items-center justify-center bg-(--rg-gbg) border-(--rg-gbd) text-(--rg-g)">
    <Icon icon="material-symbols:library-music-outline-rounded" width="20" />
  </span>
  <span class="flex-1 min-w-0">
    <span class="block truncate text-sm font-bold">{library.name}</span>
    <span class="block mt-0.5 truncate font-mono text-[11px] text-(--rg-mu)" title={dirs.map((d) => d.path).join("\n")}>
      {dirs.length === 1 ? dirs[0].path : dirs.length > 1 ? $t("library_head.dirs_n").replace("{n}", String(dirs.length)) : ""}
    </span>
  </span>
  <span class="shrink-0 flex items-center gap-1.5 px-2 py-0.75 rounded-[10px] text-[11px] font-semibold whitespace-nowrap
               {etat === 'scanning' ? 'bg-(--rg-ambg) text-(--rg-am)' : etat === 'error' ? 'bg-red-500/10 text-red-600 dark:text-red-400' : 'bg-(--rg-g)/10 text-(--rg-gtx)'}">
    <span class="w-1.5 h-1.5 rounded-full bg-current shadow-[0_0_6px_currentColor] {etat === 'scanning' ? 'animate-pulse' : ''}"></span>
    {$t(etat === "scanning" ? "library_head.status_scanning" : etat === "error" ? "library_head.status_error" : "library_head.status_ok")}
  </span>
</div>

<div class="p-1.5">
  {@render action("material-symbols:sync-rounded", $t("library_head.rescan"), $t("library_head.rescan_desc"), rescan, undefined, false, !!taches.scan)}
  {@render action("material-symbols:folder-open-outline-rounded", $t("library_head.folders"),
    [dirs.length === 1 ? $t("library_head.dirs_one") : $t("library_head.dirs_n").replace("{n}", String(dirs.length)), readableSize(taille, $currentLocale)].filter(Boolean).join(" · "),
    () => { onfolders(); onclose(); }, "fleche")}
  {@render action("material-symbols:upload-file-outline-rounded", $t("library.add_files"), $t("library_head.add_files_desc"), () => { const cible = id; onclose(); handleAddFiles(cible); })}
  {@render action("material-symbols:image-search-rounded", $t("library_head.covers"),
    !ouverte ? $t("library_head.covers_desc") : sansPochette === 0 ? $t("library_head.covers_ok") : sansPochette === 1 ? $t("library_head.covers_missing_one") : $t("library_head.covers_missing_n").replace("{n}", formatNumber(sansPochette)),
    refreshCovers, sansPochette, sansPochette > 0, !!taches.covers)}
  {@render action("material-symbols:person-outline-rounded", $t("library_head.artist_images"), $t("library_head.artist_images_desc"), portraits, undefined, false, !!taches.artists)}
  {@render action("material-symbols:swap-vert-rounded", $t("library_head.import_export"), $t("library_head.import_export_desc"), () => { onclose(); goto("/settings/storage"); }, "fleche")}
</div>

<div class="p-1.5 border-t border-(--rg-bd) dark:border-[#262d29]">
  {@render action("material-symbols:edit-outline-rounded", $t("selector.rename"), $t("selector.rename_desc"), rename)}
  {#if !library.is_default}
    {@render action("material-symbols:keep-outline-rounded", $t("selector.set_default"), $t("selector.default_hint"), makeDefault)}
  {/if}
</div>

<div class="p-1.5 border-t border-(--rg-bd) dark:border-[#262d29]">
  <button type="button" role="menuitem" class="group/a w-full flex items-center gap-3 p-2 rounded-[10px] text-left cursor-pointer transition-colors hover:bg-red-500/8" onclick={remove}>
    <span class="{tuile} bg-red-500/8 border-red-500/20 group-hover/a:bg-red-500/15 group-hover/a:border-red-500/35">
      <Icon icon="material-symbols:delete-outline-rounded" width="18" class="text-red-600 dark:text-red-400" />
    </span>
    <span class="flex-1 min-w-0">
      <span class="block text-[13px] font-semibold text-red-600 dark:text-red-400">{$t("library_head.delete")}</span>
      <span class="block mt-0.5 truncate text-[11.5px] text-(--rg-mu)">{$t("library_head.delete_desc")}</span>
    </span>
  </button>
</div>
