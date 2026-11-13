<script lang="ts">
  // Portrait d'un artiste : l'actuel en grand, une image du disque, ou un choix parmi Deezer et Wikipédia.
  import Icon from "@iconify/svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import { fade } from "svelte/transition";
  import { t, currentLocale } from "#lib/i18n";
  import { popinStore } from "#lib/stores/ui/popin.store";
  import { toasts } from "#lib/stores/ui/toast.store";
  import CoverImg from "#lib/components/ui/image/CoverImg.svelte";
  import ArtistAvatar from "#lib/components/ui/image/ArtistAvatar.svelte";
  import SearchField from "#lib/components/ui/input/SearchField.svelte";
  import SegmentedControl from "#lib/components/ui/input/SegmentedControl.svelte";
  import PopinFooter from "#lib/components/ui/popin/PopinFooter.svelte";

  type Candidate = { source: "deezer" | "wikipedia"; title: string; detail: string | null; thumb: string; full: string };

  let { artistId, name, portrait = null }: { artistId: string; name: string; portrait?: string | null } = $props();

  // svelte-ignore state_referenced_locally
  let current = $state<string | null>(portrait);
  // svelte-ignore state_referenced_locally
  let query = $state(name);
  let results = $state<Candidate[]>([]);
  let searching = $state(false);
  let searchFailed = $state(false);
  let source = $state<"all" | "deezer" | "wikipedia">("all");
  let selected = $state<Candidate | null>(null);
  let busy = $state(false);

  const shown = $derived(source === "all" ? results : results.filter((r) => r.source === source));
  const count = (s: string) => results.filter((r) => r.source === s).length;

  // Seule la dernière recherche lancée s'affiche.
  let request = 0;
  async function search() {
    const q = query.trim();
    if (!q) return;
    const n = ++request;
    searching = true;
    searchFailed = false;
    selected = null;
    try {
      const r = await invoke<Candidate[]>("search_artist_portraits", { query: q, lang: $currentLocale });
      if (n === request) results = r ?? [];
    } catch (e) {
      if (n === request) {
        results = [];
        searchFailed = true;
      }
      console.error("[portrait] recherche :", e);
    } finally {
      if (n === request) searching = false;
    }
  }
  // Dès l'ouverture : le nom de l'artiste.
  $effect(() => {
    void search();
  });

  const errorText = (e: unknown) => (String(e) === "not_an_image" ? $t("notify.not_an_image") : String(e));

  // Le sélecteur de fichiers n'occupe pas la fenêtre : annuler ne laisse rien en attente.
  async function chooseFile() {
    const file = await open({
      multiple: false,
      title: $t("actions.dialog_choose_portrait"),
      filters: [{ name: $t("actions.dialog_images"), extensions: ["jpg", "jpeg", "png", "webp"] }],
    }).catch(() => null);
    if (typeof file !== "string") return;
    busy = true;
    try {
      current = await invoke<string>("set_artist_image", { artistId, imagePath: file });
      selected = null;
      toasts.push({ type: "success", title: name, message: $t("notify.portrait_updated") });
    } catch (e) {
      toasts.push({ type: "error", title: $t("notify.error"), message: errorText(e) });
    } finally {
      busy = false;
    }
  }

  async function remove() {
    busy = true;
    try {
      await invoke("clear_artist_image", { artistId });
      current = null;
      toasts.push({ type: "success", title: name, message: $t("notify.portrait_removed") });
    } catch (e) {
      toasts.push({ type: "error", title: $t("notify.error"), message: String(e) });
    } finally {
      busy = false;
    }
  }

  async function apply() {
    if (!selected) return;
    busy = true;
    try {
      await invoke("apply_artist_image_url", { artistId, url: selected.full });
      toasts.push({ type: "success", title: name, message: $t("notify.portrait_updated") });
      popinStore.close();
    } catch (e) {
      toasts.push({ type: "error", title: $t("notify.error"), message: errorText(e) });
    } finally {
      busy = false;
    }
  }

  const fans = (n: string | null) => $t("portrait.fans").replace("{n}", Number(n ?? 0).toLocaleString($currentLocale));
  const action = "w-full h-9.5 px-3.5 inline-flex items-center justify-center gap-2 rounded-[10px] border text-[13px] font-semibold cursor-pointer transition-colors disabled:opacity-50 disabled:cursor-default";
</script>

<div class="grid grid-cols-[260px_minmax(0,1fr)] max-md:grid-cols-1 min-h-115">
  <!-- ─── Le portrait : actuel, ou l'aperçu de celui qu'on s'apprête à prendre ─── -->
  <aside class="flex flex-col items-center gap-4 p-6 border-r max-md:border-r-0 max-md:border-b border-(--rg-bd) bg-(--rg-creux)">
    <p class="self-start text-[11px] font-bold uppercase tracking-[0.1em] {selected ? 'text-(--rg-gtx)' : 'text-(--rg-mu)'}">
      {$t(selected ? "portrait.preview" : "portrait.current")}
    </p>
    <div class="relative w-50 h-50 shrink-0 rounded-full overflow-hidden bg-(--rg-s2) shadow-[0_18px_40px_rgba(0,0,0,0.25)]
                {selected ? 'ring-3 ring-(--rg-g) ring-offset-4 ring-offset-(--rg-creux)' : ''}">
      {#key selected?.full ?? current}
        <div class="absolute inset-0" in:fade={{ duration: 180 }}>
          {#if selected}
            <img src={selected.full} alt="" class="w-full h-full object-cover" />
          {:else if current}
            <CoverImg path={current} alt={name} size="2x" class="w-full h-full object-cover" />
          {:else}
            <ArtistAvatar {name} deux class="w-full h-full text-[64px] tracking-[-0.02em]" />
          {/if}
        </div>
      {/key}
    </div>
    <div class="text-center min-w-0 w-full">
      <p class="text-[15px] font-bold truncate text-(--rg-tx)">{selected?.title ?? name}</p>
      <p class="text-[12.5px] text-(--rg-mu)">
        {#if selected}{selected.source === "deezer" ? "Deezer" : "Wikipédia"}{:else if !current}{$t("portrait.none")}{/if}
      </p>
    </div>

    <div class="mt-auto w-full flex flex-col gap-2">
      <button type="button" class="{action} border-(--rg-bd2) text-(--rg-tx) hover:bg-(--rg-s2)" disabled={busy} onclick={chooseFile}>
        <Icon icon="material-symbols:upload-file-outline-rounded" width="18" />{$t("portrait.choose_file")}
      </button>
      {#if current && !selected}
        <button type="button" class="{action} border-transparent text-(--rg-mu) hover:text-(--rg-tx) hover:bg-(--rg-s2)" disabled={busy} onclick={remove}>
          <Icon icon="material-symbols:hide-image-outline-rounded" width="18" />{$t("menu.remove_portrait")}
        </button>
      {/if}
    </div>
  </aside>

  <!-- ─── Recherche en ligne ─── -->
  <section class="flex flex-col gap-4 p-6 min-w-0">
    <form class="flex items-center gap-2.5" onsubmit={(e) => { e.preventDefault(); void search(); }}>
      <SearchField bind:value={query} class="flex-1 min-w-0" placeholder={$t("portrait.search_placeholder")} clearLabel={$t("common.clear")} />
      <button type="submit" class="h-9.5 px-4 shrink-0 rounded-[10px] text-[13px] font-bold cursor-pointer bg-(--rg-g) text-(--rg-on-g) hover:brightness-110 disabled:opacity-50"
              disabled={searching || !query.trim()}>{$t("portrait.search")}</button>
    </form>

    <SegmentedControl variant="discret" class="self-start" label={$t("portrait.sources")} value={source} onchange={(v) => (source = v as typeof source)}
                      options={[
                        { value: "all", label: `${$t("portrait.source_all")} · ${results.length}` },
                        { value: "deezer", label: `Deezer · ${count("deezer")}` },
                        { value: "wikipedia", label: `Wikipédia · ${count("wikipedia")}` },
                      ]} />

    <div class="flex-1 min-h-0 max-h-90 overflow-y-auto scrollbar-app -mx-1 px-1 pb-1">
      {#if searching}
        <div class="grid grid-cols-[repeat(auto-fill,minmax(112px,1fr))] gap-x-3 gap-y-4">
          {#each Array(8) as _, i (i)}
            <div class="flex flex-col items-center gap-2 animate-pulse">
              <span class="w-full aspect-square rounded-full bg-(--rg-s2)"></span>
              <span class="w-3/4 h-3 rounded bg-(--rg-s2)"></span>
            </div>
          {/each}
        </div>
      {:else if shown.length === 0}
        <div class="h-full min-h-60 flex flex-col items-center justify-center gap-2 text-center text-(--rg-mu)">
          <Icon icon={searchFailed ? "material-symbols:wifi-off-rounded" : "material-symbols:image-search-rounded"} width="34" />
          <p class="text-sm">{$t(searchFailed ? "portrait.offline" : "portrait.empty")}</p>
        </div>
      {:else}
        <div class="grid grid-cols-[repeat(auto-fill,minmax(112px,1fr))] gap-x-3 gap-y-4">
          {#each shown as c (c.full)}
            {@const on = selected?.full === c.full}
            <button type="button" onclick={() => (selected = on ? null : c)} title={c.detail ?? c.title}
                    class="group flex flex-col items-center gap-1.5 min-w-0 p-1.5 rounded-2xl cursor-pointer text-center transition-colors {on ? 'bg-(--rg-gbg)' : 'hover:bg-(--rg-s2)'}">
              <span class="relative w-full aspect-square rounded-full overflow-hidden bg-(--rg-s2) {on ? 'ring-3 ring-(--rg-g)' : ''}">
                <img src={c.thumb} alt="" loading="lazy" class="w-full h-full object-cover transition-transform duration-300 group-hover:scale-105" />
                {#if on}
                  <span class="absolute inset-0 flex items-center justify-center bg-black/35 text-white"><Icon icon="material-symbols:check-circle-rounded" width="30" /></span>
                {/if}
              </span>
              <span class="w-full text-[12.5px] font-semibold truncate text-(--rg-tx)">{c.title}</span>
              <span class="w-full -mt-1 flex items-center justify-center gap-1 text-[11px] text-(--rg-mu) truncate">
                <Icon icon={c.source === "deezer" ? "material-symbols:group-outline-rounded" : "simple-icons:wikipedia"} width="12" class="shrink-0" />
                <span class="truncate">{c.source === "deezer" ? fans(c.detail) : (c.detail ?? "Wikipédia")}</span>
              </span>
            </button>
          {/each}
        </div>
      {/if}
    </div>
  </section>
</div>

<PopinFooter cancelLabel={$t("batch.close")} oncancel={() => popinStore.close()} submitLabel={$t("portrait.use")}
             submitIcon="material-symbols:check-rounded" onsubmit={apply} submitDisabled={!selected || busy} {busy} />
