<script lang="ts">
// Point de départ d'un import : page /import, ou bibliothèque encore vide (`library`).
// On choisit d'abord (annuler ne crée rien), puis la bibliothèque est créée si besoin.
import Icon from "@iconify/svelte";
import DisqueFiligrane from "#lib/components/ui/deco/DisqueFiligrane.svelte";
import { goto } from "$app/navigation";
import { t } from "#lib/i18n";
import { libraryStore } from "#lib/stores/library/library.store";
import { profilSelector } from "#lib/stores/profil/profil.store";
import { importProgressStore } from "#lib/stores/library/importProgress.store";
import { choisirDossier, choisirFichiers } from "#lib/services/library/library.service";
import { handleAddDirectory, handleAddFiles } from "#lib/actions/library/LibraryAction";
import { ONGLETS_BIBLIOTHEQUE, memoriserOnglet } from "#lib/config/libraryTabs";
import { messageErreur } from "#lib/helper/tools/errorTools";
import type { Library } from "#lib/types/db/library/Library";

let { section: cleSection = null, library = null }: { section?: string | null; library?: Library | null } = $props();

const FORMATS = ["FLAC", "MP3", "M4A", "AAC", "OGG", "Opus", "WAV", "AIFF", "DSF", "DFF"];

const existante = $derived(library ?? $libraryStore.librarySelected);
const section = $derived(ONGLETS_BIBLIOTHEQUE.find((o) => o.key === cleSection && o.key !== "playlists") ?? null);

let nom = $state("");
let erreur = $state<string | null>(null);
let enCours = $state(false);

const imp = $derived($importProgressStore);

async function bibliotheque(): Promise<number | null> {
  if (existante?.id) return existante.id;
  const profilId = $profilSelector.profilSelected?.id;
  if (!profilId) {
    erreur = $t("import_page.error");
    return null;
  }
  try {
    await libraryStore.addLibrary(profilId, nom.trim() || $t("import_page.name_default"), null);
  } catch (e) {
    const m = messageErreur(e);
    erreur = m.includes("UNIQUE") || m.includes("duplicate") ? $t("import_page.name_taken") : $t("import_page.error");
    return null;
  }
  return $libraryStore.librarySelected?.id ?? null;
}

async function importer(type: "dossier" | "fichiers") {
  if (enCours) return;
  erreur = null;
  enCours = true;
  try {
    const dossier = type === "dossier" ? await choisirDossier() : null;
    const fichiers = type === "fichiers" ? await choisirFichiers() : [];
    if (!dossier && fichiers.length === 0) return;

    const id = await bibliotheque();
    if (!id) return;
    // Dans une bibliothèque vide, on reste sur l'onglet : la progression s'y affiche.
    const rediriger = !library;
    if (rediriger && section) memoriserOnglet(id, section.key);

    if (dossier) handleAddDirectory(id, rediriger, dossier);
    else handleAddFiles(id, rediriger, fichiers);
  } finally {
    enCours = false;
  }
}
</script>

<div class="h-full overflow-y-auto overscroll-contain scrollbar-app py-6 px-4 md:px-10 select-none">
  <div class="@container max-w-[1100px] mx-auto flex flex-col gap-6 pb-10">

    <!-- ─── Bandeau ─── -->
    <section class="relative overflow-hidden rounded-[24px] p-8 md:p-12 text-(--rg-tx)">
      <div class="hero-vert absolute inset-0 pointer-events-none" aria-hidden="true"></div>

      <DisqueFiligrane class="absolute -right-24 -top-24 w-[420px] h-[420px] max-md:hidden" />

      <div class="relative flex flex-col gap-4 max-w-2xl">
        <p class="flex items-center gap-2 text-[13px] font-semibold uppercase tracking-[0.08em] text-(--rg-gtx)">
          <Icon icon="material-symbols:library-music-outline-rounded" width="18" />
          {existante ? $t("import_page.kicker_empty") : $t("import_page.kicker_new")}
          {#if existante}
            <span class="normal-case tracking-normal font-medium text-(--rg-tx2)">· {existante.name}</span>
          {/if}
          {#if section && !library}
            <span class="ml-1 normal-case tracking-normal font-medium px-2 py-0.5 rounded-full bg-(--rg-carte)/70 border border-(--rg-gbd) text-(--rg-tx2)">
              {$t("import_page.then").replace("{section}", $t(section.labelKey))}
            </span>
          {/if}
        </p>
        <h1 class="text-[40px] md:text-[48px] font-extrabold tracking-[-0.03em] leading-[1.05]">{$t("import_page.title")}</h1>
        <p class="text-[16px] leading-relaxed text-(--rg-tx2) max-w-xl">{$t("import_page.subtitle")}</p>

        {#if !existante}
          <label class="flex flex-col gap-1.5 mt-2 max-w-sm">
            <span class="text-xs font-semibold text-(--rg-mu)">{$t("import_page.name_label")}</span>
            <input
              bind:value={nom}
              placeholder={$t("import_page.name_default")}
              class="select-text h-11 px-3.5 rounded-xl text-[15px] font-semibold outline-none transition-colors
                     bg-(--rg-carte)/80 border border-(--rg-bd2) text-(--rg-tx) placeholder:text-(--rg-ph)
                     focus:border-(--rg-g)"
              oninput={() => (erreur = null)}
            />
          </label>
        {/if}
        {#if erreur}
          <p class="flex items-center gap-1.5 text-sm font-semibold text-red-500">
            <Icon icon="material-symbols:error-outline-rounded" width="18" />{erreur}
          </p>
        {/if}
      </div>
    </section>

    <!-- ─── Import déjà lancé ─── -->
    {#if imp.active && imp.libraryId}
      <button
        type="button"
        class="flex items-center gap-3 p-4 rounded-2xl cursor-pointer text-left transition-colors
               bg-(--rg-carte) border border-(--rg-gbd) hover:border-(--rg-g)"
        onclick={() => goto(`/library/${imp.libraryId}`)}
      >
        <Icon icon="lucide:loader-2" width="20" class="animate-spin text-(--rg-g) shrink-0" />
        <span class="flex-1 min-w-0">
          <b class="block text-sm text-(--rg-tx)">{$t("import_page.running")}</b>
          <span class="block text-xs font-mono truncate text-(--rg-mu)">{imp.directory}</span>
        </span>
        <span class="text-sm font-semibold text-(--rg-g)">{$t("import_page.running_see")}</span>
      </button>
    {/if}

    <!-- ─── Les deux façons d'importer ─── -->
    <div class="grid grid-cols-1 @3xl:grid-cols-[1.4fr_1fr] gap-4">
      <button
        type="button"
        disabled={enCours}
        class="carte group relative overflow-hidden flex flex-col gap-4 p-7 rounded-[20px] text-left cursor-pointer
               bg-(--rg-g) text-(--rg-on-g) disabled:opacity-60 disabled:cursor-default"
        onclick={() => importer("dossier")}
      >
        <span class="absolute top-5 right-5 px-2.5 py-1 rounded-full text-[11px] font-bold uppercase tracking-wider bg-black/15">
          {$t("import_page.recommended")}
        </span>
        <span class="w-14 h-14 rounded-2xl flex items-center justify-center bg-black/12">
          <Icon icon="material-symbols:create-new-folder-outline-rounded" width="30" />
        </span>
        <span class="flex flex-col gap-1.5">
          <b class="text-[22px] font-extrabold tracking-[-0.015em]">{$t("import_page.folder_title")}</b>
          <span class="text-[14px] leading-relaxed opacity-80 max-w-md">{$t("import_page.folder_desc")}</span>
        </span>
        <Icon icon="material-symbols:arrow-forward-rounded" width="26"
              class="absolute bottom-6 right-6 transition-transform group-hover:translate-x-1" />
      </button>

      <button
        type="button"
        disabled={enCours}
        class="carte group relative flex flex-col gap-4 p-7 rounded-[20px] text-left cursor-pointer
               bg-(--rg-carte) border border-(--rg-bd2) text-(--rg-tx) hover:border-(--rg-g)
               disabled:opacity-60 disabled:cursor-default"
        onclick={() => importer("fichiers")}
      >
        <span class="w-14 h-14 rounded-2xl flex items-center justify-center bg-(--rg-gbg) text-(--rg-g)">
          <Icon icon="material-symbols:audio-file-outline-rounded" width="30" />
        </span>
        <span class="flex flex-col gap-1.5">
          <b class="text-[22px] font-extrabold tracking-[-0.015em]">{$t("import_page.files_title")}</b>
          <span class="text-[14px] leading-relaxed text-(--rg-mu) max-w-sm">{$t("import_page.files_desc")}</span>
        </span>
        <Icon icon="material-symbols:arrow-forward-rounded" width="26"
              class="absolute bottom-6 right-6 text-(--rg-mu) transition-transform group-hover:translate-x-1 group-hover:text-(--rg-g)" />
      </button>
    </div>

    <!-- ─── Ce qui se passe ensuite ─── -->
    <div class="grid grid-cols-1 @3xl:grid-cols-3 gap-3">
      {#each [
        { icone: "material-symbols:sell-outline-rounded", texte: $t("import_page.feat_tags") },
        { icone: "material-symbols:monitoring-rounded", texte: $t("import_page.feat_live") },
        { icone: "material-symbols:stop-circle-outline-rounded", texte: $t("import_page.feat_stop") },
      ] as point (point.icone)}
        <div class="flex items-center gap-3 p-4 rounded-2xl bg-(--rg-creux) border border-(--rg-line)">
          <span class="w-9 h-9 shrink-0 rounded-xl flex items-center justify-center bg-(--rg-gbg) text-(--rg-g)">
            <Icon icon={point.icone} width="20" />
          </span>
          <span class="text-[13.5px] leading-snug text-(--rg-tx2)">{point.texte}</span>
        </div>
      {/each}
    </div>

    <div class="flex flex-wrap items-center gap-2">
      <span class="text-xs font-semibold uppercase tracking-[0.08em] text-(--rg-mu2) mr-1">{$t("import_page.formats")}</span>
      {#each FORMATS as f (f)}
        <span class="px-2 py-0.5 rounded-md font-mono text-[11px] font-semibold bg-(--rg-s2) text-(--rg-tx2)">{f}</span>
      {/each}
    </div>
  </div>
</div>

<style>
  .carte {
    transition: transform 160ms ease, box-shadow 160ms ease, border-color 160ms ease;
  }
  .carte:not(:disabled):hover {
    transform: translateY(-2px);
    box-shadow: 0 14px 34px -16px rgba(0, 0, 0, 0.45);
  }
  @media (prefers-reduced-motion: reduce) {
    .carte:not(:disabled):hover { transform: none; }
  }
</style>
