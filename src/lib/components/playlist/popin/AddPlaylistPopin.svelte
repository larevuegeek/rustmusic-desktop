<script lang="ts">
  import { errorMessage } from "#lib/helper/tools/errorTools";
  import { popinStore } from "#lib/stores/ui/popin.store";
  import { playlistStore } from "#lib/stores/playlist/playlist.store";
  import { profilSelector } from "#lib/stores/profil/profil.store";
  import { PLAYLIST_COLORS, PLAYLIST_ICONS } from "../playlistConfig";
  import Icon from "@iconify/svelte";
  import { t } from "#lib/i18n";
  import FormField from "#lib/components/ui/input/FormField.svelte";
  import FormInput from "#lib/components/ui/input/FormInput.svelte";
  import PopinFooter from "#lib/components/ui/popin/PopinFooter.svelte";
  import type { Playlist } from "#lib/types/db/playlist/Playlist";

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
  // Libellé traduit d'une icône, d'après son id (« mynaui:heart » → heart).
  // Bouton principal aux couleurs de la playlist.
  let submitStyle = $derived(`background: ${selectedColor}; color: #fff; box-shadow: 0 2px 12px ${selectedColor}40;`);
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
      const profilId = $profilSelector.profilSelected?.id;
      if (!profilId) throw new Error("NO_PROFIL");

      const creee = await playlistStore.addPlaylist(profilId, name, description || null, selectedColor, selectedIcon.id);
      close();
      apresCreation?.(creee);
    } catch (error) {
      const message = errorMessage(error);
      if (message.includes("UNIQUE") || message.includes("duplicate")) {
        errorName = "popins.playlist_name_taken";
      } else {
        errorName = "forms.error_generic";
      }
    } finally {
      isSubmitting = false;
    }
  }
</script>

<div class="space-y-6">
  <!-- La playlist telle qu'elle apparaîtra -->
  <div class="flex items-center gap-4 p-3.5 rounded-2xl border transition-colors"
       style="border-color: color-mix(in oklab, {selectedColor} 30%, transparent); background: linear-gradient(135deg, color-mix(in oklab, {selectedColor} 12%, transparent), transparent);">
    <span class="w-12 h-12 shrink-0 rounded-xl flex items-center justify-center transition-colors"
          style="background: color-mix(in oklab, {selectedColor} 16%, transparent); border: 1px solid color-mix(in oklab, {selectedColor} 30%, transparent); color: {selectedColor};">
      <Icon icon={selectedIcon.id} width="24" />
    </span>
    <div class="min-w-0">
      <p class="text-[15px] font-bold truncate text-(--rg-tx)">
        {name.trim() || $t("playlist.new")}
      </p>
      <p class="mt-0.5 text-xs text-(--rg-mu)">
        {$t("home.tracks_n").replace("{n}", "0")} · {$t("home.playlist")}
      </p>
    </div>
  </div>

  <FormField label={$t("forms.name")} for="pl_name" required error={errorName ? $t(errorName) : ""}>
    <FormInput id="pl_name" bind:value={name} placeholder={$t("popins.playlist_name_placeholder")}
               invalid={!!errorName} disabled={isSubmitting} oninput={() => errorName = null} />
  </FormField>

  <FormField label={$t("forms.description")} for="pl_desc" optional={$t("forms.optional")}>
    <FormInput id="pl_desc" multiline bind:value={description} placeholder={$t("forms.description_placeholder")} disabled={isSubmitting} />
  </FormField>

  <div class="grid grid-cols-[auto_1fr] items-center gap-x-5 gap-y-3.5">
    <span class="text-[11px] font-bold uppercase tracking-[0.1em] text-(--rg-mu)">{$t("forms.color")}</span>
    <div class="flex flex-wrap gap-2">
      {#each PLAYLIST_COLORS as color, i}
        <button type="button" aria-label={$t("forms.color_aria").replace("{color}", color)} aria-pressed={selectedColorIndex === i}
                class="w-6 h-6 rounded-full cursor-pointer transition-transform hover:scale-110
                       {selectedColorIndex === i ? 'ring-2 ring-offset-2 ring-(--rg-tx) ring-offset-(--rg-carte) scale-110' : ''}"
                style="background: {color}"
                onclick={() => selectedColorIndex = i}></button>
      {/each}
    </div>
    <span class="text-[11px] font-bold uppercase tracking-[0.1em] text-(--rg-mu)">{$t("forms.icon")}</span>
    <div class="flex flex-wrap gap-1">
      {#each PLAYLIST_ICONS as icon, i}
        <button type="button" title={iconLabel(icon.id)} aria-label={iconLabel(icon.id)} aria-pressed={selectedIconIndex === i}
                class="w-9 h-9 flex items-center justify-center rounded-xl cursor-pointer transition-colors
                       {selectedIconIndex === i ? 'text-white' : 'text-(--rg-mu) hover:bg-(--rg-hover) hover:text-(--rg-tx)'}"
                style={selectedIconIndex === i ? `background: ${selectedColor}` : ""}
                onclick={() => selectedIconIndex = i}>
          <Icon icon={icon.id} width="18" />
        </button>
      {/each}
    </div>
  </div>
</div>

<PopinFooter
  bleed
  cancelLabel={$t("common.cancel")}
  oncancel={close}
  cancelDisabled={isSubmitting}
  submitLabel={isSubmitting ? $t("forms.creating") : $t("common.create")}
  submitIcon="material-symbols:add-rounded"
  onsubmit={submit}
  busy={isSubmitting}
  {submitStyle}
/>
