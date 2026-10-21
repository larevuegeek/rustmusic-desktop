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
  import { countryOptions } from "#lib/helper/tools/podcastCountry";

  const paysOptions = $derived(countryOptions($currentLocale, $t("podcasts.country_auto")));

  const podcasts = $derived($settingsStore.podcasts === "true");

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

<OptionGroup title={$t("podcasts.settings_group")} hint={$t("podcasts.settings_group_hint")}>
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
