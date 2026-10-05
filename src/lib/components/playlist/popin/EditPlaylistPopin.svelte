<script lang="ts">
  import { messageErreur } from "#lib/helper/tools/errorTools";
  import { popinStore } from "#lib/stores/ui/popin.store";
  import { playlistStore } from "#lib/stores/playlist/playlist.store";
  import { PLAYLIST_COLORS, PLAYLIST_ICONS } from "../playlistConfig";
  import type { Playlist } from "#lib/types/db/playlist/Playlist";
  import Icon from "@iconify/svelte";
  import { t } from "#lib/i18n";

  /* eslint-disable svelte/valid-prop-names-in-kit-pages */
  const { playlist }: { playlist: Playlist } = $props();

  // svelte-ignore state_referenced_locally
  const { id: playlistId, name: _name, color: _color, icon: _icon, description: _desc, track_count } = playlist;


  const initialColorIndex = Math.max(0, PLAYLIST_COLORS.indexOf(_color));
  const initialIconIndex = Math.max(0, PLAYLIST_ICONS.findIndex(i => i.id === _icon));

  let name = $state(_name);
  let description = $state(_desc ?? "");
  let selectedColorIndex = $state(initialColorIndex);
  let selectedIconIndex = $state(initialIconIndex);
  let isSubmitting = $state(false);
  let errorName: string | null = $state(null);
  let showDeleteConfirm = $state(false);

  let selectedColor = $derived(PLAYLIST_COLORS[selectedColorIndex]);
  let selectedIcon = $derived(PLAYLIST_ICONS[selectedIconIndex]);
  // Libellé traduit d'une icône, d'après son id (« mynaui:heart » → heart).
  const iconLabel = (id: string) => $t(`playlist_icons.${id.split(":")[1]}`);

  function close() {
    popinStore.close();
  }

  async function submit() {
    errorName = null;
    if (!name.trim()) {
      errorName = "forms.name_required";
      return;
    }
    isSubmitting = true;
    try {
      await playlistStore.updatePlaylist(playlistId, name, description || null, selectedColor, selectedIcon.id);
      close();
    } catch (error) {
      const message = messageErreur(error);
      if (message.includes("UNIQUE") || message.includes("duplicate")) {
        errorName = "forms.name_taken";
      } else {
        errorName = "forms.error_generic";
      }
    } finally {
      isSubmitting = false;
    }
  }

  async function handleDelete() {
    isSubmitting = true;
    try {
      await playlistStore.removePlaylist(playlistId);
      close();
    } catch (error) {
      console.error("Failed to delete playlist", error);
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
        {name.trim() || _name}
      </p>
      <p class="text-xs text-neutral-500 dark:text-neutral-400">
        {track_count === 1 ? $t("home.track_one") : $t("home.tracks_n").replace("{n}", String(track_count))} · {$t("home.playlist")}
      </p>
    </div>
  </div>

  <!-- Nom -->
  <div class="flex flex-col gap-1.5">
    <label for="pl_edit_name" class="text-[11px] font-semibold uppercase tracking-wider text-neutral-500 dark:text-neutral-400">
      {$t("forms.name")} <span class="text-red-500">*</span>
    </label>
    <input
      id="pl_edit_name"
      class="w-full rounded-xl border px-4 py-3 text-sm
             bg-neutral-50 dark:bg-neutral-800/80
             text-neutral-900 dark:text-neutral-100
             placeholder-neutral-400 dark:placeholder-neutral-500
             transition-all duration-200
             focus:outline-none focus:ring-2 focus:border-transparent
             {errorName
               ? 'border-red-400/60 focus:ring-red-500/40'
               : 'border-neutral-200 dark:border-neutral-700 focus:ring-emerald-500/40 hover:border-neutral-300 dark:hover:border-neutral-600'}"
      placeholder={$t("popins.playlist_name")}
      bind:value={name}
      oninput={() => errorName = null}
      disabled={isSubmitting}
    />
    {#if errorName}
      <p class="flex items-center gap-1.5 text-xs text-red-500">
        <Icon icon="heroicons:exclamation-circle" class="w-3.5 h-3.5 shrink-0" />
        {$t(errorName)}
      </p>
    {/if}
  </div>

  <!-- Description -->
  <div class="flex flex-col gap-1.5">
    <label for="pl_edit_desc" class="text-[11px] font-semibold uppercase tracking-wider text-neutral-500 dark:text-neutral-400">
      {$t("forms.description")} <span class="normal-case font-normal tracking-normal">{$t("forms.optional")}</span>
    </label>
    <textarea id="pl_edit_desc"
      class="w-full rounded-xl border px-4 py-3 text-sm resize-none
             bg-neutral-50 dark:bg-neutral-800/80
             text-neutral-900 dark:text-neutral-100
             placeholder-neutral-400 dark:placeholder-neutral-500
             border-neutral-200 dark:border-neutral-700
             transition-all duration-200
             hover:border-neutral-300 dark:hover:border-neutral-600
             focus:outline-none focus:ring-2 focus:ring-emerald-500/40 focus:border-transparent"
      placeholder={$t("forms.description_placeholder")}
      rows="2"
      bind:value={description}
      disabled={isSubmitting}
    ></textarea>
  </div>

  <!-- Couleur -->
  <div class="flex flex-col gap-2">
    <span class="text-[11px] font-semibold uppercase tracking-wider text-neutral-500 dark:text-neutral-400">
      {$t("forms.color")}
    </span>
    <div class="flex items-center gap-2">
      {#each PLAYLIST_COLORS as color, i}
        <button
          type="button"
          class="w-6 h-6 shrink-0 aspect-square rounded-full cursor-pointer transition-all duration-200
                 {selectedColorIndex === i
                   ? 'scale-115'
                   : 'opacity-50 hover:opacity-100 hover:scale-105'}"
          style="background: {color};
                 {selectedColorIndex === i
                   ? `box-shadow: 0 0 12px ${color}50, 0 0 0 2px rgba(0,0,0,0.2), 0 0 0 4px ${color}80;`
                   : ''}"
          aria-label={$t("forms.color_aria").replace("{color}", color)}
          onclick={() => selectedColorIndex = i}
        ></button>
      {/each}
    </div>
  </div>

  <!-- Icône -->
  <div class="flex flex-col gap-2">
    <span class="text-[11px] font-semibold uppercase tracking-wider text-neutral-500 dark:text-neutral-400">
      {$t("forms.icon")}
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
          title={iconLabel(icon.id)}
        >
          <Icon icon={icon.id} width="18" height="18" />
        </button>
      {/each}
    </div>
  </div>

  <!-- Séparateur -->
  <div class="h-px bg-neutral-200 dark:bg-neutral-700/50"></div>

  <!-- Actions -->
  <div class="flex items-center justify-between">
    <!-- Delete -->
    <div>
      {#if !showDeleteConfirm}
        <button
          type="button"
          class="flex items-center gap-1.5 text-xs text-red-400/60 hover:text-red-400 transition-colors cursor-pointer"
          onclick={() => showDeleteConfirm = true}
          disabled={isSubmitting}
        >
          <Icon icon="heroicons:trash" class="w-3.5 h-3.5" />
          {$t("common.delete")}
        </button>
      {:else}
        <div class="flex items-center gap-2.5">
          <span class="text-xs text-red-400">{$t("profil.confirm_delete")}</span>
          <button
            type="button"
            class="px-2.5 py-1 rounded text-xs font-semibold text-white bg-red-500 hover:bg-red-400 cursor-pointer transition-colors"
            onclick={handleDelete}
            disabled={isSubmitting}
          >
            {$t("common.confirm")}
          </button>
          <button
            type="button"
            class="px-2.5 py-1 rounded text-xs text-neutral-500 dark:text-neutral-400
                   bg-neutral-100 dark:bg-neutral-800 hover:bg-neutral-200 dark:hover:bg-neutral-700
                   cursor-pointer transition-colors"
            onclick={() => showDeleteConfirm = false}
          >
            {$t("common.cancel")}
          </button>
        </div>
      {/if}
    </div>

    <div class="flex gap-3">
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
        {$t("common.cancel")}
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
          {$t("forms.saving")}
        {:else}
          <Icon icon="lucide:check" width="14" height="14" />
          {$t("common.save")}
        {/if}
      </button>
    </div>
  </div>
</div>
