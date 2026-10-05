<script lang="ts">
  // Raccourcis en lecture seule : ceux que gèrent la fenêtre principale et les touches média.
  import Icon from "@iconify/svelte";
  import { t } from "#lib/i18n";
  import { settingsStore } from "#lib/stores/settings/settings.store";
  import { detectOS } from "#lib/helper/tools/osDetection";
  import OptionGroup from "#lib/components/ui/input/OptionGroup.svelte";
  import OptionItem from "#lib/components/ui/input/OptionItem.svelte";

  /** Une combinaison = une liste de touches ; plusieurs combinaisons = alternatives. */
  type Raccourci = { titre: string; desc?: string; combos: string[][]; motsCles?: string };

  const mod = detectOS() === "macos" ? "⌘" : "Ctrl";
  // Figtree n'a pas de flèches : icônes à la place.
  const fleches: Record<string, string> = {
    "→": "material-symbols:arrow-forward-rounded",
    "←": "material-symbols:arrow-back-rounded",
    "↑": "material-symbols:arrow-upward-rounded",
    "↓": "material-symbols:arrow-downward-rounded",
  };
  const mediaActifs = $derived($settingsStore.system_media_controls === "true");

  const groupes = $derived.by((): { titre: string; hint: string; items: Raccourci[] }[] => {
    const txt = (cle: string) => $t(`settings.${cle}`);
    return [
      {
        titre: txt("group_playback"), hint: txt("shortcuts_playback_hint"),
        items: [
          { titre: txt("shortcut_play_pause"), combos: [[txt("key_space")]], motsCles: "space espace" },
          { titre: txt("shortcut_forward"), desc: txt("shortcut_mini_hint"), combos: [["→"]] },
          { titre: txt("shortcut_backward"), desc: txt("shortcut_mini_hint"), combos: [["←"]] },
          { titre: txt("shortcut_next"), combos: [[mod, "→"]] },
          { titre: txt("shortcut_previous"), combos: [[mod, "←"]] },
        ],
      },
      {
        titre: txt("shortcuts_volume"), hint: txt("shortcuts_volume_hint"),
        items: [
          { titre: txt("shortcut_volume_up"), combos: [["↑"]] },
          { titre: txt("shortcut_volume_down"), combos: [["↓"]] },
          { titre: txt("shortcut_mute"), combos: [["M"]], motsCles: "mute" },
        ],
      },
      {
        titre: txt("shortcuts_navigation"), hint: txt("shortcuts_navigation_hint"),
        items: [
          { titre: txt("shortcut_search"), combos: [[mod, "K"], [mod, "F"]] },
          { titre: txt("shortcut_range"), desc: txt("shortcut_range_desc"), combos: [[txt("key_shift"), txt("key_click")]] },
          { titre: txt("shortcut_close"), desc: txt("shortcut_close_desc"), combos: [[txt("key_escape")]], motsCles: "escape echap" },
        ],
      },
    ];
  });

  const media = $derived([
    { titre: $t("settings.shortcut_play_pause"), icone: "material-symbols:play-pause-rounded" },
    { titre: $t("settings.shortcut_next"), icone: "material-symbols:skip-next-rounded" },
    { titre: $t("settings.shortcut_previous"), icone: "material-symbols:skip-previous-rounded" },
  ]);
</script>

{#snippet touche(contenu: string)}
  <kbd class="min-w-7.5 h-7.5 px-2 inline-flex items-center justify-center rounded-lg
              bg-(--rg-s2) border border-(--rg-bd2) border-b-2
              font-sans text-[13px] font-semibold text-(--rg-tx2)">
    {#if fleches[contenu]}<Icon icon={fleches[contenu]} width="16" />{:else}{contenu}{/if}
  </kbd>
{/snippet}

{#each groupes as groupe (groupe.titre)}
  <OptionGroup title={groupe.titre} hint={groupe.hint}>
    {#each groupe.items as item (item.titre)}
      <OptionItem title={item.titre} desc={item.desc} keywords={[...item.combos.flat(), item.motsCles].join(" ")}>
        {#each item.combos as combo, i (i)}
          {#if i > 0}<span class="text-xs text-(--rg-mu2)">{$t("settings.key_or")}</span>{/if}
          <span class="flex items-center gap-1">
            {#each combo as cle, j (j)}
              {#if j > 0}<span class="text-xs text-(--rg-mu2)">+</span>{/if}
              {@render touche(cle)}
            {/each}
          </span>
        {/each}
      </OptionItem>
    {/each}
  </OptionGroup>
{/each}

<OptionGroup title={$t("settings.shortcuts_media")} hint={$t("settings.shortcuts_media_hint")}>
  {#if !mediaActifs}
    <OptionItem title={$t("settings.shortcuts_media_off")} desc={$t("settings.shortcuts_media_off_desc")}>
      <button
        type="button"
        class="h-8 px-3 rounded-lg cursor-pointer text-[13px] font-semibold transition-colors
               bg-(--rg-g) text-white dark:text-(--rg-on-g) hover:brightness-110"
        onclick={() => settingsStore.set("system_media_controls", "true")}
      >
        {$t("settings.enable")}
      </button>
    </OptionItem>
  {/if}
  {#each media as item (item.icone)}
    <OptionItem title={item.titre} keywords="media multimedia">
      <kbd class="w-9.5 h-7.5 inline-flex items-center justify-center rounded-lg
                  bg-(--rg-s2) border border-(--rg-bd2) border-b-2 text-(--rg-tx2)
                  {mediaActifs ? '' : 'opacity-50'}">
        <Icon icon={item.icone} width="22" />
      </kbd>
    </OptionItem>
  {/each}
</OptionGroup>
