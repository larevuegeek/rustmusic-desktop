<script lang="ts">
  import { settingsStore, type AppSettings } from "$lib/stores/settings/settings.store";
  import { t } from "$lib/i18n";
  import { detectOS } from "$lib/helper/tools/osDetection";
  import OptionGroup from "$lib/components/ui/input/OptionGroup.svelte";
  import OptionItem from "$lib/components/ui/input/OptionItem.svelte";
  import SelectField from "$lib/components/ui/input/SelectField.svelte";
  import ToggleSwitch from "$lib/components/ui/input/ToggleSwitch.svelte";

  type Interrupteur = { cle: keyof AppSettings; titre: string; desc: string; motsCles?: string };

  const os = detectOS();
  const systeme = os === "macos" ? "macOS" : os === "linux" ? "Linux" : "Windows";

  const languages = [
    { value: "fr", label: "Français" },
    { value: "en", label: "English" },
    { value: "es", label: "Español" },
    { value: "de", label: "Deutsch" },
    { value: "it", label: "Italiano" },
  ];

  const groupes = $derived.by((): { titre: string; hint: string; items: Interrupteur[] }[] => {
    const txt = (cle: string) => $t(`settings.${cle}`).replaceAll("{os}", systeme);
    return [
      {
        titre: txt("group_startup"), hint: txt("group_startup_hint"),
        items: [
          { cle: "auto_start", titre: txt("autostart"), desc: txt("autostart_desc") },
          { cle: "minimize_to_tray", titre: txt("tray"), desc: txt("tray_desc"), motsCles: "tray" },
          { cle: "scan_on_startup", titre: txt("scan_on_startup"), desc: txt("scan_on_startup_desc"), motsCles: "scan" },
        ],
      },
      {
        titre: txt("group_playback"), hint: txt("group_playback_hint"),
        items: [
          { cle: "single_click_play", titre: txt("single_click_play"), desc: txt("single_click_play_desc") },
          { cle: "resume_playback", titre: txt("resume_playback"), desc: txt("resume_playback_desc") },
          { cle: "gapless", titre: txt("gapless"), desc: txt("gapless_desc"), motsCles: "gapless" },
        ],
      },
      {
        titre: txt("group_system"), hint: txt("group_system_hint"),
        items: [
          { cle: "system_media_controls", titre: txt("system_media_controls"), desc: txt("system_media_controls_desc"), motsCles: "smtc mpris" },
          { cle: "show_notifications", titre: txt("notifications"), desc: txt("notifications_desc") },
          { cle: "show_sleep_timer", titre: txt("show_sleep_timer"), desc: txt("show_sleep_timer_desc") },
        ],
      },
    ];
  });
</script>

<OptionGroup>
  <OptionItem title={$t("settings.language")} desc={$t("settings.language_desc")} keywords="language langue">
    <SelectField
      value={$settingsStore.language}
      options={languages}
      label={$t("settings.language")}
      onchange={(v) => settingsStore.set("language", v)}
    />
  </OptionItem>
</OptionGroup>

{#each groupes as groupe (groupe.titre)}
  <OptionGroup title={groupe.titre} hint={groupe.hint}>
    {#each groupe.items as item (item.cle)}
      <OptionItem title={item.titre} desc={item.desc} keywords={item.motsCles} onclick={() => settingsStore.toggle(item.cle)}>
        <ToggleSwitch
          checked={$settingsStore[item.cle] === "true"}
          label={item.titre}
          onclick={() => settingsStore.toggle(item.cle)}
        />
      </OptionItem>
    {/each}
  </OptionGroup>
{/each}
