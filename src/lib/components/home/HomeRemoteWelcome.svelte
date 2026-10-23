<script lang="ts">
// Accueil d'une bibliothèque distante encore vide : la synchronisation avec le serveur, pas d'import.
import Icon from "@iconify/svelte";
import { goto } from "$app/navigation";
import { invoke } from "@tauri-apps/api/core";
import { t, currentLocale } from "#lib/i18n";
import DiscWatermark from "#lib/components/ui/deco/DiscWatermark.svelte";
import StatusDot from "#lib/components/ui/status/StatusDot.svelte";
import { importProgressStore as importProgress } from "#lib/stores/library/importProgress.store";
import { libraryStore } from "#lib/stores/library/library.store";
import { serverStore } from "#lib/stores/mediaServer/mediaServer.store";
import { toasts } from "#lib/stores/ui/toast.store";
import type { Library } from "#lib/types/db/library/Library";

let { library }: { library: Library } = $props();

const serverId = $derived(library.server_id as number);
const online = $derived($serverStore.online[serverId]);
const sync = $derived($importProgress.active && $importProgress.libraryId === library.id ? $importProgress : null);
const nombre = (n: number) => n.toLocaleString($currentLocale);
// « 120 / 501 morceaux · 24 % », ou les morceaux trouvés pendant le listage.
const avancement = $derived(
  !sync ? null
    : sync.phase === "listing" ? $t("servers.home_listing").replace("{n}", nombre(sync.found))
    : $t("servers.home_progress").replace("{done}", nombre(sync.current)).replace("{total}", nombre(sync.total)).replace("{percent}", String(sync.percent)),
);

let lance = $state(false);
async function synchronize() {
  lance = true;
  try {
    await invoke("rescan_library", { libraryId: library.id });
    await libraryStore.refresh();
  } catch (err) {
    if (String(err) !== "deja_en_cours") toasts.push({ type: "error", title: $t("notify.error"), message: String(err) });
  } finally {
    lance = false;
  }
}

// Connexion faite ; synchronisation en cours ou à lancer ; écoute ensuite.
const etapes = $derived([
  { icone: "material-symbols:cloud-done-outline-rounded", titre: "servers.home_step1_title", desc: "servers.home_step1_desc", etat: "fait" },
  { icone: "material-symbols:sync-rounded", titre: "servers.home_step2_title", desc: "servers.home_step2_desc", etat: sync ? "encours" : "attente" },
  { icone: "material-symbols:headphones-outline-rounded", titre: "servers.home_step3_title", desc: "servers.home_step3_desc", etat: "attente" },
]);
</script>

<!-- ─── Bandeau ─── -->
<section class="relative overflow-hidden rounded-3xl p-8 md:p-12 text-(--rg-tx) select-none">
  <div class="hero-ciel absolute inset-0 pointer-events-none" aria-hidden="true"></div>

  <DiscWatermark tone="sky" class="absolute -right-20 -bottom-28 w-110 h-110 max-md:hidden" />

  <div class="relative flex flex-col gap-4 max-w-2xl">
    <p class="flex items-center gap-2 text-[13px] font-semibold uppercase tracking-[0.08em] text-sky-700 dark:text-sky-300">
      <Icon icon="material-symbols:cloud-outline-rounded" width="17" />{$t("servers.home_kicker")}
    </p>
    <h2 class="text-[36px] md:text-[44px] font-extrabold tracking-[-0.03em] leading-[1.05] text-balance">
      {$t("servers.home_title").replace("{name}", library.name)}
    </h2>
    <p class="text-[16px] leading-relaxed text-(--rg-tx2) max-w-xl">{$t(sync ? "servers.home_desc_sync" : "servers.home_desc_idle")}</p>

    <p class="flex items-center gap-2 text-[14px] text-(--rg-tx2)">
      <StatusDot {online} />
      <b class="font-semibold text-(--rg-tx)">{library.server_name}</b>
      <span class="text-(--rg-mu)">· {online === false ? $t("servers.offline") : $t("servers.online")}</span>
    </p>

    {#if sync}
      <!-- Progression de la synchronisation : la même que la barre d'état. -->
      <div class="flex flex-col gap-2 mt-1 max-w-md">
        <div class="h-2 rounded-full overflow-hidden bg-black/10 dark:bg-white/10">
          <div class="h-full rounded-full bg-sky-500 transition-[width] duration-300 {sync.phase === 'listing' ? 'w-1/4 animate-pulse' : ''}"
               style:width={sync.phase === "listing" ? undefined : `${Math.max(2, sync.percent)}%`}></div>
        </div>
        <span class="text-[13px] font-medium tabular-nums text-(--rg-tx2)">{avancement}</span>
      </div>
    {/if}

    <div class="flex flex-wrap items-center gap-3 mt-3">
      {#if !sync}
        <button type="button" onclick={synchronize} disabled={lance || online === false}
                class="cta h-13 px-6.5 rounded-full font-bold text-[17px] flex items-center gap-2.5 cursor-pointer
                       bg-sky-500 text-white disabled:opacity-50 disabled:cursor-not-allowed">
          <Icon icon="material-symbols:sync-rounded" width="22" class={lance ? "animate-spin" : ""} />
          {$t("servers.home_sync")}
        </button>
      {/if}
      <button type="button" onclick={() => goto("/settings/storage")}
              class="h-13 px-5 rounded-full flex items-center gap-2.5 cursor-pointer transition-colors
                     text-(--rg-tx2) hover:text-(--rg-tx) hover:bg-(--rg-carte)/60">
        <Icon icon="material-symbols:settings-outline-rounded" width="22" />
        <span class="flex flex-col items-start leading-tight">
          <b class="text-[14px] font-semibold">{$t("servers.manage")}</b>
          <small class="text-[11.5px] text-(--rg-mu)">{$t("servers.home_manage_hint")}</small>
        </span>
      </button>
    </div>
  </div>
</section>

<!-- ─── Les trois étapes ─── -->
<ol class="grid grid-cols-1 @3xl:grid-cols-3 gap-3 select-none">
  {#each etapes as e, i (e.titre)}
    <li class="relative flex gap-4 p-5 rounded-2xl bg-(--rg-creux) border {e.etat === 'encours' ? 'border-sky-500/40' : 'border-(--rg-line)'}">
      <span class="relative w-11 h-11 shrink-0 rounded-xl flex items-center justify-center
                   {e.etat === 'attente' ? 'bg-(--rg-s2) text-(--rg-mu)' : 'bg-sky-500/12 text-sky-600 dark:text-sky-300'}">
        <Icon icon={e.icone} width="22" class={e.etat === "encours" ? "animate-spin [animation-duration:2.4s]" : ""} />
        <span class="absolute -top-1.5 -left-1.5 w-5 h-5 rounded-full flex items-center justify-center text-[11px] font-extrabold
                     {e.etat === 'attente' ? 'bg-(--rg-s2) text-(--rg-mu) shadow-[0_0_0_1px_var(--rg-bd)]' : 'bg-sky-500 text-white'}">
          {#if e.etat === "fait"}<Icon icon="material-symbols:check-rounded" width="14" />{:else}{i + 1}{/if}
        </span>
      </span>
      <span class="flex flex-col gap-0.5 min-w-0">
        <b class="text-[15px] font-bold text-(--rg-tx)">{$t(e.titre)}</b>
        <span class="text-[13px] leading-snug text-(--rg-mu)">{$t(e.desc)}</span>
      </span>
    </li>
  {/each}
</ol>

<style>
  .cta {
    transition: transform 160ms ease, box-shadow 160ms ease, filter 160ms ease;
  }
  .cta:not(:disabled):hover {
    filter: brightness(1.08);
    transform: translateY(-1px);
    box-shadow: 0 12px 28px -14px rgba(0, 0, 0, 0.5);
  }
  @media (prefers-reduced-motion: reduce) {
    .cta:hover { transform: none; }
  }
</style>
