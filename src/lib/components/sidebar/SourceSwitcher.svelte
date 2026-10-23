<script lang="ts">
  // Choix de la source : « Cet ordinateur », un serveur, ou toutes. Invisible sans serveur.
  import Icon from "@iconify/svelte";
  import { scale } from "svelte/transition";
  import { goto } from "$app/navigation";
  import { t, currentLocale } from "#lib/i18n";
  import { portal } from "#lib/helper/portal";
  import { libraryStore } from "#lib/stores/library/library.store";
  import { pistesAvecImport } from "#lib/stores/library/importProgress.store";
  import type { Library } from "#lib/types/db/library/Library";
  import { serverStore } from "#lib/stores/mediaServer/mediaServer.store";
  import { popinStore } from "#lib/stores/ui/popin.store";
  import { activeServers, currentSource, keepLibraryInSource, LOCAL, selectSource, type SourceKey } from "#lib/stores/source/source.store";
  import StatusDot from "#lib/components/ui/status/StatusDot.svelte";
  import AddServerPopin from "#lib/components/mediaServer/AddServerPopin.svelte";

  let { replie = false }: { replie?: boolean } = $props();

  type Entry = { key: SourceKey; name: string; icon: string; tone: string; detail: string; online?: boolean };

  const titres = (n: number) =>
    n === 1 ? $t("home.track_one") : $t("home.tracks_n").replace("{n}", n.toLocaleString($currentLocale));
  const bibliotheques = (n: number) => (n === 1 ? $t("sources.library_one") : $t("sources.libraries_n").replace("{n}", String(n)));

  const TEINTES = {
    local: "bg-(--sb-gbg) text-(--sb-g)",
    server: "bg-sky-500/12 text-sky-600 dark:text-sky-300",
    all: "bg-violet-500/12 text-violet-600 dark:text-violet-300",
  };

  const entries = $derived.by((): Entry[] => {
    // Les bibliothèques chargées, pas celles gardées à l'ajout du serveur (figées avant sa synchro).
    const locales = $libraryStore.libraries.filter((l) => !l.server_id);
    const total = (libs: Library[]) => libs.reduce((s, l) => s + $pistesAvecImport(l), 0);
    const list: Entry[] = [{
      key: LOCAL, name: $t("sources.local"), icon: "material-symbols:computer-outline-rounded", tone: TEINTES.local,
      detail: `${bibliotheques(locales.length)} · ${titres(total(locales))}`,
    }];
    for (const s of $activeServers) {
      const online = $serverStore.online[s.id];
      const libs = $libraryStore.libraries.filter((l) => l.server_id === s.id);
      list.push({
        key: s.id, name: s.name, tone: TEINTES.server, online,
        icon: s.kind === "dlna" ? "material-symbols:dns-outline-rounded" : "material-symbols:cloud-outline-rounded",
        detail: `${online === false ? $t("sources.unreachable") : bibliotheques(libs.length)} · ${titres(total(libs))}`,
      });
    }
    list.push({
      key: "all", name: $t("sources.all"), icon: "material-symbols:stacks-outline-rounded", tone: TEINTES.all,
      detail: titres(total($libraryStore.libraries)),
    });
    return list;
  });
  const courante = $derived(entries.find((e) => e.key === $currentSource) ?? entries[0]);
  const visible = $derived($activeServers.length > 0);

  // La bibliothèque ouverte suit la source (au lancement, à l'ajout ou au retrait d'un serveur).
  $effect(() => {
    void $currentSource;
    void $libraryStore.libraries;
    if ($serverStore.loaded) keepLibraryInSource();
  });

  let ouvert = $state(false);
  let boutonEl: HTMLElement | null = $state(null);
  let listeEl: HTMLElement | null = $state(null);
  let position = $state("");

  function toggle() {
    if (!ouvert && boutonEl) {
      const r = boutonEl.getBoundingClientRect();
      position = replie
        ? `left: ${r.right + 8}px; top: ${r.top}px; width: 300px;`
        : `left: ${r.left}px; top: ${r.bottom + 6}px; width: ${Math.max(r.width, 300)}px;`;
    }
    ouvert = !ouvert;
  }

  async function choose(key: SourceKey) {
    ouvert = false;
    if (key === $currentSource) return;
    await selectSource(key);
    goto("/");
  }

  function addServer() {
    ouvert = false;
    popinStore.open($t("servers.add"), AddServerPopin, {}, { size: "lg", icon: "material-symbols:cloud-outline-rounded" });
  }

  $effect(() => {
    if (!ouvert) return;
    const dehors = (e: MouseEvent) => {
      const cible = e.target as Node;
      if (!boutonEl?.contains(cible) && !listeEl?.contains(cible)) ouvert = false;
    };
    const echap = (e: KeyboardEvent) => { if (e.key === "Escape") ouvert = false; };
    const timer = setTimeout(() => document.addEventListener("click", dehors), 50);
    document.addEventListener("keydown", echap);
    return () => {
      clearTimeout(timer);
      document.removeEventListener("click", dehors);
      document.removeEventListener("keydown", echap);
    };
  });
</script>

{#if visible}
  <button type="button" bind:this={boutonEl} onclick={toggle} aria-expanded={ouvert} title={$t("sources.change")}
          class="flex items-center gap-2.5 rounded-xl cursor-pointer transition-colors bg-(--sb-s1) border border-(--sb-bd) hover:border-(--sb-bd2)
                 {replie ? 'p-1.75' : 'w-full py-1.5 pr-2.5 pl-1.5'}">
    <span class="relative w-9 h-9 rounded-[9px] shrink-0 flex items-center justify-center {courante.tone}">
      <Icon icon={courante.icon} width="20" class="sb-icone" />
      {#if courante.online !== undefined}<StatusDot online={courante.online} class="absolute -top-0.5 -right-0.5" />{/if}
    </span>
    {#if !replie}
      <span class="flex-1 flex flex-col gap-px min-w-0 text-left">
        <small class="text-[10.5px] font-bold uppercase tracking-[.1em] text-(--sb-hd)">{$t("sources.label")}</small>
        <b class="text-sm font-bold truncate text-(--sb-tx)">{courante.name}</b>
      </span>
      <Icon icon="material-symbols:unfold-more-rounded" width="18" class="shrink-0 text-(--sb-tx2)" />
    {/if}
  </button>

  {#if ouvert}
    <div use:portal bind:this={listeEl} class="fixed z-[9999]" style={position} transition:scale={{ duration: 120, start: 0.97 }}>
      <div class="p-1.5 flex flex-col gap-0.5 rounded-2xl border shadow-xl shadow-black/10 dark:shadow-black/40
                  bg-white/98 dark:bg-neutral-900/98 backdrop-blur-xl border-neutral-200/60 dark:border-white/8" role="listbox" aria-label={$t("sources.label")}>
        {#each entries as e, i (e.key)}
          {#if e.key === "all" || i === 1}<div class="my-1 h-px bg-neutral-200/70 dark:bg-white/8" role="presentation"></div>{/if}
          <button type="button" role="option" aria-selected={e.key === $currentSource} onclick={() => choose(e.key)}
                  class="flex items-center gap-3 px-2 py-2 rounded-xl text-left cursor-pointer transition-colors
                         {e.key === $currentSource ? 'bg-neutral-100 dark:bg-white/6' : 'hover:bg-neutral-100/70 dark:hover:bg-white/4'}">
            <span class="w-9 h-9 rounded-[9px] shrink-0 flex items-center justify-center {e.tone}">
              <Icon icon={e.icon} width="20" />
            </span>
            <span class="flex-1 min-w-0">
              <b class="block text-sm font-bold truncate text-neutral-800 dark:text-neutral-100">{e.name}</b>
              <small class="flex items-center gap-1.5 text-xs truncate text-neutral-500 dark:text-neutral-400">
                {#if e.online !== undefined}<StatusDot online={e.online} />{/if}<span class="truncate">{e.detail}</span>
              </small>
            </span>
            {#if e.key === $currentSource}<Icon icon="material-symbols:check-rounded" width="18" class="shrink-0 text-green-600 dark:text-green-400" />{/if}
          </button>
        {/each}
        <button type="button" onclick={addServer}
                class="mt-1 flex items-center gap-2 px-3 py-2 rounded-xl text-xs font-semibold cursor-pointer text-neutral-500 dark:text-neutral-400 hover:bg-neutral-100/70 dark:hover:bg-white/4">
          <Icon icon="material-symbols:add-rounded" width="17" />{$t("servers.add")}
        </button>
      </div>
    </div>
  {/if}
{/if}
