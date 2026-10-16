<script lang="ts">
  // Navigation de l'ampli : écran des services réseau (liste, lecture) et pavé directionnel.
  import Icon from "@iconify/svelte";
  import { t } from "#lib/i18n";
  import { ampRemoteStore, type AmpNavKey, type AmpNetLine, type AmpView } from "#lib/stores/audio/ampRemote.store";
  import ClickWheel from "#lib/components/ui/input/ClickWheel.svelte";
  import SegmentedControl from "#lib/components/ui/input/SegmentedControl.svelte";
  import { lireLocal, ecrireLocal } from "#lib/helper/tools/stockage";

  let { vue, cover }: { vue: AmpView; cover: string | null } = $props();

  const send = ampRemoteStore.send;
  const net = $derived(vue.state.net);
  // Entrée réseau de l'ampli : celle qui s'appelle NET (ou NETWORK).
  const entreeNet = $derived(vue.caps.inputs.find((i) => /^net/i.test(i.name)));
  const nomEntree = $derived(vue.caps.inputs.find((i) => i.id.toUpperCase() === vue.state.main.input?.toUpperCase())?.name ?? "");
  const surNet = $derived(/^(net|usb)/i.test(nomEntree));
  const enLecture = $derived(!!net.track);

  const ICONES: Record<string, string> = {
    playing: "material-symbols:graphic-eq-rounded",
    artist: "material-symbols:person-outline-rounded",
    album: "material-symbols:album-outline-rounded",
    folder: "material-symbols:folder-outline-rounded",
    track: "material-symbols:music-note-rounded",
    playlist: "material-symbols:queue-music-rounded",
    search: "material-symbols:search-rounded",
    account: "material-symbols:account-circle-outline",
  };

  // Services connus : leur logo, comme dans l'appli de l'ampli.
  const SERVICES: [RegExp, string][] = [
    [/^spotify/i, "simple-icons:spotify"],
    [/^deezer/i, "simple-icons:deezer"],
    [/^tunein/i, "simple-icons:tunein"],
    [/^pandora/i, "simple-icons:pandora"],
    [/^dlna/i, "simple-icons:dlna"],
    [/^airplay/i, "simple-icons:airplayaudio"],
    [/^tidal/i, "simple-icons:tidal"],
    [/^amazon/i, "simple-icons:amazonmusic"],
    [/^iheart/i, "simple-icons:iheartradio"],
    [/^soundcloud/i, "simple-icons:soundcloud"],
    [/^home media/i, "material-symbols:home-storage-outline-rounded"],
    [/^usb/i, "material-symbols:usb-rounded"],
    [/favo/i, "material-symbols:star-outline-rounded"],
  ];

  function icone(l: AmpNetLine): string {
    if (l.kind !== "item") return ICONES[l.kind] ?? "material-symbols:chevron-right-rounded";
    return SERVICES.find(([motif]) => motif.test(l.text))?.[1] ?? "material-symbols:chevron-right-rounded";
  }

  const nav = (key: AmpNavKey) => send({ type: "nav", key });

  // Commandes de direction : molette (façon iPod) ou ligne de touches, plus compacte. Retenu.
  type Pave = "molette" | "touches";
  let pave = $state<Pave>(lireLocal("ampli:pave", "molette") === "touches" ? "touches" : "molette");
  $effect(() => ecrireLocal("ampli:pave", pave));
  const TOUCHES: [AmpNavKey, string][] = [
    ["left", "material-symbols:keyboard-arrow-left-rounded"],
    ["up", "material-symbols:keyboard-arrow-up-rounded"],
    ["enter", ""],
    ["down", "material-symbols:keyboard-arrow-down-rounded"],
    ["right", "material-symbols:keyboard-arrow-right-rounded"],
  ];

  /** Garde la ligne du curseur visible dans la liste, sans faire défiler le panneau. */
  function suivreCurseur(actif: boolean) {
    return (li: HTMLElement) => {
      const ul = li.parentElement;
      if (!actif || !ul) return;
      const haut = li.offsetTop;
      const bas = haut + li.offsetHeight;
      if (haut < ul.scrollTop) ul.scrollTop = haut - 4;
      else if (bas > ul.scrollTop + ul.clientHeight) ul.scrollTop = bas - ul.clientHeight + 4;
    };
  }
  const petit = "bouton-relief h-8.5 flex-1 flex items-center justify-center gap-1.5 rounded-[11px] cursor-pointer text-[11.5px] font-semibold text-(--rg-tx2) hover:text-(--rg-tx) transition-colors";
  const transport = "w-9 h-9 flex items-center justify-center rounded-full cursor-pointer text-(--rg-tx2) hover:text-(--rg-tx) hover:bg-(--rg-s2) transition-colors";
</script>

<div class="flex flex-col gap-3.5">
  {#if surNet}
    <!-- Écran de l'ampli : la liste en cours, comme dans son appli. -->
    <div class="viseur rounded-2xl border border-(--rg-bd) overflow-hidden">
      <div class="flex items-center gap-1.5 px-1.5 py-1.5 border-b border-(--rg-bd)">
        <button type="button" class={transport} title={$t("amp_remote.back")} aria-label={$t("amp_remote.back")} onclick={() => nav("back")}>
          <Icon icon="material-symbols:arrow-back-rounded" width="18" />
        </button>
        <div class="flex-1 min-w-0 leading-[1.2]">
          <p class="text-[13px] font-bold truncate">{net.title || nomEntree}</p>
          {#if net.count > 0}<p class="text-[10.5px] text-(--rg-mu)">{$t("amp_remote.items").replace("{n}", String(net.count))}</p>{/if}
        </div>
        <button type="button" class={transport} title={$t("amp_remote.home")} aria-label={$t("amp_remote.home")} onclick={() => nav("home")}>
          <Icon icon="material-symbols:home-outline-rounded" width="18" />
        </button>
      </div>
      {#if net.lines.some((l) => l.text)}
        <!-- Hauteur bornée : la liste défile et suit le curseur au lieu d'allonger le panneau. -->
        <ul class="relative p-1 max-h-60 overflow-y-auto overscroll-contain scrollbar-app">
          {#each net.lines as ligne, i (i)}
            {#if ligne.text}
              {@const sel = net.cursor === i}
              <li {@attach suivreCurseur(sel)}>
                <button
                  type="button"
                  class="relative w-full h-8 flex items-center gap-2.5 px-2.5 rounded-[9px] text-left cursor-pointer transition-colors
                         {sel ? 'bg-(--rg-g)/13 text-(--rg-gtx)' : 'text-(--rg-tx2) hover:bg-(--rg-hover) hover:text-(--rg-tx)'}"
                  onclick={() => send({ type: "net_select", line: i })}
                >
                  {#if sel}<span class="absolute left-0 top-1.5 bottom-1.5 w-0.75 rounded-r-[3px] bg-(--rg-g)"></span>{/if}
                  <Icon icon={icone(ligne)} width="16" class="shrink-0 {sel ? '' : 'text-(--rg-mu)'}" />
                  <span class="flex-1 min-w-0 truncate text-[12.5px] font-semibold">{ligne.text}</span>
                  {#if ligne.kind === "folder" || ligne.kind === "item"}
                    <Icon icon="material-symbols:chevron-right-rounded" width="15" class="shrink-0 text-(--rg-mu2)" />
                  {/if}
                </button>
              </li>
            {/if}
          {/each}
        </ul>
      {:else}
        <p class="px-3 py-4 text-[12px] text-center text-(--rg-mu)">{$t("amp_remote.loading")}</p>
      {/if}
    </div>

    {#if enLecture}
      <!-- Lecture en cours sur l'ampli. -->
      <div class="viseur rounded-2xl border border-(--rg-bd) p-2.5 flex flex-col gap-2">
        <div class="flex items-center gap-2.5">
          {#if cover}
            <img src={cover} alt="" class="w-12 h-12 shrink-0 rounded-[10px] object-cover shadow-[0_4px_12px_rgba(0,0,0,0.35)]" />
          {:else}
            <span class="w-12 h-12 shrink-0 flex items-center justify-center rounded-[8px] bg-(--rg-s2) text-(--rg-mu)">
              <Icon icon="material-symbols:music-note-rounded" width="22" />
            </span>
          {/if}
          <div class="flex-1 min-w-0 leading-[1.25]">
            <p class="text-[13px] font-bold truncate">{net.track}</p>
            <p class="text-[11.5px] text-(--rg-mu) truncate">{[net.artist, net.album].filter(Boolean).join(" · ")}</p>
            {#if net.time}<p class="text-[11px] tabular-nums text-(--rg-gtx)">{net.time}</p>{/if}
          </div>
        </div>
        <div class="flex items-center justify-between px-1">
          <button type="button" class={transport} aria-label={$t("amp_remote.shuffle")} title={$t("amp_remote.shuffle")} onclick={() => send({ type: "transport", action: "shuffle" })}>
            <Icon icon="material-symbols:shuffle-rounded" width="18" />
          </button>
          <button type="button" class={transport} aria-label={$t("amp_remote.previous")} title={$t("amp_remote.previous")} onclick={() => send({ type: "transport", action: "previous" })}>
            <Icon icon="material-symbols:skip-previous-rounded" width="22" />
          </button>
          <button
            type="button"
            class="lecture w-11 h-11 flex items-center justify-center rounded-full cursor-pointer hover:brightness-110 transition"
            aria-label={$t("amp_remote.play_pause")}
            title={$t("amp_remote.play_pause")}
            onclick={() => send({ type: "transport", action: "play_pause" })}
          >
            <Icon icon={net.status === "play" ? "material-symbols:pause-rounded" : "material-symbols:play-arrow-rounded"} width="24" />
          </button>
          <button type="button" class={transport} aria-label={$t("amp_remote.next")} title={$t("amp_remote.next")} onclick={() => send({ type: "transport", action: "next" })}>
            <Icon icon="material-symbols:skip-next-rounded" width="22" />
          </button>
          <button type="button" class={transport} aria-label={$t("amp_remote.repeat")} title={$t("amp_remote.repeat")} onclick={() => send({ type: "transport", action: "repeat" })}>
            <Icon icon="material-symbols:repeat-rounded" width="18" />
          </button>
        </div>
      </div>
    {/if}
  {:else}
    <div class="flex flex-col items-center gap-2.5 px-2 text-center">
      <p class="text-[11.5px] leading-snug text-(--rg-mu)">{$t(pave === "molette" ? "amp_remote.nav_hint" : "amp_remote.nav_hint_keys")}</p>
      {#if entreeNet}
        <button
          type="button"
          class="h-8 px-3.5 flex items-center gap-1.5 rounded-full border border-(--rg-bd) text-[11.5px] font-semibold cursor-pointer
                 text-(--rg-tx2) hover:text-(--rg-gtx) hover:border-(--rg-g)/50 transition-colors"
          onclick={() => send({ type: "input", zone: "main", id: entreeNet.id })}
        ><Icon icon="material-symbols:lan-outline-rounded" width="15" />{$t("amp_remote.go_net")}</button>
      {/if}
    </div>
  {/if}

  {#snippet choixPave(classes: string)}
    <SegmentedControl variant="discret" label={$t("amp_remote.pad")} value={pave} onchange={(v) => (pave = v as Pave)}
                      class="scale-90 origin-right {classes}"
                      options={[
                        { value: "molette", label: $t("amp_remote.pad_wheel"), icon: "material-symbols:trip-origin", iconOnly: true },
                        { value: "touches", label: $t("amp_remote.pad_keys"), icon: "material-symbols:more-horiz", iconOnly: true },
                      ]} />
  {/snippet}

  <!-- Directions : liste réseau sur NET/USB, menu à l'écran sinon. -->
  <div class="relative flex flex-col items-center gap-3">
    {#if pave === "molette"}
      {@render choixPave("absolute right-0 top-0")}
      <ClickWheel onkey={(k) => nav(k as AmpNavKey)} size={surNet ? 136 : 152}
                  labels={{ up: $t("amp_remote.up"), down: $t("amp_remote.down"), left: $t("amp_remote.left"), right: $t("amp_remote.right"), ok: $t("amp_remote.ok"), wheel: $t("amp_remote.wheel") }} />
    {:else}
      <!-- Une seule ligne : les touches, puis le choix du mode. -->
      <div class="w-full flex items-center justify-between gap-2">
        <div class="flex items-center gap-1.5">
          {#each TOUCHES as [key, icone] (key)}
            <button
              type="button"
              class="h-9 flex items-center justify-center rounded-[10px] cursor-pointer transition-colors
                     {icone ? 'w-9 bouton-relief text-(--rg-tx2) hover:text-(--rg-tx)' : 'w-12 ok text-[12.5px] font-bold'}"
              aria-label={$t(key === "enter" ? "amp_remote.ok" : `amp_remote.${key}`)}
              onclick={() => nav(key)}
            >
              {#if icone}<Icon icon={icone} width="22" />{:else}OK{/if}
            </button>
          {/each}
        </div>
        {@render choixPave("")}
      </div>
    {/if}
    {#if !surNet}
    <div class="w-full flex gap-1.5">
      <button type="button" class={petit} onclick={() => nav("back")}>
        <Icon icon="material-symbols:arrow-back-rounded" width="15" />{$t("amp_remote.back")}
      </button>
      <button type="button" class={petit} onclick={() => nav("home")}>
        <Icon icon="material-symbols:home-outline-rounded" width="15" />{$t("amp_remote.home")}
      </button>
      <button type="button" class={petit} onclick={() => nav("menu")}>
        <Icon icon="material-symbols:menu-rounded" width="15" />{$t("amp_remote.menu")}
      </button>
    </div>
    {/if}
  </div>
</div>

<style>
  /* Écran de l'ampli : fond à peine teinté, liseré clair en haut. */
  .viseur {
    background: linear-gradient(180deg, color-mix(in oklch, var(--rg-s2) 70%, transparent), color-mix(in oklch, var(--rg-s2) 30%, transparent));
    box-shadow: inset 0 1px 0 rgb(255 255 255 / 0.04);
  }
  /* OK de la ligne de touches : bombé, s'allume au survol (comme celui de la molette). */
  .ok {
    background: radial-gradient(circle at 50% 30%, color-mix(in oklch, var(--rg-carte) 85%, white), var(--rg-carte));
    color: var(--rg-tx);
    border: 1px solid var(--rg-bd2);
    box-shadow: 0 3px 10px rgb(0 0 0 / 0.3), inset 0 1px 0 rgb(255 255 255 / 0.1);
  }
  .ok:hover {
    color: var(--rg-gtx);
    border-color: color-mix(in oklch, var(--rg-g) 60%, transparent);
  }
  .lecture {
    background: radial-gradient(circle at 50% 35%, color-mix(in oklch, var(--rg-g) 80%, white), var(--rg-g));
    color: var(--rg-on-g);
    box-shadow: 0 0 16px color-mix(in oklch, var(--rg-g) 50%, transparent), inset 0 1px 0 rgb(255 255 255 / 0.3);
  }
</style>
