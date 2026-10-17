<script lang="ts">
import Icon from "@iconify/svelte";
import { invoke } from "@tauri-apps/api/core";
import { goto } from "$app/navigation";
import { t, currentLocale } from "#lib/i18n";
import { oublierLocalisations } from "#lib/helper/library/trackLocation";
import { depuisCourt } from "#lib/helper/tools/dateTools";
import { libraryStore } from "#lib/stores/library/library.store";
import { libraryContentStore } from "#lib/stores/library/libraryContent.store";
import { profilSelector } from "#lib/stores/profil/profil.store";
import { profilPopinStore } from "#lib/stores/profil/profilPopin.store";
import { taskProgressStore } from "#lib/stores/ui/taskProgress.store";
import { sidebarStore } from "#lib/stores/ui/sidebar.store";
import { toasts } from "#lib/stores/ui/toast.store";
import LibraryFoldersPopin from "#lib/components/library/common/popin/LibraryFoldersPopin.svelte";
import { settingsStore } from "#lib/stores/settings/settings.store";
import { slide } from "svelte/transition";

let { replie = false }: { replie?: boolean } = $props();

const library = $derived($libraryStore.librarySelected);
const profil = $derived($profilSelector.profilSelected);
const couleur = $derived(profil?.color ?? "#22c55e");

let rescanEnCours = $state(false);
let dossiersOuverts = $state(false);
let dernierScan = $state<string | null>(null);
let maintenant = $state(Date.now());

const tache = $derived(
  Object.values($taskProgressStore).find((t) => t.active && (t.id === "rescan" || t.id === "import")),
);
const enScan = $derived(rescanEnCours || !!tache);
const pourcent = $derived(tache && tache.total > 0 ? Math.min(100, (tache.current * 100) / tache.total) : null);

const compte = $derived(
  tache && tache.total > 0
    ? `${tache.current.toLocaleString($currentLocale)} / ${tache.total.toLocaleString($currentLocale)}`
    : "",
);

const statut = $derived.by(() => {
  if (enScan) {
    if (pourcent === null) return $t("sidebar.scanning");
    const p = new Intl.NumberFormat($currentLocale, { style: "percent" }).format(Math.floor(pourcent) / 100);
    return `${$t("sidebar.scanning")} ${p}`;
  }
  return dernierScan
    ? $t("sidebar.scanned").replace("{ago}", depuisCourt(dernierScan, maintenant, $t))
    : $t("sidebar.never_scanned");
});

// Relu à chaque fin de scan ; l'horloge rafraîchit « il y a 3 min ».
$effect(() => {
  const id = library?.id;
  if (id == null || enScan) return;
  invoke<string | null>("get_library_last_scan", { libraryId: id })
    .then((d) => { if ($libraryStore.librarySelected?.id === id) dernierScan = d; })
    .catch(() => { dernierScan = null; });
});
$effect(() => {
  const minuterie = setInterval(() => maintenant = Date.now(), 30_000);
  return () => clearInterval(minuterie);
});

async function rescanner() {
  const id = library?.id;
  if (id == null || enScan) return;
  rescanEnCours = true;
  try {
    await invoke("rescan_library", { libraryId: id });
    // Un scan renumérote : les fiches mises en cache peuvent ne plus exister.
    oublierLocalisations();
    libraryContentStore.refresh();
    libraryStore.refresh();
  } catch (e) {
    toasts.push({ type: "error", title: $t("sidebar.rescan"), message: e === 'deja_en_cours' ? $t("notify.import_busy") : String(e) });
  } finally {
    rescanEnCours = false;
  }
}

function ouvrirProfil() {
  profilPopinStore.open();
}

// Texte sombre sur une couleur claire, blanc sinon.
const texteAvatar = $derived.by(() => {
  const m = /^#?([0-9a-f]{6})$/i.exec(couleur);
  if (!m) return "#fff";
  const n = parseInt(m[1], 16);
  const lum = (0.299 * (n >> 16) + 0.587 * ((n >> 8) & 255) + 0.114 * (n & 255)) / 255;
  return lum > 0.55 ? "#04140a" : "#fff";
});

const reduit = $derived($settingsStore.sidebar_footer_collapsed === "true");
function basculerReduit() {
  settingsStore.set("sidebar_footer_collapsed", reduit ? "false" : "true");
}

const titreProfil = $derived(`${profil?.name ?? ""} — ${$t("sidebar.profile_active")}`);
const titreRescan = $derived(enScan ? `${statut} ${compte}`.trim() : `${$t("sidebar.rescan_title")} — ${statut}`);
</script>

{#snippet avatar()}
  <span
    class="sb-avatar w-8.5 h-8.5 rounded-full shrink-0 overflow-hidden flex items-center justify-center text-sm font-extrabold"
    style="--profil: {couleur}; color: {texteAvatar};"
  >
    {#if profil?.avatar}
      <img src={profil.avatar} alt="" class="w-full h-full object-cover" draggable="false" />
    {:else}
      {profil?.name?.charAt(0)?.toUpperCase() ?? "?"}
    {/if}
  </span>
{/snippet}

<!-- `grand` : bouton de la colonne repliée ; sinon, bouton de la pilule. -->
{#snippet bouton(icone: string, label: string, action: () => void, grand: boolean, desactive = false, classeIcone = "")}
  <button
    type="button"
    class="relative shrink-0 flex items-center justify-center cursor-pointer transition-colors
           text-(--sb-mu) enabled:hover:bg-(--sb-s2) enabled:hover:text-(--sb-tx) disabled:cursor-default
           {grand ? 'w-12 h-12 rounded-lg' : 'w-7 h-7 rounded-md text-(--sb-tx2)'}"
    onclick={action}
    disabled={desactive}
    title={label}
    aria-label={label}
  >
    <Icon icon={icone} width={grand ? 22 : 17} class={classeIcone} />
  </button>
{/snippet}

{#snippet dossiers(grand: boolean)}
  {@render bouton("material-symbols:folder-open-outline-rounded", $t("sidebar.library_folders"), () => dossiersOuverts = true, grand)}
{/snippet}

{#snippet rescan(grand: boolean)}
  {@render bouton("material-symbols:refresh-rounded", titreRescan, rescanner, grand, enScan,
    enScan ? "animate-spin text-amber-400" : "")}
{/snippet}

{#if replie}
  <div class="shrink-0 w-full flex flex-col items-center gap-1 pt-2.5 border-t border-(--sb-sep)">
    {#if library}
      {@render dossiers(true)}
      <span class="relative">
        {@render rescan(true)}
        <span class="absolute top-2.5 right-2.5 w-2 h-2 rounded-full ring-2 ring-(--sb-bg) pointer-events-none
                     {enScan ? 'bg-amber-400 animate-pulse' : 'bg-(--sb-point)'}"></span>
      </span>
    {/if}
    <button type="button" class="mt-1 cursor-pointer rounded-full" onclick={ouvrirProfil} title={titreProfil}>
      {@render avatar()}
    </button>
  </div>
{:else}
  <!-- Languette : un onglet posé sur le bord de la carte, qui la réduit à une ligne.
       Elle chevauche le bas de la liste ; autour d'elle, la liste reste visible et cliquable. -->
  <div class="shrink-0 -mt-7 relative z-10 flex flex-col items-center pointer-events-none">
  <button
    type="button"
    class="pointer-events-auto relative z-10 -mb-px w-10 h-4 flex items-center justify-center rounded-t-lg cursor-pointer
           bg-(--sb-pied) border border-b-0 border-(--sb-piedbd) text-(--sb-mu) hover:text-(--sb-tx) transition-colors"
    onclick={basculerReduit}
    title={reduit ? $t("sidebar.footer_expand") : $t("sidebar.footer_collapse")}
    aria-label={reduit ? $t("sidebar.footer_expand") : $t("sidebar.footer_collapse")}
    aria-expanded={!reduit}
  >
    <Icon icon={reduit ? "material-symbols:keyboard-arrow-up-rounded" : "material-symbols:keyboard-arrow-down-rounded"} width="16" />
  </button>

  <!-- Grille de trois colonnes : avatar et point, nom et état, boutons. Chaque
       ligne porte ses actions : le profil ⚙, la synchronisation 📁 ⟳. -->
  <div
    class="sb-pied-carte pointer-events-auto relative overflow-hidden w-full flex flex-col rounded-[14px] border border-(--sb-piedbd)"
    style="--profil: {couleur};"
  >
    {#if reduit}
      <!-- Réduit : une ligne, qui se redéplie au clic. -->
      <button
        type="button"
        class="flex items-center gap-2 py-1.5 pl-2 pr-2.5 min-w-0 text-left cursor-pointer"
        onclick={basculerReduit}
        title={$t("sidebar.footer_expand")}
        transition:slide={{ duration: 180 }}
      >
        <span class="sb-avatar w-6 h-6 rounded-full shrink-0 overflow-hidden flex items-center justify-center text-[11px] font-extrabold"
              style="--profil: {couleur}; color: {texteAvatar};">
          {#if profil?.avatar}
            <img src={profil.avatar} alt="" class="w-full h-full object-cover" draggable="false" />
          {:else}
            {profil?.name?.charAt(0)?.toUpperCase() ?? "?"}
          {/if}
        </span>
        <b class="text-[13px] font-bold truncate text-(--sb-tx) shrink-0 max-w-[40%]">{profil?.name ?? "—"}</b>
        {#if library}
          <span class="sb-point w-1.5 h-1.5 rounded-full shrink-0 {enScan ? 'text-amber-400 animate-pulse' : 'text-(--sb-point)'}"></span>
          <small class="min-w-0 text-xs truncate tabular-nums text-(--sb-mu)">{statut}</small>
        {/if}
      </button>
    {:else}
    <div transition:slide={{ duration: 180 }}>
    <div class="flex items-center gap-2.5 pt-2 pb-1.5 pl-2 pr-1.5">
      <button type="button" class="group flex items-center gap-2.5 min-w-0 flex-1 text-left cursor-pointer"
              onclick={ouvrirProfil} title={titreProfil}>
        {@render avatar()}
        <span class="flex flex-col gap-px min-w-0">
          <b class="text-sm font-bold truncate text-(--sb-tx) group-hover:underline underline-offset-2 decoration-(--sb-mu)">
            {profil?.name ?? "—"}
          </b>
          <small class="text-xs truncate text-(--sb-mu)">{$t("sidebar.profile_active")}</small>
        </span>
      </button>
      {@render bouton("material-symbols:settings-outline-rounded", $t("sidebar.settings"),
        () => { goto("/settings"); sidebarStore.close(); }, false)}
    </div>

    {#if library}
      <div class="flex items-center gap-2.5 py-1 pl-2 pr-1.5 border-t border-(--sb-piedbd)">
        <span class="w-8.5 shrink-0 flex justify-center">
          <span class="sb-point w-1.5 h-1.5 rounded-full {enScan ? 'text-amber-400 animate-pulse' : 'text-(--sb-point)'}"></span>
        </span>
        <small class="flex-1 min-w-0 text-xs truncate tabular-nums text-(--sb-mu)">{statut}</small>
        <span class="flex items-center gap-0.5 shrink-0">
          {@render dossiers(false)}
          {@render rescan(false)}
        </span>
      </div>
    {/if}
    </div>
    {/if}

    {#if enScan && pourcent !== null}
      <span class="absolute left-0 bottom-0 h-0.5 bg-amber-400 transition-[width] duration-300" style="width: {pourcent}%;"></span>
    {/if}
  </div>
  </div>
{/if}

{#if dossiersOuverts && library?.id != null}
  <LibraryFoldersPopin bind:open={dossiersOuverts} libraryId={library.id} />
{/if}
