<script lang="ts">
  // Raccourcis : un clic sur les touches pour les changer, « Rétablir » pour revenir à l'origine.
  import Icon from "@iconify/svelte";
  import { t } from "#lib/i18n";
  import { settingsStore } from "#lib/stores/settings/settings.store";
  import { detectOS } from "#lib/helper/tools/osDetection";
  import OptionGroup from "#lib/components/ui/input/OptionGroup.svelte";
  import OptionItem from "#lib/components/ui/input/OptionItem.svelte";
  import {
    SHORTCUT_ACTIONS, DEFAULT_SHORTCUTS, RESERVED_KEYS,
    eventToBinding, parseOverrides, resolveShortcuts, findAction,
    type ShortcutAction, type ShortcutGroup,
  } from "#lib/config/shortcuts";

  const isMac = detectOS() === "macos";
  // Figtree n'a pas de flèches : icônes à la place.
  const arrows: Record<string, string> = {
    ArrowRight: "material-symbols:arrow-forward-rounded",
    ArrowLeft: "material-symbols:arrow-back-rounded",
    ArrowUp: "material-symbols:arrow-upward-rounded",
    ArrowDown: "material-symbols:arrow-downward-rounded",
  };
  const mediaEnabled = $derived($settingsStore.system_media_controls === "true");

  const overrides = $derived(parseOverrides($settingsStore.shortcuts));
  const shortcuts = $derived(resolveShortcuts($settingsStore.shortcuts));
  const customized = $derived(Object.keys(overrides).length > 0);

  const groups: { group: ShortcutGroup; titleKey: string; hintKey: string }[] = [
    { group: "playback", titleKey: "settings.group_playback", hintKey: "settings.shortcuts_playback_hint" },
    { group: "volume", titleKey: "settings.shortcuts_volume", hintKey: "settings.shortcuts_volume_hint" },
    { group: "navigation", titleKey: "settings.shortcuts_navigation", hintKey: "settings.shortcuts_navigation_hint" },
  ];

  /** Libellés affichés d'une combinaison enregistrée (« Mod+Shift+M »). */
  function keyLabels(binding: string): string[] {
    return binding.split("+").map((part) => {
      if (part === "Mod") return isMac ? "⌘" : "Ctrl";
      if (part === "Alt") return isMac ? "⌥" : "Alt";
      if (part === "Shift") return isMac ? "⇧" : $t("settings.key_shift");
      if (part === "Space") return $t("settings.key_space");
      return part;
    });
  }

  // ─── Saisie d'une nouvelle touche ───
  let editing = $state<ShortcutAction | null>(null);
  let message = $state("");

  function save(action: ShortcutAction, binding: string | null) {
    const next = { ...overrides };
    if (binding === null || DEFAULT_SHORTCUTS[action].join() === binding) delete next[action];
    else next[action] = [binding];
    settingsStore.set("shortcuts", Object.keys(next).length ? JSON.stringify(next) : "");
  }

  function startEditing(action: ShortcutAction) {
    editing = action;
    message = "";
  }

  // En phase de capture sur la fenêtre : la touche ne déclenche pas le raccourci qu'elle remplace.
  $effect(() => {
    if (!editing) return;
    const action = editing;
    const onKey = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      if (e.key === "Escape") {
        editing = null;
        return;
      }
      const binding = eventToBinding(e);
      if (!binding) return;
      if (RESERVED_KEYS.includes(e.key)) {
        message = $t("settings.shortcut_reserved");
        return;
      }
      const owner = findAction(shortcuts, binding);
      if (owner && owner !== action) {
        const label = SHORTCUT_ACTIONS.find((a) => a.action === owner)?.labelKey ?? "";
        message = $t("settings.shortcut_in_use").replace("{action}", $t(label));
        return;
      }
      save(action, binding);
      editing = null;
    };
    const onPointer = () => (editing = null);
    window.addEventListener("keydown", onKey, true);
    // Un clic ailleurs annule (le clic qui ouvre la saisie est déjà passé).
    const timer = setTimeout(() => window.addEventListener("pointerdown", onPointer), 0);
    return () => {
      window.removeEventListener("keydown", onKey, true);
      clearTimeout(timer);
      window.removeEventListener("pointerdown", onPointer);
    };
  });

  const media = $derived([
    { title: $t("settings.shortcut_play_pause"), icon: "material-symbols:play-pause-rounded" },
    { title: $t("settings.shortcut_next"), icon: "material-symbols:skip-next-rounded" },
    { title: $t("settings.shortcut_previous"), icon: "material-symbols:skip-previous-rounded" },
  ]);

  const ghostButton = "h-7.5 px-2.5 rounded-lg cursor-pointer text-[12px] font-semibold transition-colors text-(--rg-mu) hover:text-(--rg-tx) hover:bg-(--rg-hover)";
</script>

{#snippet key(label: string)}
  <kbd class="min-w-7.5 h-7.5 px-2 inline-flex items-center justify-center rounded-lg
              bg-(--rg-s2) border border-(--rg-bd2) border-b-2
              font-sans text-[13px] font-semibold text-(--rg-tx2)">
    {#if arrows[label]}<Icon icon={arrows[label]} width="16" />{:else}{label}{/if}
  </kbd>
{/snippet}

{#snippet combo(labels: string[])}
  <span class="flex items-center gap-1">
    {#each labels as label, j (j)}
      {#if j > 0}<span class="text-xs text-(--rg-mu2)">+</span>{/if}
      {@render key(label)}
    {/each}
  </span>
{/snippet}

{#each groups as g (g.group)}
  <OptionGroup title={$t(g.titleKey)} hint={$t(g.hintKey)}>
    {#each SHORTCUT_ACTIONS.filter((a) => a.group === g.group) as item (item.action)}
      {@const isEditing = editing === item.action}
      <OptionItem
        title={$t(item.labelKey)}
        desc={isEditing ? message || $t("settings.shortcut_press") : undefined}
        keywords={shortcuts[item.action].join(" ")}
      >
        {#if overrides[item.action] && !isEditing}
          <button type="button" class={ghostButton} onclick={() => save(item.action, null)}>
            {$t("settings.shortcut_reset")}
          </button>
        {/if}
        <!-- Les touches sont le bouton : un clic, puis la nouvelle combinaison. -->
        <button
          type="button"
          class="flex items-center gap-1.5 p-1 -m-1 rounded-xl cursor-pointer transition-colors
                 {isEditing ? 'ring-2 ring-(--rg-g)' : 'hover:bg-(--rg-hover)'}"
          title={$t("settings.shortcut_edit")}
          aria-label="{$t('settings.shortcut_edit')} : {$t(item.labelKey)}"
          onclick={() => (isEditing ? (editing = null) : startEditing(item.action))}
        >
          {#if isEditing}
            <span class="h-7.5 px-3 inline-flex items-center rounded-lg text-[13px] font-semibold animate-pulse
                         {message ? 'text-red-500' : 'text-(--rg-gtx)'}">
              {$t("settings.shortcut_waiting")}
            </span>
          {:else}
            {#each shortcuts[item.action] as binding, i (binding)}
              {#if i > 0}<span class="text-xs text-(--rg-mu2)">{$t("settings.key_or")}</span>{/if}
              {@render combo(keyLabels(binding))}
            {/each}
          {/if}
        </button>
      </OptionItem>
    {/each}
    {#if g.group === "navigation"}
      <!-- Ces deux-là gardent leurs touches : elles sont communes à toute l'interface. -->
      <OptionItem title={$t("settings.shortcut_range")} desc={$t("settings.shortcut_range_desc")}>
        {@render combo([$t("settings.key_shift"), $t("settings.key_click")])}
      </OptionItem>
      <OptionItem title={$t("settings.shortcut_close")} desc={$t("settings.shortcut_close_desc")} keywords="escape echap">
        {@render combo([$t("settings.key_escape")])}
      </OptionItem>
    {/if}
  </OptionGroup>
{/each}

<OptionGroup title={$t("settings.shortcuts_custom")} hint={$t("settings.shortcuts_custom_hint")}>
  <OptionItem title={$t("settings.shortcuts_reset_all")} desc={$t("settings.shortcuts_reset_all_desc")} disabled={!customized}>
    <button
      type="button"
      disabled={!customized}
      class="h-8 px-3 rounded-lg cursor-pointer text-[13px] font-semibold transition-colors border border-(--rg-bd2)
             text-(--rg-tx) hover:bg-(--rg-hover) disabled:opacity-40 disabled:cursor-default"
      onclick={() => { editing = null; settingsStore.set("shortcuts", ""); }}
    >
      {$t("settings.shortcut_reset")}
    </button>
  </OptionItem>
</OptionGroup>

<OptionGroup title={$t("settings.shortcuts_media")} hint={$t("settings.shortcuts_media_hint")}>
  {#if !mediaEnabled}
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
  {#each media as item (item.icon)}
    <OptionItem title={item.title} keywords="media multimedia">
      <kbd class="w-9.5 h-7.5 inline-flex items-center justify-center rounded-lg
                  bg-(--rg-s2) border border-(--rg-bd2) border-b-2 text-(--rg-tx2)
                  {mediaEnabled ? '' : 'opacity-50'}">
        <Icon icon={item.icon} width="22" />
      </kbd>
    </OptionItem>
  {/each}
</OptionGroup>
