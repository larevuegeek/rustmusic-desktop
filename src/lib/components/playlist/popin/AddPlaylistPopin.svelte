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
  import { serverStore } from "#lib/stores/mediaServer/mediaServer.store";
  import StatusDot from "#lib/components/ui/status/StatusDot.svelte";
  import { currentSource } from "#lib/stores/source/source.store";

  /** Appelé avec la playlist créée — sert à y verser aussitôt un morceau. */
  let { apresCreation }: { apresCreation?: (pl: Playlist) => void } = $props();

  let name = $state("");
  let description = $state("");
  let selectedColorIndex = $state(0);
  let selectedIconIndex = $state(0);
  let isSubmitting = $state(false);
  // Où la créer : `serverId` = sur ce serveur (synchronisée) ; `sourceId` = la source qui la range.
  type Lieu = { key: string; label: string; icon: string; serverId: number | null; sourceId: number; online?: boolean };
  const lieux = $derived.by((): Lieu[] => {
    const actifs = $serverStore.servers.filter((s) => s.enabled);
    // Playlists sur le serveur : Subsonic et Jellyfin (le DLNA n'en a pas).
    const avecPlaylists = (s: (typeof actifs)[number]) => s.kind === "subsonic" || s.kind === "jellyfin";
    const surServeur = (s: (typeof actifs)[number]): Lieu => ({
      key: `s${s.id}`, label: s.name, icon: "material-symbols:cloud-outline-rounded",
      serverId: avecPlaylists(s) ? s.id : null, sourceId: s.id, online: $serverStore.online[s.id],
    });
    if (!actifs.length || $currentSource === 0) return [];
    if ($currentSource === "all") {
      return [{ key: "local", label: $t("sources.local"), icon: "material-symbols:computer-outline-rounded", serverId: null, sourceId: 0 }, ...actifs.map(surServeur)];
    }
    const s = actifs.find((x) => x.id === $currentSource);
    if (!s || !avecPlaylists(s)) return [];
    return [surServeur(s), { key: `r${s.id}`, label: "RustMusic", icon: "material-symbols:hard-drive-outline", serverId: null, sourceId: s.id }];
  });
  let choix = $state<string | null>(null);
  const lieu = $derived(lieux.find((l) => l.key === choix) ?? lieux[0] ?? null);
  const serverId = $derived(lieu?.serverId ?? null);
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

      const creee = await playlistStore.addPlaylist(profilId, name, description || null, selectedColor, selectedIcon.id, serverId, lieu?.sourceId ?? null);
      close();
      apresCreation?.(creee);
    } catch (error) {
      const message = errorMessage(error);
      if (message.includes("UNIQUE") || message.includes("duplicate")) {
        errorName = "popins.playlist_name_taken";
      } else if (serverId != null) {
        errorName = "servers.playlist_create_failed";
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

  {#if lieux.length}
    <!-- Où la créer : sur le serveur (synchronisée), dans RustMusic, ou dans quelle source. -->
    <FormField label={$t("servers.playlist_where")} for="pl_where">
      <div id="pl_where" class="flex flex-wrap gap-2">
        {#each lieux as l (l.key)}
          {@const actif = lieu?.key === l.key}
          <button type="button" aria-pressed={actif} disabled={isSubmitting}
                  class="h-9 px-3 inline-flex items-center gap-2 rounded-xl border text-[13px] font-semibold cursor-pointer transition-colors
                         {actif ? (l.sourceId ? 'border-sky-500/50 bg-sky-500/10 text-sky-700 dark:text-sky-300' : 'border-(--rg-g)/50 bg-(--rg-gbg) text-(--rg-gtx)')
                                : 'border-(--rg-bd) text-(--rg-tx2) hover:bg-(--rg-hover) hover:text-(--rg-tx)'}"
                  onclick={() => (choix = l.key)}>
            <Icon icon={l.icon} width="17" />
            {l.label}
            {#if l.online !== undefined}<StatusDot online={l.online} />{/if}
          </button>
        {/each}
      </div>
      {#if serverId != null}
        <p class="mt-2 text-xs text-(--rg-mu)">{$t("servers.playlist_where_hint")}</p>
      {/if}
    </FormField>
  {/if}

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
