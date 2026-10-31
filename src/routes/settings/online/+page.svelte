<script lang="ts">
  // Écoute en ligne : les sources distantes, chacune activable à part. Les podcasts d'abord.
  import Icon from "@iconify/svelte";
  import { t, currentLocale } from "#lib/i18n";
  import OptionGroup from "#lib/components/ui/input/OptionGroup.svelte";
  import OptionItem from "#lib/components/ui/input/OptionItem.svelte";
  import OptionBlock from "#lib/components/ui/input/OptionBlock.svelte";
  import ToggleSwitch from "#lib/components/ui/input/ToggleSwitch.svelte";
  import Badge from "#lib/components/ui/text/Badge.svelte";
  import SegmentedControl from "#lib/components/ui/input/SegmentedControl.svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { settingsStore } from "#lib/stores/settings/settings.store";
  import { profilSelector } from "#lib/stores/profil/profil.store";
  import { readableSize } from "#lib/helper/tools/sizeTools";
  import GhostButton from "#lib/components/ui/button/GhostButton.svelte";
  import { goto } from "$app/navigation";
  import type { DownloadView } from "#lib/types/ui/podcast/Podcast";
  import MenuSelect from "#lib/components/ui/menu/MenuSelect.svelte";
  import { SPEEDS } from "#lib/stores/player/playbackSpeed.store";
  import { countryOptions } from "#lib/helper/tools/podcastCountry";
  import { open, save } from "@tauri-apps/plugin-dialog";
  import { toasts } from "#lib/stores/ui/toast.store";
  import { podcastStore } from "#lib/stores/podcast/podcast.store";
  import { radioStore } from "#lib/stores/radio/radio.store";
  import { popinStore } from "#lib/stores/ui/popin.store";
  import Dialog from "#lib/components/ui/dialog/Dialog.svelte";
  import ListenBrainzSettings from "#lib/components/settings/ListenBrainzSettings.svelte";
  import LastfmSettings from "#lib/components/settings/LastfmSettings.svelte";

  // OPML : les abonnements, échangés avec les autres applis de podcasts.
  let opml = $state<"import" | "export" | null>(null);

  // Historique d'écoute (podcasts : écouté et reprise ; radios : dernières écoutées et titres passés), après confirmation.
  function clearHistory(kind: "podcasts" | "radios") {
    const profilId = $profilSelector.profilSelected?.id;
    if (profilId == null) return;
    popinStore.open($t(`${kind}.clear_history`), Dialog, {
      message: $t(`${kind}.clear_history_confirm`),
      confirmText: $t("common.delete"),
      cancelText: $t("common.cancel"),
      variant: "danger",
      onConfirm: async () => {
        await invoke(kind === "podcasts" ? "podcast_clear_history" : "clear_radio_history", { profilId });
        await (kind === "podcasts" ? podcastStore.load(profilId) : radioStore.load(profilId));
        toasts.push({ type: "success", title: $t(`${kind}.history_cleared`) });
      },
    });
  }
  const opmlFilter = [{ name: "OPML", extensions: ["opml", "xml"] }];

  async function importOpml() {
    const profilId = $profilSelector.profilSelected?.id;
    const path = await open({ title: $t("podcasts.opml_import"), filters: opmlFilter, multiple: false, directory: false });
    if (!profilId || typeof path !== "string") return;
    opml = "import";
    try {
      const r = await invoke<{ added: number; existing: number; failed: number }>("podcast_import_opml", { profilId, path });
      await podcastStore.load(profilId);
      const message = $t("podcasts.opml_imported").replace("{added}", String(r.added)).replace("{existing}", String(r.existing))
        + (r.failed ? " · " + $t("podcasts.opml_failed").replace("{n}", String(r.failed)) : "");
      toasts.push({ type: r.added || r.existing ? "success" : "error", title: $t("podcasts.opml_import"), message });
    } catch (e) {
      toasts.push({ type: "error", title: $t("podcasts.opml_import"), message: String(e) });
    } finally {
      opml = null;
    }
  }

  async function exportOpml() {
    const profilId = $profilSelector.profilSelected?.id;
    const path = await save({ title: $t("podcasts.opml_export"), defaultPath: "rustmusic-podcasts.opml", filters: opmlFilter });
    if (!profilId || !path) return;
    opml = "export";
    try {
      const n = await invoke<number>("podcast_export_opml", { profilId, path });
      toasts.push({ type: "success", title: $t("podcasts.opml_export"), message: $t("podcasts.opml_exported").replace("{n}", String(n)) });
    } catch (e) {
      toasts.push({ type: "error", title: $t("podcasts.opml_export"), message: String(e) });
    } finally {
      opml = null;
    }
  }

  const paysOptions = $derived(countryOptions($currentLocale, $t("podcasts.country_auto")));

  const podcasts = $derived($settingsStore.podcasts === "true");
  const radios = $derived($settingsStore.radios === "true");

  // Place prise par les épisodes téléchargés du profil.
  let occupe = $state<number | null>(null);
  $effect(() => {
    const profilId = $profilSelector.profilSelected?.id;
    if (!podcasts || profilId == null) return;
    invoke<DownloadView[]>("get_podcast_downloads", { profilId })
      .then((l) => (occupe = l.reduce((s, d) => s + d.size, 0)))
      .catch(() => (occupe = null));
  });
</script>

<OptionGroup foldKey="online-podcasts" title={$t("podcasts.settings_group")} hint={$t("podcasts.settings_group_hint")}>
  {#snippet action()}<Badge tone="amber">{$t("settings.beta_badge")}</Badge>{/snippet}
  <OptionItem
    title={$t("podcasts.toggle")}
    desc={$t("podcasts.toggle_desc")}
    keywords="podcast podcasts rss flux épisodes abonnements apple"
    onclick={() => settingsStore.toggle("podcasts")}
  >
    <ToggleSwitch checked={podcasts} label={$t("podcasts.toggle")} onclick={() => settingsStore.toggle("podcasts")} />
  </OptionItem>

  {#if podcasts}
    <OptionItem title={$t("podcasts.play_mode")} desc={$t("podcasts.play_mode_desc")} keywords="podcast lecture téléchargement streaming">
      <SegmentedControl variant="discret" label={$t("podcasts.play_mode")} value={$settingsStore.podcast_play_mode}
                        onchange={(v) => settingsStore.set("podcast_play_mode", v)}
                        options={[
                          { value: "stream", label: $t("podcasts.play_mode_stream") },
                          { value: "download", label: $t("podcasts.play_mode_download") },
                        ]} />
    </OptionItem>
    <OptionItem title={$t("podcasts.country")} desc={$t("podcasts.country_desc")} keywords="podcast pays country classement annuaire langue">
      <MenuSelect value={$settingsStore.podcast_country} onchange={(v) => settingsStore.set("podcast_country", v)}
                  options={paysOptions} menuClass="w-64 max-h-80 overflow-y-auto" />
    </OptionItem>
    <OptionItem title={$t("podcasts.speed_default")} desc={$t("podcasts.speed_default_desc")} keywords="podcast vitesse lecture rapide accélérer">
      <MenuSelect value={$settingsStore.podcast_speed} onchange={(v) => settingsStore.set("podcast_speed", v)}
                  options={SPEEDS.map((v) => ({ value: String(v), label: `${v.toLocaleString($currentLocale)}×` }))} menuClass="w-40" />
    </OptionItem>
    <OptionItem title={$t("podcasts.refresh")} desc={$t("podcasts.refresh_desc")} keywords="podcast vérifier nouveaux épisodes actualiser automatique">
      <MenuSelect value={$settingsStore.podcast_refresh} onchange={(v) => settingsStore.set("podcast_refresh", v)}
                  options={["never", "1h", "6h", "24h"].map((v) => ({ value: v, label: $t(`podcasts.refresh_${v}`) }))} menuClass="w-56" />
    </OptionItem>
    <OptionItem title={$t("podcasts.cleanup")} desc={$t("podcasts.cleanup_desc")} keywords="podcast ménage effacer écoutés téléchargements espace">
      <MenuSelect value={$settingsStore.podcast_cleanup} onchange={(v) => settingsStore.set("podcast_cleanup", v)}
                  options={["never", "now", "24h", "7d", "30d"].map((v) => ({ value: v, label: $t(`podcasts.cleanup_${v}`) }))} menuClass="w-56" />
    </OptionItem>
    <OptionItem title={$t("podcasts.opml")} desc={$t("podcasts.opml_desc")} keywords="podcast opml import export abonnements sauvegarde">
      <div class="flex flex-wrap gap-2">
        <GhostButton icon="material-symbols:upload-file-outline-rounded" label={$t("podcasts.opml_import_short")} busy={opml === "import"} disabled={opml !== null} onclick={importOpml} />
        <GhostButton icon="material-symbols:download-rounded" label={$t("podcasts.opml_export_short")} busy={opml === "export"} disabled={opml !== null} onclick={exportOpml} />
      </div>
    </OptionItem>
    <OptionItem title={$t("podcasts.clear_history")} desc={$t("podcasts.clear_history_desc")} keywords="podcast historique écoute effacer reprise écoutés">
      <GhostButton icon="material-symbols:history-off-rounded" label={$t("podcasts.clear_history_short")} danger onclick={() => clearHistory("podcasts")} />
    </OptionItem>
    <OptionItem title={$t("podcasts.downloads")}
                desc={$t("podcasts.space_used").replace("{size}", readableSize(occupe ?? 0, $currentLocale) || "0")}
                keywords="podcast téléchargements cache espace disque vider supprimer">
      <GhostButton icon="material-symbols:download-for-offline-outline-rounded" label={$t("podcasts.manage_downloads")} onclick={() => goto("/podcasts/downloads")} />
    </OptionItem>
    <OptionBlock keywords={$t("podcasts.network_title")} class="flex gap-3.5 px-5 py-4 leading-[1.25]">
      <Icon icon="material-symbols:public" width="20" class="shrink-0 mt-px text-(--rg-mu)" />
      <div class="min-w-0">
        <p class="text-[15px] font-semibold text-(--rg-tx)">{$t("podcasts.network_title")}</p>
        <p class="mt-0.75 text-[13px] text-(--rg-mu) text-pretty">{$t("podcasts.network_desc")}</p>
      </div>
    </OptionBlock>
  {/if}
</OptionGroup>

<OptionGroup foldKey="online-radios" title={$t("radios.settings_group")} hint={$t("radios.settings_group_hint")}>
  {#snippet action()}<Badge tone="amber">{$t("settings.beta_badge")}</Badge>{/snippet}
  <OptionItem
    title={$t("radios.toggle")}
    desc={$t("radios.toggle_desc")}
    keywords="radio radios webradio direct flux stream"
    onclick={() => settingsStore.toggle("radios")}
  >
    <ToggleSwitch checked={radios} label={$t("radios.toggle")} onclick={() => settingsStore.toggle("radios")} />
  </OptionItem>

  {#if radios}
    <OptionItem title={$t("radios.clear_history")} desc={$t("radios.clear_history_desc")} keywords="radio historique dernières écoutées titres effacer">
      <GhostButton icon="material-symbols:history-off-rounded" label={$t("radios.clear_history_short")} danger onclick={() => clearHistory("radios")} />
    </OptionItem>
    <OptionBlock keywords={$t("radios.network_title")} class="flex gap-3.5 px-5 py-4 leading-[1.25]">
      <Icon icon="material-symbols:public" width="20" class="shrink-0 mt-px text-(--rg-mu)" />
      <div class="min-w-0">
        <p class="text-[15px] font-semibold text-(--rg-tx)">{$t("radios.network_title")}</p>
        <p class="mt-0.75 text-[13px] text-(--rg-mu) text-pretty">{$t("radios.network_desc")}</p>
      </div>
    </OptionBlock>
  {/if}
</OptionGroup>

<ListenBrainzSettings />

<LastfmSettings />
