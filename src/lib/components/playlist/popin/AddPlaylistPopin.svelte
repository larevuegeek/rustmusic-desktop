<script lang="ts">
  import { popinStore } from "$lib/stores/ui/popin.store";
  import { playlistStore } from "$lib/stores/playlist/playlist.store";
  import { profilSelector } from "$lib/stores/profil/profil.store";
  import { PLAYLIST_COLORS, PLAYLIST_ICONS } from "../playlistConfig";
  import Icon from "@iconify/svelte";
  import type { Playlist } from "$lib/types/db/playlist/Playlist";

  /** Appelé avec la playlist créée — sert à y verser aussitôt un morceau. */
  let { apresCreation }: { apresCreation?: (pl: Playlist) => void } = $props();

  let name = $state("");
  let description = $state("");
  let selectedColorIndex = $state(0);
  let selectedIconIndex = $state(0);
  let isSubmitting = $state(false);
  let errorName: string | null = $state(null);

  let selectedColor = $derived(PLAYLIST_COLORS[selectedColorIndex]);
  let selectedIcon = $derived(PLAYLIST_ICONS[selectedIconIndex]);

  function close() {
    popinStore.close();
  }

  async function submit() {
    errorName = null;
    if (!name.trim()) {
      errorName = "Le nom est obligatoire";
      return;
    }
    isSubmitting = true;
    try {
      const profilId = $profilSelector.profilSelected?.id;
      if (!profilId) throw new Error("NO_PROFIL");

      const creee = await playlistStore.addPlaylist(profilId, name, description || null, selectedColor, selectedIcon.id);
      close();
      apresCreation?.(creee);
    } catch (error: any) {
      const message = String(error?.message ?? error ?? "");
      if (message.includes("UNIQUE") || message.includes("duplicate")) {
        errorName = "Une playlist avec ce nom existe déjà";
      } else {
        errorName = "Une erreur est survenue";
      }
    } finally {
      isSubmitting = false;
    }
  }
</script>

<div class="space-y-5">

  <!-- Preview live -->
  <div class="flex items-center gap-4 p-3 rounded-xl"
       style="background: {selectedColor}10;">
    <div class="w-14 h-14 rounded-xl shrink-0 flex items-center justify-center"
         style="background: {selectedColor}25; border: 1px solid {selectedColor}30;">
      <Icon icon={selectedIcon.id} width="24" height="24"
            style="color: {selectedColor};" class="drop-shadow-sm" />
    </div>
    <div class="min-w-0">
      <p class="text-sm font-semibold text-neutral-900 dark:text-neutral-100 truncate">
        {name.trim() || 'Nouvelle playlist'}
      </p>
      <p class="text-xs text-neutral-500 dark:text-neutral-400">
        0 titre · Playlist
      </p>
    </div>
  </div>

  <!-- Nom -->
  <div class="flex flex-col gap-1.5">
    <label for="pl_name" class="text-[11px] font-semibold uppercase tracking-wider text-neutral-500 dark:text-neutral-400">
      Nom <span class="text-red-500">*</span>
    </label>
    <input
      id="pl_name"
      class="w-full rounded-xl border px-4 py-3 text-sm
             bg-neutral-50 dark:bg-neutral-800/80
             text-neutral-900 dark:text-neutral-100
             placeholder-neutral-400 dark:placeholder-neutral-500
             transition-all duration-200
             focus:outline-none focus:ring-2 focus:border-transparent
             {errorName
               ? 'border-red-400/60 focus:ring-red-500/40'
               : 'border-neutral-200 dark:border-neutral-700 focus:ring-emerald-500/40 hover:border-neutral-300 dark:hover:border-neutral-600'}"
      placeholder="Ex : Chill Vibes, Workout Mix…"
      bind:value={name}
      oninput={() => errorName = null}
      disabled={isSubmitting}
    />
    {#if errorName}
      <p class="flex items-center gap-1.5 text-xs text-red-500">
        <Icon icon="heroicons:exclamation-circle" class="w-3.5 h-3.5 shrink-0" />
        {errorName}
      </p>
    {/if}
  </div>

  <!-- Description -->
  <div class="flex flex-col gap-1.5">
    <label for="pl_desc" class="text-[11px] font-semibold uppercase tracking-wider text-neutral-500 dark:text-neutral-400">
      Description <span class="normal-case font-normal tracking-normal">(optionnel)</span>
    </label>
    <textarea id="pl_desc"
      class="w-full rounded-xl border px-4 py-3 text-sm resize-none
             bg-neutral-50 dark:bg-neutral-800/80
             text-neutral-900 dark:text-neutral-100
             placeholder-neutral-400 dark:placeholder-neutral-500
             border-neutral-200 dark:border-neutral-700
             transition-all duration-200
             hover:border-neutral-300 dark:hover:border-neutral-600
             focus:outline-none focus:ring-2 focus:ring-emerald-500/40 focus:border-transparent"
      placeholder="Une courte description…"
      rows="2"
      bind:value={description}
      disabled={isSubmitting}
    ></textarea>
  </div>

  <!-- Couleur -->
  <div class="flex flex-col gap-2">
    <span class="text-[11px] font-semibold uppercase tracking-wider text-neutral-500 dark:text-neutral-400">
      Couleur
    </span>
    <div class="flex items-center gap-2">
      {#each PLAYLIST_COLORS as color, i}
        <button
          type="button"
          aria-label="Couleur {color}"
          class="w-6 h-6 shrink-0 aspect-square rounded-full cursor-pointer transition-all duration-200
                 {selectedColorIndex === i
                   ? 'scale-115'
                   : 'opacity-50 hover:opacity-100 hover:scale-105'}"
          style="background: {color};
                 {selectedColorIndex === i
                   ? `box-shadow: 0 0 12px ${color}50, 0 0 0 2px rgba(0,0,0,0.2), 0 0 0 4px ${color}80;`
                   : ''}"
          onclick={() => selectedColorIndex = i}
        ></button>
      {/each}
    </div>
  </div>

  <!-- Icône -->
  <div class="flex flex-col gap-2">
    <span class="text-[11px] font-semibold uppercase tracking-wider text-neutral-500 dark:text-neutral-400">
      Icône
    </span>
    <div class="flex flex-wrap items-center gap-1.5">
      {#each PLAYLIST_ICONS as icon, i}
        <button
          type="button"
          class="w-9 h-9 shrink-0 rounded-lg flex items-center justify-center cursor-pointer
                 transition-all duration-200
                 {selectedIconIndex === i
                   ? 'bg-neutral-200 dark:bg-neutral-700 scale-105'
                   : 'hover:bg-neutral-100 dark:hover:bg-neutral-800 opacity-50 hover:opacity-100'}"
          style={selectedIconIndex === i ? `color: ${selectedColor};` : ''}
          onclick={() => selectedIconIndex = i}
          title={icon.label}
        >
          <Icon icon={icon.id} width="18" height="18" />
        </button>
      {/each}
    </div>
  </div>

  <!-- Actions -->
  <div class="flex justify-end gap-3 pt-1">
    <button
      type="button"
      class="px-4 py-2 rounded-lg text-sm font-medium cursor-pointer
            text-neutral-500 hover:text-neutral-700 dark:hover:text-neutral-200
            hover:bg-neutral-100 dark:hover:bg-neutral-800
            transition-all duration-150
            disabled:opacity-50 disabled:cursor-not-allowed"
      onclick={close}
      disabled={isSubmitting}
    >
      Annuler
    </button>

    <button
      type="button"
      class="flex items-center gap-2 px-5 py-2 rounded-lg text-sm font-semibold cursor-pointer
            text-white shadow-sm
            hover:brightness-110 hover:shadow-md
            active:scale-[0.97]
            transition-all duration-150
            disabled:opacity-50 disabled:cursor-not-allowed disabled:shadow-none"
      style="background: {selectedColor}; box-shadow: 0 2px 12px {selectedColor}40;"
      onclick={submit}
      disabled={isSubmitting}
    >
      {#if isSubmitting}
        <Icon icon="lucide:loader-2" width="14" height="14" class="animate-spin" />
        Création…
      {:else}
        <Icon icon="lucide:plus" width="14" height="14" />
        Créer
      {/if}
    </button>
  </div>
</div>
