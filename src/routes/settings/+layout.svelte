<script module lang="ts">
  // Page d'où l'on est venu : le bouton retour y ramène, pas à la section précédente.
  let retour = "/";
</script>

<script lang="ts">
  // Réglages : navigation à gauche (recherche, sections, version), section active à droite.
  import Icon from "@iconify/svelte";
  import { page } from "$app/state";
  import { afterNavigate, goto } from "$app/navigation";
  import { fade } from "svelte/transition";
  import { t } from "$lib/i18n";
  import { settingsSections } from "$lib/config/settingsSections";
  import { settingsDefaults, settingsStore } from "$lib/stores/settings/settings.store";
  import { rechercheReglages } from "$lib/stores/ui/settingsSearch.store";
  import VersionCard from "$lib/components/settings/VersionCard.svelte";
  import type { Snippet } from "svelte";

  let { children }: { children: Snippet } = $props();

  afterNavigate(({ from }) => {
    if (from && !from.url.pathname.startsWith("/settings")) {
      retour = from.url.pathname + from.url.search;
      rechercheReglages.set("");
    }
  });

  const actif = $derived(page.url.pathname.split("/")[2] ?? "general");
  const section = $derived(settingsSections.find((s) => s.id === actif) ?? settingsSections[0]);

  const modifiees = $derived((section.cles ?? []).filter((cle) => $settingsStore[cle] !== settingsDefaults[cle]));

  async function retablir() {
    for (const cle of modifiees) await settingsStore.set(cle, settingsDefaults[cle]);
  }
</script>

<div class="reglages flex h-full">
  <aside class="shrink-0 w-[300px] max-lg:w-[264px] flex flex-col gap-3.5 px-4 pt-5 pb-4
                bg-(--rg-aside) border-r border-(--rg-aside-bd) overflow-y-auto overflow-x-hidden scrollbar-app">
    <div class="shrink-0 flex items-center gap-3">
      <button
        type="button"
        class="w-10 h-10 flex items-center justify-center rounded-xl cursor-pointer transition-colors
               bg-(--rg-champ) border border-(--rg-bd) hover:border-(--rg-bd2) text-(--rg-tx)"
        onclick={() => goto(retour)}
        aria-label={$t("settings.back")}
        title={$t("settings.back")}
      >
        <Icon icon="material-symbols:arrow-back-rounded" width="20" class="sb-icone" />
      </button>
      <h1 class="text-xl font-extrabold text-(--rg-tx)">{$t("settings.title")}</h1>
    </div>

    <label
      class="shrink-0 h-10 flex items-center gap-2.5 px-3 rounded-[10px] cursor-text
             bg-(--rg-champ) border border-(--rg-bd) focus-within:border-(--rg-bd2)"
      data-focus-ring="row"
    >
      <Icon icon="material-symbols:search-rounded" width="19" class="shrink-0 text-(--rg-mu)" />
      <input
        type="search"
        bind:value={$rechercheReglages}
        placeholder={$t("settings.search_placeholder")}
        aria-label={$t("settings.search_placeholder")}
        data-focus-ring="none"
        class="flex-1 min-w-0 bg-transparent outline-none text-sm text-(--rg-tx) placeholder:text-(--rg-ph)
               [&::-webkit-search-cancel-button]:hidden"
      />
      {#if $rechercheReglages}
        <button
          type="button"
          class="shrink-0 flex text-(--rg-mu) hover:text-(--rg-tx) cursor-pointer"
          onclick={() => rechercheReglages.set("")}
          aria-label={$t("settings.search_clear")}
        >
          <Icon icon="material-symbols:close-rounded" width="18" />
        </button>
      {/if}
    </label>

    <nav class="shrink-0 flex flex-col gap-0.5">
      {#each settingsSections as s (s.id)}
        {@const estActif = s.id === actif}
        <a
          href="/settings/{s.id}"
          class="flex items-center gap-3 px-3 py-2.25 rounded-[10px] leading-[1.2] transition-colors
                 {estActif ? 'bg-(--rg-gbg)' : 'hover:bg-(--rg-champ)'}"
          aria-current={estActif ? "page" : undefined}
        >
          <Icon icon={s.icon} width="21" class="shrink-0 sb-icone {estActif ? 'text-(--rg-gtx)' : 'text-(--rg-mu)'}" />
          <span class="min-w-0">
            <span class="block text-[15px] font-semibold {estActif ? 'text-(--rg-gtx)' : 'text-(--rg-nav)'}">{$t(s.labelKey)}</span>
            <span class="rg-nav-sous block text-xs text-(--rg-mu) truncate">{$t(s.hintKey)}</span>
          </span>
        </a>
      {/each}
    </nav>

    <VersionCard />
  </aside>

  <div class="flex-1 min-w-0 overflow-y-auto overflow-x-hidden scrollbar-app">
    {#key actif}
      <div class="rg-contenu max-w-[820px] mx-auto px-10 max-md:px-6 pt-10 pb-15 flex flex-col gap-7" in:fade={{ duration: 120 }}>
        <header class="flex items-end justify-between gap-5">
          <div class="min-w-0">
            <h1 class="text-[34px] leading-[1.2] font-extrabold tracking-[-0.02em] text-(--rg-tx)">{$t(section.labelKey)}</h1>
            <p class="mt-1.5 text-base leading-[1.25] text-(--rg-sous)">{$t(section.descKey)}</p>
          </div>
          {#if modifiees.length}
            <div class="shrink-0 flex items-center gap-2.5 text-[13px] leading-[1.2] text-(--rg-sous)">
              <span>
                <b class="text-(--rg-gtx)">{modifiees.length}</b>
                {$t(modifiees.length > 1 ? "settings.modified_many" : "settings.modified_one")}
              </span>
              <button
                type="button"
                class="h-7.5 px-3 rounded-lg border border-(--rg-off) hover:border-(--rg-bd2) cursor-pointer
                       text-[13px] font-semibold text-(--rg-tx2) hover:text-(--rg-tx) transition-colors"
                title={$t("settings.restore_defaults_hint")}
                onclick={retablir}
              >
                {$t("settings.restore_defaults")}
              </button>
            </div>
          {/if}
        </header>

        {@render children()}

        <div class="rg-aucun flex-col items-center gap-2 py-16 text-center">
          <Icon icon="material-symbols:search-off-rounded" width="32" class="text-(--rg-mu2)" />
          <p class="text-[15px] font-semibold text-(--rg-tx)">
            {$t("settings.search_empty").replace("{query}", $rechercheReglages.trim())}
          </p>
          <p class="text-[13px] text-(--rg-mu)">{$t("settings.search_empty_hint")}</p>
        </div>
      </div>
    {/key}
  </div>
</div>
