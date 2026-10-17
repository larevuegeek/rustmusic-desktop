<script lang="ts">
// « Ajouter un dossier » et le menu « Gérer » : scan, dossiers, pochettes, nom, défaut, suppression.
import Icon from "@iconify/svelte";
import { t } from "#lib/i18n";
import { handleAddDirectory } from "#lib/actions/library/LibraryAction";
import Menu from "#lib/components/ui/menu/Menu.svelte";
import LibraryManageMenu, { isLibraryBusy } from "#lib/components/library/common/LibraryManageMenu.svelte";
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
async function start() {
  if (!principale || lecture) return;
  lecture = true;
  try { await principale.lancer(); } finally { lecture = false; }
}

let ouvert = $state(false);
let dossiers = $state(false);

const id = $derived(library.id as number);
const occupe = $derived(isLibraryBusy(id));
</script>


<div class="flex items-center gap-2 shrink-0">
  <button
    type="button"
    class="h-10 pl-3 pr-4 rounded-[10px] flex items-center gap-2 text-sm font-bold whitespace-nowrap cursor-pointer transition-colors
           bg-(--rg-g) text-(--rg-on-g) hover:bg-[#34d673]"
    onclick={() => (principale ? start() : handleAddDirectory(id))}
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
      <LibraryManageMenu {library} {dirs} {onchange} onclose={() => (ouvert = false)} onfolders={() => (dossiers = true)} />
    </Menu>
  </div>
</div>

{#if dossiers}
  <LibraryFoldersPopin bind:open={dossiers} libraryId={id} />
{/if}
