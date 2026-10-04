<script lang="ts">
  import Icon from "@iconify/svelte";
  import { onMount } from "svelte";
  import { settingsStore, type AppSettings } from "$lib/stores/settings/settings.store";
  import { t } from "$lib/i18n";
  import { detectOS } from "$lib/helper/tools/osDetection";
  import OptionGroup from "$lib/components/ui/input/OptionGroup.svelte";
  import OptionItem from "$lib/components/ui/input/OptionItem.svelte";
  import OptionBlock from "$lib/components/ui/input/OptionBlock.svelte";
  import ChoiceCard from "$lib/components/ui/input/ChoiceCard.svelte";
  import SegmentedControl from "$lib/components/ui/input/SegmentedControl.svelte";
  import TogglePill from "$lib/components/ui/input/TogglePill.svelte";
  import ToggleSwitch from "$lib/components/ui/input/ToggleSwitch.svelte";
  import ThemePreview from "$lib/components/settings/ThemePreview.svelte";
  import SidebarPreview from "$lib/components/settings/SidebarPreview.svelte";
  import LibraryLayoutPreview from "$lib/components/settings/LibraryLayoutPreview.svelte";
  import WindowControlsPreview from "$lib/components/settings/WindowControlsPreview.svelte";
  import {
    ONGLETS_BIBLIOTHEQUE,
    ORDRE_ONGLETS,
    lireOnglets,
    lirePlacement,
    type LibraryTabKey,
    type PlacementOnglets,
  } from "$lib/config/libraryTabs";
  import {
    getRenderMode,
    setRenderMode,
    type RenderModeStatus,
    type RenderMode,
  } from "$lib/services/system/renderMode.service";

  // `!== 'false'` : sans réglage en base, le défaut est « visible ».
  const visible = (cle: keyof AppSettings) => $settingsStore[cle] !== "false";

  const themes: { valeur: "auto" | "light" | "dark"; labelKey: string }[] = [
    { valeur: "auto", labelKey: "settings.theme_auto" },
    { valeur: "light", labelKey: "settings.theme_light" },
    { valeur: "dark", labelKey: "settings.theme_dark" },
  ];

  // ─── Barre latérale ───
  const ouvrir = $derived($settingsStore.show_open_buttons === "true");
  const albums = $derived(visible("show_pinned_albums"));
  const artistes = $derived(visible("show_pinned_artists"));

  // ─── Listes automatiques : l'accueil ne montre plus que les récents. ───
  const favoris = $derived(visible("show_favorites"));
  const listes = $derived([
    {
      titre: $t("playlist_page.liked_title"), icone: "material-symbols:favorite-rounded", teinte: 10,
      // Sans cœur, plus rien ne remplit la liste : elle suit le bouton favori.
      coupee: !favoris,
      emplacements: [{ cle: "show_liked_in_playlists" as const, label: $t("nav.playlists") }],
    },
    {
      titre: $t("playlist_page.recent_title"), icone: "material-symbols:history-rounded", teinte: 230,
      coupee: false,
      emplacements: [
        { cle: "show_recent_in_playlists" as const, label: $t("nav.playlists") },
        { cle: "show_recent_in_home" as const, label: $t("nav.home") },
      ],
    },
  ]);

  // ─── Sections de la bibliothèque ───
  const placement = $derived(lirePlacement($settingsStore.library_tabs_position));
  const ongletsRetenus = $derived(lireOnglets($settingsStore.library_tabs));
  const placements: { valeur: PlacementOnglets; labelKey: string }[] = [
    { valeur: "sidebar", labelKey: "settings.library_nav_sidebar" },
    { valeur: "top", labelKey: "settings.library_nav_top" },
    { valeur: "both", labelKey: "settings.library_nav_both" },
  ];

  /** Conserve l'ordre d'origine : la voir sauter en dernier désorienterait. */
  function basculerOnglet(cle: LibraryTabKey) {
    const suivant = ongletsRetenus.includes(cle)
      ? ongletsRetenus.filter((c) => c !== cle)
      : ORDRE_ONGLETS.filter((c) => c === cle || ongletsRetenus.includes(c));

    // Le bouton est déjà désactivé, mais un réglage importé peut arriver vide.
    if (suivant.length === 0) return;
    settingsStore.set("library_tabs", JSON.stringify(suivant));
  }

  // ─── Boutons de fenêtre : « Auto » montre ce que donne le système détecté ───
  const os = detectOS();
  type StyleFenetre = "macos" | "windows" | "linux";
  const styleSysteme: StyleFenetre = os === "macos" ? "macos" : os === "linux" ? "linux" : "windows";
  const nomsStyles = { macos: "macOS", windows: "Windows", linux: "Linux" } as const;
  const stylesFenetre = $derived<{ valeur: string; apercu: StyleFenetre; label: string; title?: string }[]>([
    {
      valeur: "auto", apercu: styleSysteme, label: $t("settings.window_controls_auto"),
      title: $t("settings.window_controls_auto_hint").replace("{os}", nomsStyles[styleSysteme]),
    },
    ...(["macos", "windows", "linux"] as const).map((v) => ({ valeur: v, apercu: v, label: nomsStyles[v] })),
  ]);
  const cotePosition = $derived($settingsStore.window_controls_position === "left" ? "left" : "right");

  // ─── Mode de rendu : variables WebKit, n'agit que sous Linux ───
  const surLinux = os === "linux";
  let renderMode = $state<RenderModeStatus | null>(null);
  let renderModeSaving = $state(false);
  let renderModeChanged = $state(false);
  let renderModeInitial: RenderMode | null = null;

  onMount(async () => {
    if (!surLinux) return;
    try {
      renderMode = await getRenderMode();
      renderModeInitial = renderMode.mode;
    } catch (e) {
      console.error("[render mode] init failed:", e);
    }
  });

  async function handleRenderModeChange(value: RenderMode) {
    if (renderModeSaving) return;
    renderModeSaving = true;
    try {
      renderMode = await setRenderMode(value);
      // Les variables d'environnement sont figées au démarrage.
      renderModeChanged = renderModeInitial !== null && renderModeInitial !== value;
    } catch (e) {
      console.error("[render mode] save failed:", e);
    } finally {
      renderModeSaving = false;
    }
  }

  const descRendu = $derived(
    [
      $t("settings.render_mode_desc"),
      renderMode?.virt_kind ? $t("settings.render_mode_vm_detected").replace("{kind}", renderMode.virt_kind) : "",
      renderModeChanged ? $t("settings.render_mode_restart_required") : "",
    ].filter(Boolean).join(" "),
  );
</script>

<!-- ─── Thème ─── -->
<OptionGroup title={$t("settings.theme")} hint={$t("settings.theme_desc")}>
  <OptionBlock keywords="{$t('settings.theme')} {themes.map((th) => $t(th.labelKey)).join(' ')} theme" class="p-4">
    <div class="grid grid-cols-3 gap-3" role="radiogroup" aria-label={$t("settings.theme")}>
      {#each themes as th (th.valeur)}
        <ChoiceCard
          label={$t(th.labelKey)}
          selected={$settingsStore.theme === th.valeur}
          onclick={() => settingsStore.set("theme", th.valeur)}
        >
          <ThemePreview mode={th.valeur} />
        </ChoiceCard>
      {/each}
    </div>
  </OptionBlock>
  <OptionItem title={$t("settings.contrast")} desc={$t("settings.contrast_desc")} keywords="contrast">
    <SegmentedControl
      value={$settingsStore.contrast === "high" ? "high" : "normal"}
      options={[
        { value: "normal", label: $t("settings.contrast_normal") },
        { value: "high", label: $t("settings.contrast_high") },
      ]}
      label={$t("settings.contrast")}
      onchange={(v) => settingsStore.set("contrast", v)}
    />
  </OptionItem>
  <OptionItem
    title={$t("settings.help_bubbles")}
    desc={$t("settings.help_bubbles_desc")}
    keywords="aide help glossary dac dsd"
    onclick={() => settingsStore.toggle("show_help_bubbles")}
  >
    <ToggleSwitch
      checked={$settingsStore.show_help_bubbles !== "false"}
      label={$t("settings.help_bubbles")}
      onclick={() => settingsStore.toggle("show_help_bubbles")}
    />
  </OptionItem>
</OptionGroup>

<!-- ─── Barre latérale, avec son aperçu ─── -->
<OptionGroup title={$t("settings.sidebar_group")} hint={$t("settings.sidebar_group_hint")}>
  <div class="grid grid-cols-[minmax(0,1fr)_200px] max-md:grid-cols-1">
    <div class="rg-lignes border-r border-(--rg-line) max-md:border-r-0">
      {#each [
        { cle: "show_open_buttons" as const, titre: $t("settings.open_buttons"), desc: $t("settings.open_buttons_desc"), actif: ouvrir },
        { cle: "show_pinned_albums" as const, titre: $t("settings.pinned_albums"), desc: $t("settings.pinned_albums_desc"), actif: albums },
        { cle: "show_pinned_artists" as const, titre: $t("settings.pinned_artists"), desc: $t("settings.pinned_artists_desc"), actif: artistes },
      ] as ligne (ligne.cle)}
        <OptionItem title={ligne.titre} desc={ligne.desc} keywords="sidebar" onclick={() => settingsStore.toggle(ligne.cle)}>
          <ToggleSwitch checked={ligne.actif} label={ligne.titre} onclick={() => settingsStore.toggle(ligne.cle)} />
        </OptionItem>
      {/each}
    </div>
    <div class="max-md:hidden">
      <SidebarPreview {ouvrir} {albums} {artistes} />
    </div>
  </div>
</OptionGroup>

<!-- ─── Listes et favoris ─── -->
<OptionGroup title={$t("settings.lists_group")} hint={$t("settings.lists_group_hint")}>
  {#each listes as liste (liste.icone)}
    {@const masquee = liste.emplacements.every((e) => !visible(e.cle))}
    <OptionItem
      title={liste.titre}
      desc={liste.coupee ? $t("settings.liked_needs_favorites") : masquee ? $t("settings.builtin_playlists_hidden_note") : undefined}
      keywords={liste.emplacements.map((e) => e.label).join(" ")}
    >
      {#snippet lead()}
        <span class="w-8 h-8 shrink-0 rounded-lg flex items-center justify-center rg-icone-teinte" style:--h={liste.teinte}>
          <Icon icon={liste.icone} width="18" />
        </span>
      {/snippet}
      <div class="flex gap-1.5">
        {#each liste.emplacements as e (e.cle)}
          <TogglePill
            label={e.label}
            checked={visible(e.cle) && !liste.coupee}
            disabled={liste.coupee}
            onclick={() => settingsStore.toggle(e.cle)}
          />
        {/each}
      </div>
    </OptionItem>
  {/each}
  <OptionItem
    title={$t("settings.favorites")}
    desc={$t("settings.favorites_desc")}
    keywords="favoris coeur like heart"
    onclick={() => settingsStore.toggle("show_favorites")}
  >
    <ToggleSwitch checked={favoris} label={$t("settings.favorites")} onclick={() => settingsStore.toggle("show_favorites")} />
  </OptionItem>
</OptionGroup>

<!-- ─── Navigation de la bibliothèque ─── -->
<OptionGroup title={$t("settings.library_nav")} hint={$t("settings.library_nav_hint")}>
  <OptionBlock keywords="{$t('settings.library_nav')} {$t('settings.library_nav_hint')} {placements.map((p) => $t(p.labelKey)).join(' ')}" class="p-4">
    <div class="grid grid-cols-3 gap-3" role="radiogroup" aria-label={$t("settings.library_nav")}>
      {#each placements as p (p.valeur)}
        <ChoiceCard
          label={$t(p.labelKey)}
          selected={placement === p.valeur}
          onclick={() => settingsStore.set("library_tabs_position", p.valeur)}
        >
          <LibraryLayoutPreview placement={p.valeur} />
        </ChoiceCard>
      {/each}
    </div>
  </OptionBlock>
  <OptionBlock
    keywords="{$t('settings.library_tabs_shown')} {$t('settings.library_tabs_note')} {ONGLETS_BIBLIOTHEQUE.map((o) => $t(o.labelKey)).join(' ')}"
    class="px-5 py-4 leading-[1.2]"
  >
    <p class="text-[15px] font-semibold text-(--rg-tx)">{$t("settings.library_tabs_shown")}</p>
    <p class="mt-0.75 text-[13px] leading-[1.25] text-(--rg-mu) text-pretty">{$t("settings.library_tabs_note")}</p>
    <div class="mt-3 flex flex-wrap gap-1.5">
      {#each ONGLETS_BIBLIOTHEQUE as onglet (onglet.key)}
        {@const coche = ongletsRetenus.includes(onglet.key)}
        {@const dernier = coche && ongletsRetenus.length === 1}
        <TogglePill
          label={$t(onglet.labelKey)}
          checked={coche}
          disabled={dernier}
          title={dernier ? $t("settings.library_tabs_last") : undefined}
          onclick={() => basculerOnglet(onglet.key)}
        />
      {/each}
    </div>
  </OptionBlock>
</OptionGroup>

<!-- ─── Fenêtre ─── -->
<OptionGroup title={$t("settings.window_group")} hint={$t("settings.window_group_hint")}>
  <OptionBlock keywords="{$t('settings.window_controls_style')} {$t('settings.window_group_hint')} macos windows linux" class="p-4">
    <div class="grid grid-cols-4 max-sm:grid-cols-2 gap-3" role="radiogroup" aria-label={$t("settings.window_controls_style")}>
      {#each stylesFenetre as st (st.valeur)}
        <ChoiceCard
          label={st.label}
          title={st.title}
          selected={($settingsStore.window_controls_style || "auto") === st.valeur}
          onclick={() => settingsStore.set("window_controls_style", st.valeur)}
        >
          <WindowControlsPreview style={st.apercu} position={cotePosition} />
        </ChoiceCard>
      {/each}
    </div>
  </OptionBlock>
  <OptionItem title={$t("settings.window_controls_position")} desc={$t("settings.window_controls_position_desc")}>
    <SegmentedControl
      value={cotePosition}
      options={[
        { value: "left", label: $t("settings.left") },
        { value: "right", label: $t("settings.right") },
      ]}
      label={$t("settings.window_controls_position")}
      onchange={(v) => settingsStore.set("window_controls_position", v)}
    />
  </OptionItem>
  {#if renderMode}
    <OptionItem title={$t("settings.render_mode")} desc={descRendu} keywords="gpu webkit">
      <SegmentedControl
        value={renderMode.mode}
        options={[
          { value: "auto", label: $t("settings.auto") },
          { value: "force-gpu", label: "GPU" },
          { value: "force-software", label: $t("settings.render_mode_software_short") },
        ]}
        label={$t("settings.render_mode")}
        onchange={(v) => handleRenderModeChange(v as RenderMode)}
      />
    </OptionItem>
  {/if}
</OptionGroup>
