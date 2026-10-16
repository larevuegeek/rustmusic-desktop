<script lang="ts">
  // Télécommande d'ampli : panneau rangé au bord droit, sorti par sa languette.
  // Afficheur façon façade (VFD), volume dessous ; mode mini réduit à l'afficheur et au volume.
  import Icon from "@iconify/svelte";
  import { goto } from "$app/navigation";
  import { t } from "#lib/i18n";
  import ToggleSwitch from "#lib/components/ui/input/ToggleSwitch.svelte";
  import AmpRemoteNav from "#lib/components/remote/AmpRemoteNav.svelte";
  import { settingsStore } from "#lib/stores/settings/settings.store";
  import { ampRemoteStore, type AmpBrand, type AmpChoice, type AmpZone, type AmpZoneState } from "#lib/stores/audio/ampRemote.store";

  const MARQUE: Record<AmpBrand, string> = { onkyo: "Onkyo", denon: "Denon", yamaha: "Yamaha" };

  const actif = $derived($settingsStore.amp_remote === "true");
  const ouvert = $derived($settingsStore.amp_remote_open === "true");
  const mini = $derived($settingsStore.amp_remote_mini === "true");
  const largeur = 340;
  const vue = $derived($ampRemoteStore.view);
  const etat = $derived(vue?.state);
  const caps = $derived(vue?.caps);
  const allume = $derived(!!etat?.connected && !!etat.main.power);

  // Volume en cours de glissement : affiché tout de suite, envoyé au relâché.
  let glisse = $state<{ zone: AmpZone; valeur: number } | null>(null);
  let onglet = $state<"sources" | "son" | "navigation">("sources");

  const memeId = (a: string | null | undefined, b: string) => a?.toUpperCase() === b.toUpperCase();
  const nomEntree = (id: string | null | undefined) => caps?.inputs.find((i) => memeId(id, i.id))?.name ?? "";
  const entreePrincipale = $derived(nomEntree(etat?.main.input));

  // Passage sur l'entrée réseau : l'écran de navigation s'ouvre de lui-même.
  let derniereEntree = "";
  $effect(() => {
    const entree = entreePrincipale;
    if (entree !== derniereEntree) {
      if (/^(net|usb)/i.test(entree)) onglet = "navigation";
      derniereEntree = entree;
    }
  });

  // Icône d'une entrée d'après son nom, tel que l'ampli le donne.
  const ICONES_ENTREE: [RegExp, string][] = [
    [/bd|dvd|blu-?ray/i, "material-symbols:movie-outline-rounded"],
    [/^tv/i, "material-symbols:tv-outline-rounded"],
    [/cbl|sat|cable/i, "material-symbols:satellite-alt-outline-rounded"],
    [/strm|stream|media/i, "material-symbols:cast-rounded"],
    [/game/i, "material-symbols:sports-esports-outline-rounded"],
    [/^pc/i, "material-symbols:computer-outline-rounded"],
    [/^cd/i, "material-symbols:album-outline-rounded"],
    [/phono/i, "material-symbols:radio-button-checked-outline"],
    [/^(fm|am|tuner|dab)/i, "material-symbols:radio-outline-rounded"],
    [/^net/i, "material-symbols:lan-outline-rounded"],
    [/bluetooth/i, "material-symbols:bluetooth-rounded"],
    [/usb/i, "material-symbols:usb-rounded"],
    [/aux/i, "material-symbols:cable-rounded"],
    [/hdmi|video/i, "material-symbols:settings-input-hdmi-outline-rounded"],
  ];
  const iconeEntree = (nom: string) => ICONES_ENTREE.find(([m]) => m.test(nom))?.[1] ?? "material-symbols:input-rounded";

  // Sources rangées en deux familles : image d'un côté, son et radio de l'autre.
  const VIDEO = /bd|dvd|blu-?ray|^tv|cbl|sat|strm|stream|game|^pc|hdmi|video/i;
  const familles = $derived.by(() => {
    const liste = caps?.inputs ?? [];
    return [
      { cle: "amp_remote.group_video", items: liste.filter((c) => VIDEO.test(c.name)) },
      { cle: "amp_remote.group_audio", items: liste.filter((c) => !VIDEO.test(c.name)) },
    ].filter((f) => f.items.length);
  });

  const send = ampRemoteStore.send;
  const volume = (zone: AmpZone, z: AmpZoneState) => (glisse?.zone === zone ? glisse.valeur : z.volume);
  const pas = $derived(caps?.volume_step ?? 1);
  const maxi = $derived(caps?.volume_max || 100);

  function affiche(v: number | null): string {
    if (v === null) return "—";
    return pas < 1 ? v.toFixed(1) : String(Math.round(v));
  }

  // Chiffres 7 segments : « 8 » fantômes derrière, « ! » = case éteinte dans DSEG7.
  const chiffres = $derived(String(Math.floor(maxi)).length);
  const fantome = $derived("8".repeat(chiffres) + (pas < 1 ? ".8" : ""));
  function segments(v: number | null): string {
    if (v === null) return "-".repeat(chiffres);
    const [ent, dec] = (pas < 1 ? v.toFixed(1) : String(Math.round(v))).split(".");
    return ent.padStart(chiffres, "!") + (dec ? `.${dec}` : "");
  }

  function lacher(zone: AmpZone) {
    if (glisse?.zone === zone) send({ type: "volume", zone, value: glisse.valeur });
    glisse = null;
  }

  // −/+ maintenus : un pas, puis répétition.
  let tRepete: ReturnType<typeof setTimeout> | undefined;
  let iRepete: ReturnType<typeof setInterval> | undefined;
  function repeter(zone: AmpZone, up: boolean) {
    arreter();
    send({ type: "volume_step", zone, up });
    tRepete = setTimeout(() => (iRepete = setInterval(() => send({ type: "volume_step", zone, up }), 120)), 380);
  }
  function arreter() {
    clearTimeout(tRepete);
    clearInterval(iRepete);
  }

  // Mise en veille : appui maintenu, pour éviter les coupures par mégarde.
  let maintien = $state(false);
  let indice = $state(false);
  let tMaintien: ReturnType<typeof setTimeout> | undefined;
  let tIndice: ReturnType<typeof setTimeout> | undefined;
  function appuiMarche(e: PointerEvent) {
    if (!etat?.connected || e.button !== 0) return;
    if (!allume) return void send({ type: "power", zone: "main", on: true });
    maintien = true;
    indice = false;
    tMaintien = setTimeout(() => {
      maintien = false;
      send({ type: "power", zone: "main", on: false });
    }, 800);
  }
  function relacheMarche() {
    if (!maintien) return;
    clearTimeout(tMaintien);
    maintien = false;
    indice = true;
    clearTimeout(tIndice);
    tIndice = setTimeout(() => (indice = false), 1600);
  }
  // Au clavier (detail 0), une simple validation suffit.
  function clicMarche(e: MouseEvent) {
    if (e.detail === 0 && etat?.connected) send({ type: "power", zone: "main", on: !allume });
  }

  $effect(() => () => {
    arreter();
    clearTimeout(tMaintien);
    clearTimeout(tIndice);
  });

  // Molette sur l'afficheur : volume, sans faire défiler le panneau.
  function molette(node: HTMLElement) {
    let dernier = 0;
    const f = (e: WheelEvent) => {
      if (!allume) return;
      e.preventDefault();
      const maintenant = performance.now();
      if (maintenant - dernier < 70 || !e.deltaY) return;
      dernier = maintenant;
      send({ type: "volume_step", zone: "main", up: e.deltaY < 0 });
    };
    node.addEventListener("wheel", f, { passive: false });
    return () => node.removeEventListener("wheel", f);
  }

  // Flèches ↑↓ et M quand la télécommande a le focus.
  function touches(e: KeyboardEvent) {
    if (!allume || e.target instanceof HTMLInputElement) return;
    if (e.key === "ArrowUp" || e.key === "ArrowDown") {
      e.preventDefault();
      e.stopPropagation();
      send({ type: "volume_step", zone: "main", up: e.key === "ArrowUp" });
    } else if (e.key === "m" || e.key === "M") {
      e.stopPropagation();
      send({ type: "mute", zone: "main", on: !etat?.main.muted });
    }
  }

  function sourceVoisine(d: number) {
    const liste = caps?.inputs ?? [];
    if (!liste.length) return;
    const i = liste.findIndex((c) => memeId(etat?.main.input, c.id));
    send({ type: "input", zone: "main", id: liste[(i + d + liste.length) % liste.length].id });
  }

  // Voyants de l'afficheur : seulement ce que l'ampli nous dit vraiment.
  const voyants = $derived.by(() => {
    const m = (etat?.mode?.name ?? "").toLowerCase();
    return [
      ["STEREO", /st[eé]r[eé]o|direct|pure/.test(m)],
      ["DOLBY", m.includes("dolby")],
      ["DTS", m.includes("dts")],
      ["NET", /^(net|usb|bluetooth)/i.test(entreePrincipale)],
      ["ZONE2", !!etat?.zone2?.power],
      ["MUTING", !!etat?.main.muted],
    ] as const;
  });
  const code = $derived((entreePrincipale || "—").toUpperCase());
  const tailleCode = $derived(code.length <= 6 ? 28 : code.length <= 9 ? 21 : 16);

  const dB = (v: number) => `${v > 0 ? "+" : v < 0 ? "−" : ""}${Math.abs(v)} dB`;
  const basculerMini = () => settingsStore.set("amp_remote_mini", mini ? "false" : "true");

  const ONGLETS = [
    { id: "sources", cle: "amp_remote.tab_sources", icone: "material-symbols:input-rounded" },
    { id: "son", cle: "amp_remote.tab_sound", icone: "material-symbols:graphic-eq-rounded" },
    { id: "navigation", cle: "amp_remote.tab_nav", icone: "material-symbols:gamepad-outline-rounded" },
  ] as const;

  const rond = "flex items-center justify-center rounded-full cursor-pointer transition-colors disabled:cursor-default disabled:opacity-40";
  const fantomeBtn = `${rond} w-9 h-9 shrink-0 text-(--rg-mu) hover:text-(--rg-tx) hover:bg-(--rg-s2)`;
  const carre = "flex items-center justify-center shrink-0 rounded-[11px] cursor-pointer transition-colors touch-none";
  const touche = `${carre} bouton-relief border border-(--rg-bd) text-(--rg-tx)`;
</script>

{#snippet titre(texte: string, extra?: string)}
  <p class="flex items-center gap-2 px-1 pb-1.5 text-[10.5px] font-semibold uppercase tracking-[0.12em] text-(--rg-mu)">
    <span class="flex-1">{texte}</span>{#if extra}<span class="normal-case tracking-normal text-(--rg-gtx) truncate">{extra}</span>{/if}
  </p>
{/snippet}

<!-- Bouton marche : appui maintenu pour la veille, anneau rouge qui se remplit. -->
{#snippet marche(taille: string)}
  <button
    type="button"
    class="{rond} {taille} relative shrink-0 overflow-hidden border touch-none
           {allume ? 'border-(--rg-g)/30 bg-(--rg-s2) text-(--rg-g)' : 'border-(--rg-bd) bg-(--rg-s2) text-(--rg-mu) hover:text-(--rg-tx)'}"
    title={allume ? $t("amp_remote.hold_power") : $t("amp_remote.power_on")}
    aria-label={allume ? $t("amp_remote.power_off") : $t("amp_remote.power_on")}
    aria-pressed={allume}
    disabled={!etat?.connected}
    onpointerdown={appuiMarche}
    onpointerup={relacheMarche}
    onpointerleave={relacheMarche}
    onpointercancel={relacheMarche}
    onclick={clicMarche}
  >
    <span class="absolute inset-0 rounded-full bg-[#e2675a] {maintien ? 'scale-100 duration-800 ease-linear' : 'scale-0 duration-150 ease-out'} transition-transform"></span>
    <Icon icon="material-symbols:power-settings-new-rounded" width="21" class="relative {maintien ? 'text-white' : ''}" />
  </button>
{/snippet}

<!-- −/+ maintenus et sourdine, communs à la vue complète, au mode mini et à la zone 2. -->
{#snippet moinsPlus(zone: AmpZone, taille: string)}
  <button type="button" class="{touche} {taille}" aria-label="{$t('amp_remote.volume')} −"
          onpointerdown={(e) => e.button === 0 && repeter(zone, false)} onpointerup={arreter} onpointerleave={arreter} onpointercancel={arreter}
          onclick={(e) => e.detail === 0 && send({ type: "volume_step", zone, up: false })}>
    <Icon icon="material-symbols:remove-rounded" width="21" />
  </button>
{/snippet}
{#snippet plus(zone: AmpZone, taille: string)}
  <button type="button" class="{touche} {taille}" aria-label="{$t('amp_remote.volume')} +"
          onpointerdown={(e) => e.button === 0 && repeter(zone, true)} onpointerup={arreter} onpointerleave={arreter} onpointercancel={arreter}
          onclick={(e) => e.detail === 0 && send({ type: "volume_step", zone, up: true })}>
    <Icon icon="material-symbols:add-rounded" width="21" />
  </button>
{/snippet}
{#snippet sourdine(zone: AmpZone, z: AmpZoneState, taille: string)}
  <button
    type="button"
    class="{carre} {taille} border {z.muted ? 'sourdine-on' : 'bouton-relief border-(--rg-bd) text-(--rg-tx)'}"
    title={$t("amp_remote.mute")}
    aria-label={$t("amp_remote.mute")}
    aria-pressed={z.muted}
    onclick={() => send({ type: "mute", zone, on: !z.muted })}
  >
    <Icon icon={z.muted ? "material-symbols:volume-off-rounded" : "material-symbols:volume-up-rounded"} width="20" />
  </button>
{/snippet}

<!-- Rangée volume : sourdine, −, curseur, +. -->
{#snippet rangeeVolume(zone: AmpZone, z: AmpZoneState)}
  {@const v = volume(zone, z)}
  <div class="flex items-center gap-1.5">
    {@render sourdine(zone, z, "w-10 h-10")}
    {@render moinsPlus(zone, "w-10 h-10")}
    <input
      type="range"
      class="curseur flex-1 min-w-0 {z.muted ? 'curseur-muet' : ''}"
      min="0"
      max={maxi}
      step={pas}
      value={v ?? 0}
      style="--rempli: {((v ?? 0) / maxi) * 100}%"
      aria-label={$t("amp_remote.volume")}
      oninput={(e) => (glisse = { zone, valeur: Number(e.currentTarget.value) })}
      onchange={() => lacher(zone)}
    />
    {@render plus(zone, "w-10 h-10")}
  </div>
{/snippet}

<!-- Grille de choix : tuiles à icône (sources) ou simples pastilles (modes, zone 2). -->
{#snippet choix(liste: AmpChoice[], courant: string | null | undefined, choisir: (id: string) => void, colonnes: string, icones: boolean)}
  <div class="grid {colonnes} gap-1.5">
    {#each liste as c (c.id)}
      {@const sel = memeId(courant, c.id)}
      <button
        type="button"
        class="min-w-0 flex items-center justify-center rounded-xl border cursor-pointer transition-colors px-1
               {icones ? 'h-14 flex-col gap-1' : 'h-9'}
               {sel ? 'tuile-active' : 'border-(--rg-bd) bg-(--rg-s2)/50 text-(--rg-tx2) hover:text-(--rg-tx) hover:border-(--rg-bd2)'}"
        title={c.name}
        aria-pressed={sel}
        onclick={() => choisir(c.id)}
      >
        {#if icones}<Icon icon={iconeEntree(c.name)} width="20" />{/if}
        <span class="max-w-full truncate {icones ? 'text-[11px]' : 'text-[12px]'} font-semibold">{c.name}</span>
      </button>
    {/each}
  </div>
{/snippet}

<!-- Graves, aigus : − valeur +. -->
{#snippet niveau(libelle: string, valeur: number | null, regler: (v: number) => void, premier: boolean)}
  {@const plage = caps?.tone}
  {#if plage && valeur !== null}
    <div class="flex items-center gap-2 py-1 pl-3 pr-1 {premier ? '' : 'border-t border-(--rg-line)'}">
      <span class="flex-1 text-[13px] text-(--rg-tx)">{libelle}</span>
      <button type="button" class="{touche} w-9 h-9" aria-label="{libelle} −" disabled={valeur <= plage.min}
              onclick={() => regler(Math.max(plage.min, valeur - plage.step))}>
        <Icon icon="material-symbols:remove-rounded" width="19" />
      </button>
      <span class="w-15 text-center tabular-nums font-mono text-[12.5px] {valeur === 0 ? 'text-(--rg-mu)' : 'text-(--rg-tx)'}">{dB(valeur)}</span>
      <button type="button" class="{touche} w-9 h-9" aria-label="{libelle} +" disabled={valeur >= plage.max}
              onclick={() => regler(Math.min(plage.max, valeur + plage.step))}>
        <Icon icon="material-symbols:add-rounded" width="19" />
      </button>
    </div>
  {/if}
{/snippet}

<!-- Écran fluorescent : trame et reflet par-dessus. -->
{#snippet trame()}
  <span class="absolute inset-0 pointer-events-none vfd-trame"></span>
  <span class="absolute inset-x-0 top-0 h-[42%] pointer-events-none vfd-reflet"></span>
{/snippet}

{#if actif && vue && etat && caps}
  <div
    class="fixed right-0 top-21 z-30 flex items-start transition-transform duration-300 ease-out"
    style="transform: translateX({ouvert ? 0 : largeur}px)"
  >
    <!-- Languette : toujours visible, elle sort et range la télécommande. -->
    <button
      type="button"
      class="languette {mini ? 'mt-3 h-24' : 'mt-10 h-30'} w-10 flex flex-col items-center justify-center gap-3 rounded-l-2xl border border-r-0 cursor-pointer
             border-(--rg-bd) text-(--rg-mu) hover:text-(--rg-tx) transition-colors"
      title={ouvert ? $t("amp_remote.close") : $t("amp_remote.open")}
      aria-label={ouvert ? $t("amp_remote.close") : $t("amp_remote.open")}
      aria-expanded={ouvert}
      onclick={() => settingsStore.set("amp_remote_open", ouvert ? "false" : "true")}
    >
      <Icon icon={ouvert ? "material-symbols:chevron-right-rounded" : "material-symbols:settings-remote-outline-rounded"} width="21" />
      <span class="w-1.5 h-1.5 rounded-full {!etat.connected ? 'bg-(--rg-am)' : allume ? 'bg-(--rg-g) shadow-[0_0_8px_var(--rg-g)]' : 'bg-(--rg-off)'}"></span>
    </button>

    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <aside
      class="panneau max-h-[calc(100dvh-230px)] max-[1250px]:max-h-[calc(100dvh-330px)] overflow-y-auto scrollbar-app rounded-l-[22px] border border-r-0 border-(--rg-bd) text-(--rg-tx) select-none
             {mini ? 'p-2 flex flex-col gap-2' : 'p-3 flex flex-col gap-2.5'}"
      style="width: {largeur}px"
      aria-label={$t("amp_remote.panel")}
      inert={!ouvert}
      onkeydown={touches}
    >
      {#if mini}
        <!-- Mode mini : afficheur (clic = agrandir), puis marche, source et volume. -->
        <button
          type="button"
          class="vfd relative h-14.5 px-3 pr-7.5 flex items-center justify-between gap-2.5 rounded-xl overflow-hidden cursor-pointer text-left"
          title={$t("amp_remote.expand")}
          aria-label={$t("amp_remote.expand")}
          {@attach molette}
          onclick={basculerMini}
        >
          <Icon icon="material-symbols:open-in-full-rounded" width="15" class="absolute top-1.5 right-1.75 text-[#4fa98c]" />
          <span class="min-w-0 flex flex-col gap-1.25">
            <span class="dot text-[20px] leading-none tracking-[0.08em] truncate {allume ? 'vfd-lum' : 'text-[#2f6a58]'}">
              {allume ? code : !etat.connected ? "OFFLINE" : "STANDBY"}
            </span>
            <span class="dot text-[11px] leading-none tracking-[0.08em] truncate {allume ? 'text-[#4fa98c]' : 'text-[#1d4a3c]'}">
              {!allume ? vue.amp.name.toUpperCase() : etat.main.muted ? "MUTING" : (etat.mode?.name ?? "").toUpperCase()}
            </span>
          </span>
          <span class="seg relative shrink-0 text-[30px] leading-none">
            <span class="text-[#0f231d]">{fantome}</span>
            <span class="absolute right-0 top-0 {allume && !etat.main.muted ? 'vfd-lum' : 'text-[#2f6a58]'}">{allume ? segments(volume("main", etat.main)) : ""}</span>
          </span>
          {@render trame()}
        </button>
        <div class="flex items-center gap-1.5">
          {@render marche("w-9.5 h-9.5")}
          <div class="flex-1 min-w-0 h-9.5 flex items-center rounded-[11px] border border-(--rg-bd) bg-(--rg-s2)/50">
            <button type="button" class="w-6 h-9.5 shrink-0 flex items-center justify-center cursor-pointer text-(--rg-mu) hover:text-(--rg-tx) disabled:opacity-40"
                    aria-label={$t("amp_remote.prev_input")} title={$t("amp_remote.prev_input")} disabled={!allume} onclick={() => sourceVoisine(-1)}>
              <Icon icon="material-symbols:chevron-left-rounded" width="20" />
            </button>
            <span class="flex-1 min-w-0 flex items-center justify-center gap-1.25 text-[12.5px] font-semibold {allume ? 'text-(--rg-gtx)' : 'text-(--rg-mu)'}">
              <Icon icon={iconeEntree(entreePrincipale)} width="17" class="shrink-0" /><span class="truncate">{entreePrincipale || "—"}</span>
            </span>
            <button type="button" class="w-6 h-9.5 shrink-0 flex items-center justify-center cursor-pointer text-(--rg-mu) hover:text-(--rg-tx) disabled:opacity-40"
                    aria-label={$t("amp_remote.next_input")} title={$t("amp_remote.next_input")} disabled={!allume} onclick={() => sourceVoisine(1)}>
              <Icon icon="material-symbols:chevron-right-rounded" width="20" />
            </button>
          </div>
          {#if allume}
            {@render sourdine("main", etat.main, "w-9.5 h-9.5")}
            {@render moinsPlus("main", "w-9.5 h-9.5")}
            {@render plus("main", "w-9.5 h-9.5")}
          {/if}
        </div>
      {:else}
        <!-- En-tête : l'ampli, le mode mini, les réglages et la mise en marche. -->
        <div class="flex items-center gap-2.5 pl-0.5">
          <span class="w-9.5 h-9.5 shrink-0 flex items-center justify-center rounded-[11px] border
                       {allume ? 'bg-(--rg-g)/10 border-(--rg-g)/25 text-(--rg-g)' : 'bg-(--rg-s2) border-(--rg-bd) text-(--rg-mu)'}">
            <Icon icon="material-symbols:speaker-outline-rounded" width="22" />
          </span>
          <div class="flex-1 min-w-0 leading-[1.25]">
            <p class="text-[15px] font-semibold tracking-[-0.01em] truncate">{vue.amp.name}</p>
            <p class="flex items-center gap-1.5 text-[12px] truncate {etat.connected ? 'text-(--rg-mu)' : 'text-(--rg-am)'}">
              <span class="w-1.5 h-1.5 shrink-0 rounded-full {!etat.connected ? 'bg-(--rg-am)' : allume ? 'bg-(--rg-g)' : 'bg-(--rg-off)'}"></span>
              <!-- Le modèle quand le nom ne le dit pas : un Marantz passe par le pilote Denon. -->
              {etat.connected ? (allume ? `${vue.amp.model && vue.amp.model !== vue.amp.name ? vue.amp.model : MARQUE[vue.amp.brand]} · ${vue.amp.ip}` : $t("amp_remote.standby")) : $t("amp_remote.reconnecting")}
            </p>
          </div>
          <button type="button" class={fantomeBtn} title={$t("amp_remote.mini")} aria-label={$t("amp_remote.mini")} onclick={basculerMini}>
            <Icon icon="material-symbols:close-fullscreen-rounded" width="19" />
          </button>
          <button type="button" class={fantomeBtn} title={$t("amp_remote.settings")} aria-label={$t("amp_remote.settings")} onclick={() => goto("/settings/remote")}>
            <Icon icon="material-symbols:settings-outline-rounded" width="20" />
          </button>
          {@render marche("w-10 h-10")}
        </div>

        {#if indice}
          <p class="self-end -mt-1 px-2.5 py-1.5 rounded-lg bg-(--rg-s2) border border-(--rg-bd) text-[11.5px] text-(--rg-tx)">{$t("amp_remote.hold_power")}</p>
        {/if}

        {#if !etat.connected}
          <p class="px-3.5 py-3 rounded-[14px] bg-(--rg-am)/10 text-[12px] leading-snug text-(--rg-am)">
            {$t("amp_remote.unreachable").replace("{name}", vue.amp.name)}
          </p>
        {:else if !allume}
          <button
            type="button"
            class="h-52 flex flex-col items-center justify-center gap-3 rounded-[18px] border border-dashed border-(--rg-bd2) bg-(--rg-s2)/40 cursor-pointer
                   text-(--rg-mu) hover:text-(--rg-tx) transition-colors"
            onclick={() => send({ type: "power", zone: "main", on: true })}
          >
            <span class="w-18 h-18 flex items-center justify-center rounded-full bg-(--rg-s2) border border-(--rg-bd) text-(--rg-g)">
              <Icon icon="material-symbols:power-settings-new-rounded" width="34" />
            </span>
            <span class="text-[14px] font-medium text-(--rg-tx)">{$t("amp_remote.standby")}</span>
            <span class="text-[12.5px]">{$t("amp_remote.tap_power_on")}</span>
          </button>
        {:else}
          <!-- Afficheur et volume : la façade de l'ampli. -->
          {@const v = volume("main", etat.main)}
          <div class="rounded-[18px] border border-(--rg-bd) bg-(--rg-s2)/50 p-2 pb-2.5 flex flex-col gap-2">
            <div class="vfd relative rounded-xl overflow-hidden px-3 pt-2.25 pb-2.5" {@attach molette}>
              <div class="dot flex justify-between gap-1 text-[9.5px] leading-none tracking-[0.06em]">
                {#each voyants as [libelle, allumeV] (libelle)}
                  <span class={allumeV ? "vfd-lum" : "text-[#11261f]"}>{libelle}</span>
                {/each}
              </div>
              <!-- Source et volume, puis le mode d'écoute sur toute la largeur. -->
              <div class="mt-2 flex items-end justify-between gap-2.5">
                <span class="dot min-w-0 pb-0.5 vfd-lum leading-none tracking-[0.08em] truncate" style="font-size: {tailleCode}px">{code}</span>
                <span class="seg relative shrink-0 text-[46px] leading-none">
                  <span class="text-[#0f231d]">{fantome}</span>
                  <span class="absolute right-0 top-0 {etat.main.muted ? 'text-[#2f6a58]' : 'vfd-lum vfd-lum-fort'}">{segments(v)}</span>
                </span>
              </div>
              <div class="mt-1.5 flex items-center justify-between gap-2">
                <button
                  type="button"
                  class="dot min-w-0 h-7 -ml-2 px-2 flex items-center gap-1.5 rounded-md border border-transparent cursor-pointer
                         text-[13px] tracking-[0.06em] vfd-lum hover:border-[#8cf7d4]/25"
                  title={$t("amp_remote.mode")}
                  onclick={() => (onglet = "son")}
                ><span class="truncate">{(etat.mode?.name ?? "—").toUpperCase()}</span><span class="shrink-0 text-[10px] opacity-70">▶</span></button>
                <span class="dot shrink-0 text-[11px] leading-none tracking-[0.08em] {etat.main.muted ? 'vfd-lum' : 'text-[#4fa98c]'}">
                  {etat.main.muted ? "MUTING" : `VOL / ${affiche(maxi)}`}
                </span>
              </div>
              {@render trame()}
            </div>
            {@render rangeeVolume("main", etat.main)}
          </div>

          <div class="p-0.75 grid grid-cols-3 gap-0.75 rounded-[14px] border border-(--rg-bd) bg-(--rg-s2)/50" role="tablist">
            {#each ONGLETS as o (o.id)}
              <button
                type="button"
                role="tab"
                aria-selected={onglet === o.id}
                class="h-9 flex items-center justify-center gap-1.5 rounded-[11px] text-[12.5px] font-medium cursor-pointer transition-colors
                       {onglet === o.id ? 'onglet-actif text-(--rg-tx)' : 'text-(--rg-mu) hover:text-(--rg-tx)'}"
                onclick={() => (onglet = o.id)}
              >
                <Icon icon={o.icone} width="17" />{$t(o.cle)}
              </button>
            {/each}
          </div>

          <div class="min-h-66">
            {#if onglet === "sources"}
              <div class="flex flex-col gap-2.5">
                {#each familles as f (f.cle)}
                  <section>
                    {#if familles.length > 1}{@render titre($t(f.cle))}{/if}
                    {@render choix(f.items, etat.main.input, (id) => send({ type: "input", zone: "main", id }), "grid-cols-4", true)}
                  </section>
                {/each}
              </div>
            {:else if onglet === "son"}
              <div class="flex flex-col gap-2.5">
                {#if caps.modes.length}
                  <section>
                    {@render titre($t("amp_remote.mode"))}
                    {@render choix(caps.modes, etat.mode?.id, (id) => send({ type: "mode", id }), "grid-cols-3", false)}
                  </section>
                {/if}
                {#if caps.tone && (etat.bass !== null || etat.treble !== null)}
                  <section>
                    {@render titre($t("amp_remote.tone"))}
                    <div class="rounded-2xl border border-(--rg-bd) bg-(--rg-s2)/50">
                      {@render niveau($t("amp_remote.bass"), etat.bass, (value) => send({ type: "bass", value }), true)}
                      {@render niveau($t("amp_remote.treble"), etat.treble, (value) => send({ type: "treble", value }), etat.bass === null)}
                    </div>
                  </section>
                {/if}
                {#if caps.zone2 && etat.zone2}
                  {@const z2 = etat.zone2}
                  <section class="rounded-2xl border border-(--rg-bd) bg-(--rg-s2)/50 p-2.5 flex flex-col gap-2.5">
                    <div class="flex items-center gap-2.5 pl-0.5">
                      <span class="flex-1 min-w-0 leading-[1.25]">
                        <span class="block text-[13px]">{$t("amp_remote.zone2")}</span>
                        <span class="block text-[12px] truncate {z2.power ? 'text-(--rg-gtx)' : 'text-(--rg-mu)'}">
                          {z2.power ? `${nomEntree(z2.input)} · ${affiche(volume("zone2", z2))}` : $t("amp_remote.standby")}
                        </span>
                      </span>
                      <ToggleSwitch size="sm" checked={z2.power} label={$t("amp_remote.zone2")} onclick={() => send({ type: "power", zone: "zone2", on: !z2.power })} />
                    </div>
                    {#if z2.power}
                      {@render rangeeVolume("zone2", z2)}
                      {@render choix(caps.inputs, z2.input, (id) => send({ type: "input", zone: "zone2", id }), "grid-cols-4", false)}
                    {/if}
                  </section>
                {/if}
              </div>
            {:else}
              <AmpRemoteNav {vue} cover={$ampRemoteStore.cover} />
            {/if}
          </div>
        {/if}
      {/if}
    </aside>
  </div>
{/if}

<style>
  /* Polices d'afficheur, tirées de la maquette (OFL) : 7 segments et matricielle. */
  @font-face {
    font-family: "DSEG7";
    src: url("../../assets/fonts/dseg7.woff2") format("woff2");
    font-display: block;
  }
  @font-face {
    font-family: "DotGothic16";
    src: url("../../assets/fonts/dotgothic16.woff2") format("woff2");
    font-display: swap;
  }
  .seg {
    font-family: "DSEG7", ui-monospace, monospace;
  }
  .dot {
    font-family: "DotGothic16", ui-monospace, monospace;
  }

  .panneau {
    background: var(--rg-carte);
    box-shadow: -18px 10px 48px rgb(0 0 0 / 0.22), inset 1px 1px 0 rgb(255 255 255 / 0.04);
  }
  .languette {
    background: var(--rg-carte);
    box-shadow: -8px 4px 20px rgb(0 0 0 / 0.16), inset 1px 1px 0 rgb(255 255 255 / 0.04);
  }

  /* Afficheur fluorescent : toujours sombre, quel que soit le thème. */
  .vfd {
    background: #030605;
    color: #8cf7d4;
    box-shadow: inset 0 3px 14px rgb(0 0 0 / 0.95), inset 0 0 0 1px rgb(255 255 255 / 0.05);
  }
  .vfd-lum {
    color: #8cf7d4;
    text-shadow: 0 0 7px rgb(120 245 200 / 0.55);
  }
  .vfd-lum-fort {
    text-shadow: 0 0 10px rgb(120 245 200 / 0.65);
  }
  .vfd-trame {
    background: repeating-linear-gradient(0deg, rgb(0 0 0 / 0.28) 0 1px, transparent 1px 3px);
  }
  .vfd-reflet {
    background: linear-gradient(180deg, rgb(255 255 255 / 0.045), transparent);
  }

  /* Touches en léger relief. */
  :global(.bouton-relief) {
    background: linear-gradient(180deg, color-mix(in oklch, var(--rg-s2) 92%, white), var(--rg-s2));
    box-shadow: 0 1px 2px rgb(0 0 0 / 0.18), inset 0 1px 0 rgb(255 255 255 / 0.05);
  }
  :global(.bouton-relief:hover:not(:disabled)) {
    background: var(--rg-bd);
  }
  :global(.bouton-relief:active:not(:disabled)) {
    background: var(--rg-bd2);
  }

  .sourdine-on {
    background: color-mix(in oklch, var(--rg-am) 14%, transparent);
    border-color: color-mix(in oklch, var(--rg-am) 45%, transparent);
    color: var(--rg-am);
  }

  :global(.tuile-active) {
    border-color: color-mix(in oklch, var(--rg-g) 45%, transparent);
    background: color-mix(in oklch, var(--rg-g) 14%, transparent);
    color: var(--rg-gtx);
  }

  .onglet-actif {
    background: var(--rg-carte);
    box-shadow: 0 1px 4px rgb(0 0 0 / 0.16), inset 0 1px 0 rgb(255 255 255 / 0.04);
  }

  /* Curseur épais : rail de 8 px, pastille de 22 px, toute la hauteur est cliquable. */
  .curseur {
    appearance: none;
    height: 40px;
    background: transparent;
    cursor: pointer;
    touch-action: none;
  }
  .curseur::-webkit-slider-runnable-track {
    height: 8px;
    border-radius: 999px;
    background: linear-gradient(to right, var(--rg-g) var(--rempli), color-mix(in oklch, var(--rg-tx) 9%, transparent) var(--rempli));
  }
  .curseur-muet::-webkit-slider-runnable-track {
    background: linear-gradient(to right, var(--rg-mu2) var(--rempli), color-mix(in oklch, var(--rg-tx) 9%, transparent) var(--rempli));
  }
  .curseur::-webkit-slider-thumb {
    appearance: none;
    width: 22px;
    height: 22px;
    margin-top: -7px;
    border-radius: 999px;
    background: #ecefed;
    box-shadow: 0 2px 10px rgb(0 0 0 / 0.5);
  }
</style>
