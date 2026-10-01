<script lang="ts">
  import { onMount } from "svelte";
  import Icon from "@iconify/svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { toasts } from "$lib/stores/ui/toast.store";
  import { libraryContentStore } from "$lib/stores/library/libraryContent.store";
  import { fade, scale } from "svelte/transition";
  import { t } from "$lib/i18n";

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
  class="fixed inset-0 z-[9999] bg-black/80 backdrop-blur-xl cursor-default"
  onclick={onclose}
  aria-label={$t("common.close")}
  transition:fade={{ duration: 150 }}
></button>

<!-- Modal -->
<div
  class="fixed inset-0 z-[10000] flex items-center justify-center p-6 pointer-events-none"
>
  <div
    class="relative w-full max-w-2xl h-[75vh] pointer-events-auto
           bg-neutral-950 border border-white/6
           rounded-3xl
           shadow-[0_24px_80px_rgba(0,0,0,0.6)]
           flex flex-col overflow-hidden"
    transition:scale={{ duration: 200, start: 0.95 }}
  >

    <!-- Glow top -->
    <div class="absolute top-0 left-1/2 -translate-x-1/2 w-80 h-32 rounded-full blur-[80px] opacity-15 pointer-events-none bg-green-500"></div>

    <!-- Header -->
    <div class="relative px-6 pt-5 pb-4">
      <div class="flex items-center justify-between mb-4">
        <div class="flex items-center gap-2.5">
          <div class="w-8 h-8 rounded-xl bg-green-500/10 border border-green-500/15
                      flex items-center justify-center">
            <Icon icon="lucide:disc-album" width={14} class="text-green-400" />
          </div>
          <div>
            <h2 class="text-sm font-bold text-white">{$t("popins.cover_search_title")}</h2>
            <p class="text-[10px] text-neutral-500">{$t("popins.cover_search_source")}</p>
          </div>
        </div>

        <button
          type="button"
          class="w-8 h-8 rounded-xl flex items-center justify-center cursor-pointer
                 bg-white/5 border border-white/6
                 text-neutral-400 hover:text-white hover:bg-white/10
                 transition-all duration-200"
          onclick={onclose}
          aria-label={$t("common.close")}
          title={$t("common.close")}
        >
          <Icon icon="lucide:x" width={14} />
        </button>
      </div>

      <!-- Search bar -->
      <div class="flex items-center gap-2">
        <div class="relative flex-1">
          <Icon icon="lucide:search" width={14}
                class="absolute left-3 top-1/2 -translate-y-1/2 text-neutral-500 pointer-events-none" />
          <input
            type="text"
            bind:value={query}
            placeholder={$t("popins.cover_search_placeholder")}
            class="w-full pl-9 pr-4 py-2.5 text-sm rounded-xl
                   bg-white/4 border border-white/8
                   text-white placeholder-neutral-500
                   outline-none focus:border-green-500/30 focus:ring-1 focus:ring-green-500/15
                   transition-all duration-200"
          />
        </div>
        <button
          type="button"
          class="px-4 py-2.5 rounded-xl text-xs font-semibold cursor-pointer
                 bg-green-500/15 text-green-400 border border-green-500/20
                 hover:bg-green-500/25 hover:border-green-500/30
                 transition-all duration-200 disabled:opacity-50"
          onclick={search}
          disabled={loading || !query.trim()}
        >
          {#if loading}
            <Icon icon="lucide:loader-2" width={13} class="animate-spin" />
          {:else}
            {$t("settings.shortcut_search")}
          {/if}
        </button>
      </div>
    </div>

    <!-- Separator -->
    <div class="h-px bg-linear-to-r from-transparent via-white/8 to-transparent"></div>

    <!-- Results -->
    <div class="flex-1 overflow-y-auto scrollbar-app p-5 min-h-0">
      {#if loading}
        <div class="flex items-center justify-center h-full">
          <div class="flex flex-col items-center gap-3">
            <Icon icon="lucide:loader-2" width={24} class="animate-spin text-green-500/60" />
            <p class="text-xs text-neutral-500">{$t("popins.searching")}</p>
          </div>
        </div>
      {:else if results.length === 0}
        <div class="flex flex-col items-center justify-center h-full text-center">
          <div class="w-16 h-16 rounded-2xl bg-white/3 border border-white/5
                      flex items-center justify-center mb-4">
            <Icon icon="lucide:disc-album" width={28} class="text-neutral-600" />
          </div>
          <p class="text-sm text-neutral-400 mb-1">
            {query.trim() ? $t("search.no_result") : $t("popins.cover_search_empty")}
          </p>
          <p class="text-[11px] text-neutral-600">
            {query.trim() ? $t("search.no_result_desc") : $t("popins.cover_search_empty_hint")}
          </p>
        </div>
      {:else}
        <div class="grid grid-cols-3 sm:grid-cols-4 gap-3">
          {#each results as result}
            {@const isSelected = selected?.cover_xl === result.cover_xl}
            <button
              type="button"
              class="group flex flex-col rounded-xl overflow-hidden cursor-pointer
                     transition-all duration-200
                     {isSelected
                       ? 'ring-2 ring-green-500 bg-green-500/8 scale-[1.02]'
                       : 'bg-white/2 border border-white/5 hover:border-white/15 hover:bg-white/4'}"
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
                  <div class="absolute inset-0 bg-green-500/10 flex items-center justify-center">
                    <div class="w-8 h-8 rounded-full bg-green-500 flex items-center justify-center
                                shadow-lg shadow-green-500/40">
                      <Icon icon="lucide:check" width={14} class="text-black" />
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
              <div class="px-2.5 py-2 min-w-0">
                <p class="text-[11px] font-medium truncate
                          {isSelected ? 'text-green-400' : 'text-neutral-200'}">
                  {result.title}
                </p>
                <p class="text-[10px] text-neutral-500 truncate">{result.artist}</p>
              </div>
            </button>
          {/each}
        </div>
      {/if}
    </div>

    <!-- Footer -->
    {#if selected}
      <div class="px-5 py-4 border-t border-white/6
                  bg-neutral-950/80 backdrop-blur-sm">
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-3 min-w-0">
            <img src={selected.cover_small} alt=""
                 class="w-10 h-10 rounded-lg object-cover shadow-md" />
            <div class="min-w-0">
              <p class="text-xs font-medium text-white truncate">{selected.title}</p>
              <p class="text-[10px] text-neutral-500 truncate">{selected.artist}</p>
            </div>
          </div>

          <button
            type="button"
            class="flex items-center gap-2 px-5 py-2 rounded-xl text-sm font-semibold cursor-pointer
                   bg-green-500 text-black
                   hover:bg-green-400
                   active:scale-[0.97]
                   transition-all duration-150
                   disabled:opacity-50 disabled:cursor-not-allowed"
            onclick={applyCover}
            disabled={applying !== null}
          >
            {#if applying}
              <Icon icon="lucide:loader-2" width={14} class="animate-spin" />
              {$t("popins.applying")}
            {:else}
              <Icon icon="lucide:check" width={14} />
              {$t("popins.cover_apply")}
            {/if}
          </button>
        </div>
      </div>
    {/if}
  </div>
</div>
