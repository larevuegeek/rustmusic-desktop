<script lang="ts">
  import { onMount } from "svelte";
  import Icon from "@iconify/svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { toasts } from "#lib/stores/ui/toast.store";
  import { libraryContentStore } from "#lib/stores/library/libraryContent.store";
  import { fade, scale } from "svelte/transition";
  import { t } from "#lib/i18n";

  type DeezerCoverResult = {
    title: string;
    artist: string;
    cover_small: string;
    cover_xl: string;
  };

  let {
    albumId,
    initialQuery = "",
    onclose,
    oncover,
  }: {
    albumId: string;
    initialQuery?: string;
    onclose: () => void;
    oncover?: () => void;
  } = $props();

  // svelte-ignore state_referenced_locally
  let query = $state(initialQuery);
  let results = $state<DeezerCoverResult[]>([]);
  let loading = $state(false);
  let applying = $state<string | null>(null);
  let selected = $state<DeezerCoverResult | null>(null);

  // Seule la dernière recherche lancée s'affiche.
  let demande = 0;
  async function search() {
    if (!query.trim()) return;
    const n = ++demande;
    loading = true;
    results = [];
    selected = null;
    try {
      const r = await invoke<DeezerCoverResult[]>('search_deezer_covers', {
        query: query.trim(),
        limit: 12,
      });
      if (n === demande) results = r;
    } catch (e) {
      if (n === demande) toasts.push({ type: "error", title: $t("popins.error_title"), message: String(e) });
    } finally {
      if (n === demande) loading = false;
    }
  }

  async function applyCover() {
    if (!selected) return;
    applying = selected.cover_xl;
    try {
      await invoke('apply_deezer_cover', {
        albumId,
        coverUrl: selected.cover_xl,
      });
      await libraryContentStore.refresh();
      oncover?.();
      toasts.push({ type: "success", title: $t("tags.cover"), message: $t("popins.cover_applied").replace("{title}", selected.title) });
      onclose();
    } catch (e) {
      toasts.push({ type: "error", title: $t("popins.error_title"), message: String(e) });
    } finally {
      applying = null;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') onclose();
    if (e.key === 'Enter' && !selected) search();
  }

  // Une recherche à l'ouverture ; ensuite, Entrée ou le bouton (pas une requête par frappe).
  onMount(() => {
    if (query.trim()) search();
  });
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- Backdrop -->
<button
  type="button"
  class="fixed inset-0 z-[9999] bg-black/60 backdrop-blur-sm cursor-default"
  onclick={onclose}
  aria-label={$t("common.close")}
  transition:fade={{ duration: 150 }}
></button>

<!-- Modal : même habillage que Popin -->
<div
  class="fixed inset-0 z-[10000] flex items-center justify-center p-6 pointer-events-none"
>
  <div
    class="relative w-full max-w-2xl h-[75vh] pointer-events-auto
           flex flex-col overflow-hidden rounded-2xl text-(--rg-tx)
           bg-(--rg-carte) border border-(--rg-bd) dark:border-[#2a312d]
           shadow-[0_24px_60px_-12px_rgba(0,0,0,0.25)] dark:shadow-[0_30px_80px_-20px_rgba(0,0,0,0.9)]"
    transition:scale={{ duration: 180, start: 0.96 }}
  >

    <!-- Header -->
    <div class="shrink-0 px-6 pt-5 pb-4 border-b border-(--rg-bd)
                bg-linear-135 from-(--rg-g)/8 to-transparent to-60%">
      <div class="flex items-center justify-between gap-3">
        <div class="flex items-center gap-2.5 min-w-0">
          <span class="shrink-0 w-8 h-8 rounded-[9px] flex items-center justify-center
                       bg-(--rg-gbg) text-(--rg-g) border border-(--rg-gbd)">
            <Icon icon="lucide:disc-album" width="15" />
          </span>
          <div class="min-w-0">
            <h2 class="truncate text-[17px] font-extrabold tracking-[-0.01em] text-(--rg-tx)">{$t("popins.cover_search_title")}</h2>
            <p class="text-[11px] text-(--rg-mu)">{$t("popins.cover_search_source")}</p>
          </div>
        </div>

        <button
          type="button"
          class="shrink-0 -mr-1.5 w-8 h-8 rounded-[9px] flex items-center justify-center cursor-pointer
                 text-(--rg-mu) hover:text-(--rg-tx) hover:bg-(--rg-hover) transition-colors"
          onclick={onclose}
          aria-label={$t("common.close")}
          title={$t("common.close")}
        >
          <Icon icon="lucide:x" width={15} />
        </button>
      </div>
    </div>

    <!-- Search bar -->
    <div class="shrink-0 flex items-center gap-2 px-6 py-4 border-b border-(--rg-bd) bg-(--rg-creux)">
      <div class="relative flex-1">
        <Icon icon="lucide:search" width={16}
              class="absolute left-3.5 top-1/2 -translate-y-1/2 text-(--rg-mu) pointer-events-none" />
        <input
          type="text"
          bind:value={query}
          placeholder={$t("popins.cover_search_placeholder")}
          class="w-full h-11 pl-10 pr-4 text-sm rounded-xl outline-none transition-colors
                 bg-(--rg-champ) border border-(--rg-bd) text-(--rg-tx) placeholder:text-(--rg-ph)
                 hover:border-(--rg-bd2) rg-field"
        />
      </div>
      <button
        type="button"
        class="h-11 min-w-24 px-4 rounded-xl border text-sm font-bold cursor-pointer
               flex items-center justify-center transition-colors
               border-(--rg-gbd) bg-(--rg-gbg) text-(--rg-gtx) hover:border-(--rg-g)
               disabled:opacity-50 disabled:cursor-default"
        onclick={search}
        disabled={loading || !query.trim()}
      >
        {#if loading}
          <Icon icon="lucide:loader-2" width={16} class="animate-spin" />
        {:else}
          {$t("settings.shortcut_search")}
        {/if}
      </button>
    </div>

    <!-- Results -->
    <div class="flex-1 overflow-y-auto scrollbar-app px-6 py-5 min-h-0">
      {#if loading}
        <div class="flex items-center justify-center h-full">
          <div class="flex flex-col items-center gap-3">
            <Icon icon="lucide:loader-2" width={24} class="animate-spin text-(--rg-g)" />
            <p class="text-[13px] text-(--rg-mu)">{$t("popins.searching")}</p>
          </div>
        </div>
      {:else if results.length === 0}
        <div class="flex flex-col items-center justify-center h-full text-center">
          <span class="w-16 h-16 rounded-2xl border border-(--rg-bd) bg-(--rg-creux)
                       flex items-center justify-center mb-4 text-(--rg-mu2)">
            <Icon icon="lucide:disc-album" width={28} />
          </span>
          <p class="text-sm font-semibold text-(--rg-tx2) mb-1">
            {query.trim() ? $t("search.no_result") : $t("popins.cover_search_empty")}
          </p>
          <p class="text-xs text-(--rg-mu)">
            {query.trim() ? $t("search.no_result_desc") : $t("popins.cover_search_empty_hint")}
          </p>
        </div>
      {:else}
        <div class="grid grid-cols-3 sm:grid-cols-4 gap-3">
          {#each results as result}
            {@const isSelected = selected?.cover_xl === result.cover_xl}
            <button
              type="button"
              class="group flex flex-col rounded-2xl overflow-hidden border text-left cursor-pointer
                     transition-colors duration-200
                     {isSelected
                       ? 'border-(--rg-g) bg-(--rg-creux-on) shadow-[0_0_0_1px_var(--rg-g)]'
                       : 'border-(--rg-bd) bg-(--rg-creux) hover:border-(--rg-bd2)'}"
              onclick={() => selected = isSelected ? null : result}
              disabled={applying !== null}
            >
              <!-- Cover -->
              <div class="aspect-square relative overflow-hidden">
                <img
                  src={result.cover_small}
                  alt={result.title}
                  class="w-full h-full object-cover
                         {isSelected ? 'scale-105' : 'group-hover:scale-105'}
                         transition-transform duration-300"
                />
                {#if isSelected}
                  <div class="absolute inset-0 bg-(--rg-g)/10 flex items-center justify-center">
                    <div class="w-8 h-8 rounded-full bg-(--rg-g) text-(--rg-on-g) flex items-center justify-center shadow-lg">
                      <Icon icon="material-symbols:check-rounded" width={18} />
                    </div>
                  </div>
                {/if}
                {#if applying === result.cover_xl}
                  <div class="absolute inset-0 bg-black/50 flex items-center justify-center">
                    <Icon icon="lucide:loader-2" width={20} class="text-white animate-spin" />
                  </div>
                {/if}
              </div>

              <!-- Info -->
              <div class="px-3 py-2.5 min-w-0">
                <p class="text-[12px] font-semibold truncate
                          {isSelected ? 'text-(--rg-gtx)' : 'text-(--rg-tx)'}">
                  {result.title}
                </p>
                <p class="mt-0.5 text-[11px] text-(--rg-mu) truncate">{result.artist}</p>
              </div>
            </button>
          {/each}
        </div>
      {/if}
    </div>

    <!-- Footer -->
    {#if selected}
      <div class="shrink-0 flex items-center justify-between gap-3 px-6 py-3.5
                  border-t border-(--rg-bd) bg-(--rg-creux)">
        <div class="flex items-center gap-3 min-w-0">
          <img src={selected.cover_small} alt=""
               class="w-10 h-10 rounded-lg object-cover border border-(--rg-bd)" />
          <div class="min-w-0">
            <p class="text-[13px] font-semibold text-(--rg-tx) truncate">{selected.title}</p>
            <p class="text-[11px] text-(--rg-mu) truncate">{selected.artist}</p>
          </div>
        </div>

        <button
          type="button"
          class="shrink-0 h-10 pl-4 pr-5 rounded-xl text-sm font-bold cursor-pointer flex items-center gap-2
                 bg-(--rg-g) text-(--rg-on-g) hover:brightness-110 transition
                 disabled:opacity-40 disabled:cursor-default"
          onclick={applyCover}
          disabled={applying !== null}
        >
          {#if applying}
            <Icon icon="lucide:loader-2" width={17} class="animate-spin" />
            {$t("popins.applying")}
          {:else}
            <Icon icon="material-symbols:check-rounded" width={17} />
            {$t("popins.cover_apply")}
          {/if}
        </button>
      </div>
    {/if}
  </div>
</div>
