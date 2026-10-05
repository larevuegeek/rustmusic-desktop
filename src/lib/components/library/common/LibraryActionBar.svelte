<script lang="ts">
import { recupererPochettes, recupererPortraits } from "#lib/actions/library/ImageAction";
// « Ajouter un dossier » et le menu « Gérer » : scan, dossiers, pochettes, import, suppression.
import { goto } from "$app/navigation";
import Icon from "@iconify/svelte";
import { invoke } from "@tauri-apps/api/core";
import { t, currentLocale } from "#lib/i18n";
import { oublierLocalisations } from "#lib/helper/library/trackLocation";
import { tailleLisible } from "#lib/helper/tools/sizeTools";
import { libraryStore } from "#lib/stores/library/library.store";
import { libraryContentStore } from "#lib/stores/library/libraryContent.store";
import { popinStore } from "#lib/stores/ui/popin.store";
import { handleAddDirectory, handleAddFiles } from "#lib/actions/library/LibraryAction";
import Dialog from "#lib/components/ui/dialog/Dialog.svelte";
import Menu from "#lib/components/ui/menu/Menu.svelte";
import LibraryFoldersPopin from "#lib/components/library/common/popin/LibraryFoldersPopin.svelte";
import type { Library } from "#lib/types/db/library/Library";
import type { LibraryDir } from "#lib/types/db/library/LibraryDir";
import type { ActionEntete } from "#lib/stores/library/libraryHeader";

let { library, dirs = [], onchange, action: principale = null }: {
  library: Library;
  dirs?: LibraryDir[];
  onchange?: () => void;
  /** Fourni par la page (« Tout lire », « Mix aléatoire ») : remplace « Ajouter un dossier ». */
  action?: ActionEntete | null;
} = $props();

let lecture = $state(false);
async function lancer() {
  if (!principale || lecture) return;
  lecture = true;
  try { await principale.lancer(); } finally { lecture = false; }
}

let ouvert = $state(false);
let dossiers = $state(false);
let enCours = $state<{ scan?: boolean; covers?: boolean; artists?: boolean }>({});
const occupe = $derived(!!(enCours.scan || enCours.covers || enCours.artists));

const id = $derived(library.id as number);
const taille = $derived(dirs.reduce((s, d) => s + (d.total_size ?? 0), 0));
const etat = $derived(
  dirs.some((d) => d.scan_status === "scanning") || enCours.scan ? "scanning" : dirs.some((d) => d.scan_status === "error") ? "error" : "ok",
);
const sansPochette = $derived($libraryContentStore.albums.filter((a) => !a.cover_url).length);
const nombre = (n: number) => n.toLocaleString($currentLocale);

async function agir(quoi: "scan" | "covers" | "artists", tache: () => Promise<void>) {
  if (enCours[quoi]) return;
  enCours[quoi] = true;
  try {
    await tache();
  } catch (e) {
    console.error(`[bibliothèque] ${quoi} :`, e);
  } finally {
    enCours[quoi] = false;
  }
}

const rescanner = () => agir("scan", async () => {
  ouvert = false;
  await invoke("rescan_library", { libraryId: id });
  // Un scan renumérote : les fiches mises en cache peuvent ne plus exister.
  oublierLocalisations();
  libraryContentStore.refresh();
  onchange?.();
});

const pochettes = () => agir("covers", () => recupererPochettes([id]));
const portraits = () => agir("artists", () => recupererPortraits());

function supprimer() {
  ouvert = false;
  popinStore.open($t("library_head.delete"), Dialog, {
    message: $t("library_head.delete_confirm").replace("{name}", library.name),
    confirmText: $t("common.delete"),
    cancelText: $t("common.cancel"),
    variant: "danger",
    onConfirm: async () => {
      try {
        await libraryStore.removeLibrary(library);
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

<!-- Une action riche : tuile d'icône, titre, sous-titre, fin (raccourci, flèche, compteur). -->
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
      <span class="shrink-0 px-1.75 py-0.5 rounded-[9px] font-mono text-[11px] font-bold bg-(--rg-ambg) text-(--rg-am)">{nombre(fin)}</span>
    {/if}
  </button>
{/snippet}

<div class="flex items-center gap-2 shrink-0">
  <button
    type="button"
    class="h-10 pl-3 pr-4 rounded-[10px] flex items-center gap-2 text-sm font-bold whitespace-nowrap cursor-pointer transition-colors
           bg-(--rg-g) text-(--rg-on-g) hover:bg-[#34d673]"
    onclick={() => (principale ? lancer() : handleAddDirectory(id))}
  >
    {#if principale}
      <Icon icon={lecture ? "material-symbols:progress-activity" : principale.icone} width="20" class={lecture ? "animate-spin" : ""} />
      <span class="max-sm:hidden">{$t(principale.cle)}</span>
    {:else}
      <Icon icon="material-symbols:add-rounded" width="20" />
      <span class="max-sm:hidden">{$t("library_head.add_folder")}</span>
    {/if}
  </button>

  <div class="relative">
    <button
      type="button"
      aria-expanded={ouvert}
      class="h-10 pl-3 pr-4 rounded-[10px] flex items-center gap-2 text-sm font-semibold whitespace-nowrap cursor-pointer transition-colors border
             {ouvert ? 'text-(--rg-tx) border-(--rg-bd2) bg-(--rg-carte)' : 'text-(--rg-tx2) border-(--rg-bd2) hover:text-(--rg-tx) hover:bg-(--rg-carte)'}"
      onclick={() => (ouvert = !ouvert)}
    >
      <Icon icon={occupe ? "material-symbols:progress-activity" : "material-symbols:more-horiz"} width="20" class={occupe ? "animate-spin" : ""} />
      {$t("library_head.manage")}
    </button>

    <Menu bind:open={ouvert} class="w-85 p-0! overflow-hidden rounded-2xl! dark:bg-linear-to-b dark:from-[#1c2220] dark:to-[#161a18]">
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
        {@render action("material-symbols:sync-rounded", $t("library_head.rescan"), $t("library_head.rescan_desc"), rescanner, undefined, false, !!enCours.scan)}
        {@render action("material-symbols:folder-open-outline-rounded", $t("library_head.folders"),
          [dirs.length === 1 ? $t("library_head.dirs_one") : $t("library_head.dirs_n").replace("{n}", String(dirs.length)), tailleLisible(taille, $currentLocale)].filter(Boolean).join(" · "),
          () => { ouvert = false; dossiers = true; }, "fleche")}
        {@render action("material-symbols:upload-file-outline-rounded", $t("library.add_files"), $t("library_head.add_files_desc"), () => { ouvert = false; handleAddFiles(id); })}
        {@render action("material-symbols:image-search-rounded", $t("library_head.covers"),
          sansPochette === 0 ? $t("library_head.covers_ok") : sansPochette === 1 ? $t("library_head.covers_missing_one") : $t("library_head.covers_missing_n").replace("{n}", nombre(sansPochette)),
          pochettes, sansPochette, sansPochette > 0, !!enCours.covers)}
        {@render action("material-symbols:person-outline-rounded", $t("library_head.artist_images"), $t("library_head.artist_images_desc"), portraits, undefined, false, !!enCours.artists)}
        {@render action("material-symbols:swap-vert-rounded", $t("library_head.import_export"), $t("library_head.import_export_desc"), () => { ouvert = false; goto("/settings/storage"); }, "fleche")}
      </div>

      <div class="p-1.5 border-t border-(--rg-bd) dark:border-[#262d29]">
        <button type="button" role="menuitem" class="group/a w-full flex items-center gap-3 p-2 rounded-[10px] text-left cursor-pointer transition-colors hover:bg-red-500/8" onclick={supprimer}>
          <span class="{tuile} bg-red-500/8 border-red-500/20 group-hover/a:bg-red-500/15 group-hover/a:border-red-500/35">
            <Icon icon="material-symbols:delete-outline-rounded" width="18" class="text-red-600 dark:text-red-400" />
          </span>
          <span class="flex-1 min-w-0">
            <span class="block text-[13px] font-semibold text-red-600 dark:text-red-400">{$t("library_head.delete")}</span>
            <span class="block mt-0.5 truncate text-[11.5px] text-(--rg-mu)">{$t("library_head.delete_desc")}</span>
          </span>
        </button>
      </div>
    </Menu>
  </div>
</div>

{#if dossiers}
  <LibraryFoldersPopin bind:open={dossiers} libraryId={id} />
{/if}
