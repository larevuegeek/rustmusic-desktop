<script lang="ts">
  // Compléter en ligne, en deux étapes : choisir l'édition (Deezer ou MusicBrainz), puis
  // les champs à reprendre. Rien n'est écrit : « Appliquer » remplit les modifications de l'atelier.
  import Icon from "@iconify/svelte";
  import { untrack } from "svelte";
  import { t } from "#lib/i18n";
  import { messageErreur } from "#lib/helper/tools/errorTools";
  import { popinStore } from "#lib/stores/ui/popin.store";
  import { tagWorkshop, valueOf } from "#lib/stores/tags/tagWorkshop.store";
  import { metadataSource, SOURCE_OPTIONS } from "#lib/stores/tags/metadataSource.store";
  import {
    formatDuration,
    matchAlbum,
    searchAlbums,
    SOURCE_FIELDS,
    type AlbumHit,
    type MetadataSource,
    type SourceField,
  } from "#lib/services/tags/metadata.service";
  import { SourceSelection, cellKey, coverKey, type Pair } from "./source/sourceSelection.svelte.js";

  /* eslint-disable svelte/valid-prop-names-in-kit-pages */
  /** Appelé après application : valeurs et fichiers touchés. */
  const { onapplied = () => {} }: { onapplied?: (values: number, files: number) => void } = $props();

  let workshop = $derived($tagWorkshop);
  let targets = $derived(workshop.files.filter((f) => f.readable && workshop.selected.has(f.path)));
  const selection = new SourceSelection(() => targets);

  const SOURCES = SOURCE_OPTIONS.map((o) => ({ id: o.value, label: o.label, icon: o.icon }));
  let sourceName = $derived(SOURCES.find((s) => s.id === $metadataSource)?.label ?? "");

  let step: 1 | 2 = $state(1);
  let error: string | null = $state(null);

  // ─── Étape 1 : l'édition ───
  let query = $state(untrack(() => {
    const first = targets[0];
    return first ? [valueOf(first, "artist"), valueOf(first, "album")].filter(Boolean).join(" ").trim() : "";
  }));
  let hits: AlbumHit[] = $state([]);
  let picked: AlbumHit | null = $state(null);
  let searching = $state(false);
  let searched = $state(false);
  let matching = $state(false);
  /** Vignettes absentes chez la source (MusicBrainz n'en a pas pour toutes les éditions). */
  let noCover = $state(new Set<string>());

  let count = $derived(targets.length);
  let fitting = $derived(hits.filter((h) => h.track_count === count));
  let others = $derived(hits.filter((h) => h.track_count !== count));
  /** Meilleur choix : la première édition (ordre de pertinence) au bon nombre de pistes. */
  let best = $derived(fitting[0] ?? null);

  /** Recherche en cours : une réponse plus ancienne (autre source, autre requête) est ignorée. */
  let searchSeq = 0;

  async function search() {
    const seq = ++searchSeq;
    picked = null;
    hits = [];
    if (!query.trim()) {
      searching = false;
      return;
    }
    searching = true;
    error = null;
    noCover = new Set();
    try {
      const found = await searchAlbums($metadataSource, query, 12);
      if (seq !== searchSeq) return;
      hits = found;
      picked = found.find((h) => h.track_count === count) ?? found[0] ?? null;
    } catch (e) {
      if (seq !== searchSeq) return;
      error = messageErreur(e);
    } finally {
      if (seq === searchSeq) {
        searching = false;
        searched = true;
      }
    }
  }

  // La recherche part d'elle-même à l'ouverture, avec l'artiste et l'album du fichier.
  $effect(() => {
    untrack(() => {
      if (query.trim()) search();
    });
  });

  function changeSource(id: MetadataSource) {
    if (id === $metadataSource) return;
    metadataSource.set(id);
    search();
  }

  async function next() {
    if (!picked || matching) return;
    matching = true;
    error = null;
    try {
      selection.load(
        await matchAlbum(
          $metadataSource,
          picked.id,
          targets.map((file, index) => ({
            index,
            title: valueOf(file, "title"),
            track_number: Number(valueOf(file, "track_number")) || null,
            duration: null,
          })),
        ),
      );
      mode = "fill";
      opened = new Set();
      step = 2;
    } catch (e) {
      error = messageErreur(e);
    } finally {
      matching = false;
    }
  }

  // Ce que disent vos fichiers : la référence pour choisir l'édition.
  let yourYear = $derived.by(() => {
    const years = new Set(targets.map((f) => valueOf(f, "year")).filter(Boolean));
    return years.size === 1 ? [...years][0] : null;
  });
  let emptyFields = $derived(
    SOURCE_FIELDS.filter((field) => targets.length > 0 && targets.every((f) => !valueOf(f, field).trim())).map((f) => $t(`tags.${f}`).toLowerCase()),
  );

  // ─── Étape 2 : les champs ───
  type Mode = "fill" | "all" | "custom";
  let mode: Mode = $state("fill");
  let opened = $state(new Set<string>());
  let mapOpen = $state(false);

  function setMode(next: Mode) {
    mode = next;
    if (next === "fill") selection.selectAll("empty");
    if (next === "all") selection.selectAll("all");
  }

  type FieldRow = {
    field: SourceField;
    diffs: { pair: Pair; from: string; to: string; key: string }[];
    fills: number;
    replaces: number;
  };
  let rows: FieldRow[] = $derived(
    SOURCE_FIELDS.map((field) => {
      const diffs = selection.pairs.flatMap((pair) => {
        const cell = pair.cells.find((c) => c.field === field);
        return cell && cell.state === "differs" ? [{ pair, from: cell.from, to: cell.to, key: cellKey(pair.file.path, field) }] : [];
      });
      const fills = diffs.filter((d) => d.from === "").length;
      return { field, diffs, fills, replaces: diffs.length - fills };
    }).filter((r) => r.diffs.length > 0),
  );
  const fieldState = (field: SourceField) => {
    const cells = selection.pairs.map((p) => p.cells.find((c) => c.field === field)?.state);
    return cells.every((s) => s === "absent") ? "absent" : cells.some((s) => s === "differs") ? "differs" : "same";
  };
  let identical = $derived(SOURCE_FIELDS.filter((f) => selection.pairs.length > 0 && fieldState(f) === "same").map((f) => $t(`tags.${f}`)));
  let missing = $derived(SOURCE_FIELDS.filter((f) => selection.pairs.length > 0 && fieldState(f) === "absent").map((f) => $t(`tags.${f}`)));

  const distinct = (values: string[]) => [...new Set(values)];
  const checkState = (keys: string[]) => {
    const n = keys.filter((k) => selection.chosen.has(k)).length;
    return n === 0 ? "off" : n === keys.length ? "on" : "mid";
  };
  function toggleKeys(keys: string[]) {
    selection.toggleKeys(keys);
    mode = "custom";
  }
  function toggleOpen(id: string) {
    const nextSet = new Set(opened);
    if (nextSet.has(id)) nextSet.delete(id);
    else nextSet.add(id);
    opened = nextSet;
  }

  // Pochette : remplit celles qui manquent, remplace les autres.
  let coverKeys = $derived(selection.hasCover ? selection.pairs.map((p) => coverKey(p.file.path)) : []);
  let coverFills = $derived(selection.pairs.filter((p) => !p.file.cover).length);

  /** Moyenne des appariements retenus : ce que vaut l'édition pour vos fichiers. */
  let fitScore = $derived.by(() => {
    const kept = selection.matches.filter((m) => m.confidence !== "rejected");
    return kept.length ? Math.round((kept.reduce((s, m) => s + m.score, 0) / kept.length) * 100) : 0;
  });
  let matchedCount = $derived(selection.pairs.length);
  let allSure = $derived(selection.pairs.length === count && selection.pairs.every((p) => !p.doubtful));

  function apply() {
    // Comptés avant d'appliquer : ensuite, les valeurs reprises ne diffèrent plus.
    const values = selection.changeCount;
    const files = selection.fileCount;
    tagWorkshop.group(() => {
      for (const { pair, cell } of selection.picked) tagWorkshop.set(pair.file.path, cell.field, cell.to);
      const { remoteCover, coverPicked } = selection;
      if (remoteCover && coverPicked.length > 0) {
        tagWorkshop.setCoverOn(coverPicked.map((pair) => pair.file.path), { path: remoteCover.path, src: remoteCover.src });
      }
    });
    onapplied(values, files);
    popinStore.close();
  }

  const plural = (n: number, one: string, many: string) => `${n} ${n > 1 ? many : one}`;
  const tile = "w-16 h-16 shrink-0 rounded-[9px] overflow-hidden bg-(--rg-s2) shadow-[0_4px_14px_rgba(0,0,0,0.35)] flex items-center justify-center text-(--rg-mu2)";
</script>

{#snippet box(state: string)}
  <span class="ts-case {state}">
    <Icon icon={state === "mid" ? "material-symbols:remove-rounded" : "material-symbols:check-rounded"} width="15" />
  </span>
{/snippet}

{#snippet cover(src: string | null, id: string, size: string)}
  <span class="{tile} {size}">
    {#if src && !noCover.has(id)}
      <img src={src} alt="" loading="lazy" class="w-full h-full object-cover"
           onerror={() => (noCover = new Set([...noCover, id]))} />
    {:else}
      <Icon icon="material-symbols:album-outline-rounded" width="24" />
    {/if}
  </span>
{/snippet}

{#snippet edition(hit: AlbumHit)}
  {@const on = picked?.id === hit.id}
  {@const diff = hit.track_count - count}
  <button type="button" class="ts-res {on ? 'on' : ''}" onclick={() => (picked = hit)} ondblclick={next}>
    <span class="ts-radio"></span>
    {@render cover(hit.cover, hit.id, "")}
    <span class="min-w-0 flex flex-col gap-1.25 text-left">
      <b class="flex items-center gap-2 text-[15.5px] font-bold min-w-0">
        <span class="truncate">{hit.title}</span>
        {#if best?.id === hit.id}<span class="shrink-0 px-1.75 py-0.5 rounded-[5px] text-[10.5px] font-extrabold tracking-wide bg-(--rg-g) text-(--rg-on-g)">{$t("source.best")}</span>{/if}
      </b>
      <span class="flex flex-wrap gap-x-3 gap-y-1 text-[12.5px] text-(--rg-mu)">
        <span>{hit.artist}</span>
        {#if hit.detail}<span>{hit.detail}</span>{/if}
      </span>
    </span>
    <span class="ts-pilule {diff === 0 ? 'ok' : diff > 0 ? 'am' : 'non'}">
      {#if diff === 0}
        <Icon icon="material-symbols:check-rounded" width="15" />{hit.track_count} / {count} {$t("source.tracks")}
      {:else if diff > 0}
        {$t("source.extra").replace("{n}", String(hit.track_count)).replace("{d}", String(diff))}
      {:else}
        {$t("source.short_by").replace("{n}", String(hit.track_count)).replace("{d}", String(-diff))}
      {/if}
    </span>
  </button>
{/snippet}

<div class="ts biblio-maquette h-[min(740px,calc(88vh-2px))] flex flex-col min-h-0 bg-(--rg-carte)">
  <!-- ─────────── En-tête : titre, étapes, fermeture ─────────── -->
  <div class="shrink-0 flex flex-wrap items-center gap-4 pl-5.5 pr-4.5 py-4.5 border-b border-(--rg-bd)">
    <span class="w-10 h-10 shrink-0 rounded-[11px] border flex items-center justify-center bg-(--rg-gbg) border-(--rg-gbd) text-(--rg-g)">
      <Icon icon="material-symbols:cloud-download-outline-rounded" width="21" />
    </span>
    <div class="min-w-0">
      <h2 class="text-[19px] font-extrabold tracking-[-0.01em] whitespace-nowrap">{$t("workshop.complete_online")}</h2>
      <p class="text-[13px] text-(--rg-mu) truncate">{workshop.source} · {plural(count, $t("workshop.file_one"), $t("workshop.file_many"))}</p>
    </div>
    <div class="ml-auto flex items-center gap-1.5">
      <button type="button" class="ts-etape {step === 1 ? 'on' : 'fait'}" onclick={() => (step = 1)}>
        <b>{#if step === 2}<Icon icon="material-symbols:check-rounded" width="14" />{:else}1{/if}</b>{$t("source.step_edition")}
      </button>
      <span class="w-4.5 h-px bg-(--rg-bd2)"></span>
      <button type="button" class="ts-etape {step === 2 ? 'on' : ''}" disabled={step === 1 && !picked} onclick={() => step === 1 && next()}>
        <b>2</b>{$t("source.step_fields")}
      </button>
    </div>
    <button type="button" onclick={() => popinStore.requestClose()} title={$t("common.close")} aria-label={$t("common.close")}
            class="w-9 h-9 rounded-[9px] flex items-center justify-center cursor-pointer transition-colors text-(--rg-mu) hover:bg-(--rg-s2) hover:text-(--rg-tx)">
      <Icon icon="material-symbols:close-rounded" width="20" />
    </button>
  </div>

  <!-- ─────────── Corps ─────────── -->
  <div class="flex-1 min-h-0 overflow-y-auto scrollbar-app px-5.5 pt-5 pb-6">
    {#if error}
      <p class="mb-4 flex items-start gap-2 px-3.5 py-2.5 rounded-xl text-[13px] bg-red-500/10 text-red-600 dark:text-red-400">
        <Icon icon="material-symbols:error-outline-rounded" width="17" class="shrink-0" />{error}
      </p>
    {/if}

    {#if step === 1}
      <div class="flex flex-wrap gap-2.5">
        <div class="flex p-1 rounded-xl border border-(--rg-bd) bg-(--rg-champ)" role="radiogroup" aria-label={$t("source.provider")}>
          {#each SOURCES as s (s.id)}
            {@const on = $metadataSource === s.id}
            <button type="button" role="radio" aria-checked={on} onclick={() => changeSource(s.id)}
                    class="h-9 px-3.5 rounded-[9px] flex items-center gap-2 text-[13.5px] font-bold cursor-pointer transition-colors
                           {on ? 'bg-(--rg-s2) text-(--rg-tx) shadow-sm' : 'text-(--rg-mu) hover:text-(--rg-tx)'}">
              <Icon icon={s.icon} width="16" class={on ? "text-(--rg-g)" : ""} />{s.label}
            </button>
          {/each}
        </div>
        <label class="flex-[1_1_280px] h-11.5 flex items-center gap-2.5 pl-3.5 pr-1.5 rounded-xl border border-(--rg-bd) bg-(--rg-champ) focus-within:border-(--rg-bd2)">
          <Icon icon="material-symbols:search-rounded" width="20" class="text-(--rg-mu)" />
          <input bind:value={query} onkeydown={(e) => e.key === "Enter" && search()} data-focus-ring="none"
                 placeholder={$t("source.search_placeholder")}
                 class="flex-1 min-w-0 bg-transparent outline-none text-[15px] text-(--rg-tx) placeholder:text-(--rg-ph)" />
          <button type="button" onclick={search} disabled={searching || !query.trim()}
                  class="h-8.5 px-3.5 rounded-[9px] text-[13px] font-bold cursor-pointer transition-colors bg-(--rg-s2) text-(--rg-tx2) hover:text-(--rg-tx) disabled:opacity-40">
            {$t("source.search")}
          </button>
        </label>
      </div>

      <p class="mt-3.5 mb-5.5 flex flex-wrap items-center gap-x-2.5 gap-y-1 text-[13px] text-(--rg-mu)">
        <span class="text-[11px] font-bold tracking-widest uppercase text-(--rg-mu2)">{$t("source.your_files")}</span>
        <b class="font-semibold text-(--rg-tx2)">{plural(count, $t("source.track_one"), $t("source.track_many"))}</b>
        {#if yourYear}<span>·</span><span>{yourYear}</span>{/if}
        {#if emptyFields.length > 0}<span>·</span><span>{$t("source.empty_fields").replace("{f}", emptyFields.join(", "))}</span>{/if}
      </p>

      {#if searching}
        <div class="flex flex-col gap-2">
          {#each [0, 1, 2] as i (i)}<div class="ts-squelette"></div>{/each}
        </div>
      {:else if hits.length === 0}
        {#if searched}<p class="py-12 text-center text-[13.5px] text-(--rg-mu)">{$t("source.no_result")}</p>{/if}
      {:else}
        {#if fitting.length > 0}
          <p class="ts-groupe">{$t("source.fit_your").replace("{n}", String(count))}<small>{plural(fitting.length, $t("source.edition_one"), $t("source.edition_many"))}</small></p>
          <div class="flex flex-col gap-2">{#each fitting as hit (hit.id)}{@render edition(hit)}{/each}</div>
        {/if}
        {#if others.length > 0}
          <p class="ts-groupe {fitting.length > 0 ? 'mt-6.5' : ''}">{$t("source.other_editions")}<small>{$t("source.other_hint")}</small></p>
          <div class="flex flex-col gap-2">{#each others as hit (hit.id)}{@render edition(hit)}{/each}</div>
        {/if}
      {/if}

      <p class="mt-5 flex gap-2 text-[12.5px] leading-normal text-(--rg-mu2)">
        <Icon icon="material-symbols:info-outline-rounded" width="17" class="shrink-0" />{$t(`source.note_${$metadataSource}`)}
      </p>
    {:else if selection.proposal}
      {@const album = selection.proposal.album}
      <!-- Édition retenue -->
      <div class="flex flex-wrap items-center gap-4 px-4 py-3.5 rounded-[14px] border border-(--rg-bd) bg-(--rg-champ)">
        {@render cover(picked?.cover ?? album.cover ?? null, picked?.id ?? album.id, "w-13! h-13!")}
        <div class="flex-[1_1_220px] min-w-0">
          <b class="block text-[15px] font-bold truncate">{album.title}</b>
          <small class="block mt-0.75 text-[12.5px] text-(--rg-mu) truncate">
            {[sourceName, picked?.detail, plural(album.tracks.length, $t("source.track_one"), $t("source.track_many")), $t("source.fit").replace("{n}", String(fitScore))].filter(Boolean).join(" · ")}
          </small>
        </div>
        <button type="button" onclick={() => (step = 1)}
                class="shrink-0 h-8 px-3 rounded-[9px] border border-(--rg-bd2) text-[13px] font-bold cursor-pointer transition-colors text-(--rg-tx2) hover:bg-(--rg-s2) hover:text-(--rg-tx)">
          {$t("source.change_edition")}
        </button>
      </div>

      <!-- Mode de mise à jour -->
      <div class="mt-5.5 mb-3 flex flex-wrap items-center gap-3">
        <span class="text-[11.5px] font-bold tracking-widest uppercase text-(--rg-mu)">{$t("source.update")}</span>
        <div class="flex p-0.75 rounded-[10px] border border-(--rg-bd) bg-(--rg-champ)">
          {#each [["fill", "source.mode_fill"], ["all", "source.mode_all"], ["custom", "source.mode_custom"]] as [id, key] (id)}
            <button type="button" aria-pressed={mode === id} disabled={id === "custom" && mode !== "custom"}
                    onclick={() => setMode(id as Mode)}
                    class="h-8 px-3.5 rounded-[7px] text-[13px] font-bold whitespace-nowrap cursor-pointer transition-colors disabled:opacity-50 disabled:cursor-default
                           {mode === id ? 'bg-(--rg-s2) text-(--rg-tx) shadow-sm' : 'text-(--rg-mu) hover:text-(--rg-tx)'}">
              {$t(key)}
            </button>
          {/each}
        </div>
        <span class="text-[12.5px] text-(--rg-mu2)">{$t(`source.mode_${mode}_hint`)}</span>
      </div>

      <!-- Champs -->
      <div class="rounded-[14px] border border-(--rg-bd) overflow-hidden">
        <div class="ts-champ ts-tete h-9.5 bg-(--rg-champ) text-[11px] font-bold tracking-widest uppercase text-(--rg-mu2) border-b border-(--rg-bd)">
          <span></span><span>{$t("source.col_field")}</span><span>{$t("source.your_files")}</span><span></span>
          <span class="text-(--rg-gtx)">{sourceName}</span><span>{$t("source.col_effect")}</span><span></span>
        </div>

        {#if selection.hasCover}
          {@const state = checkState(coverKeys)}
          {@const img = selection.remoteCover!}
          {@const current = selection.pairs.find((p) => p.file.cover)?.file.cover ?? null}
          <div class="ts-ligne {state !== 'off' ? 'on' : ''}">
            <div class="ts-champ min-h-14.5 cursor-pointer" role="presentation" onclick={() => toggleKeys(coverKeys)}>
              {@render box(state)}
              <span class="text-[14px] font-bold">{$t("tags.cover")}</span>
              <span class="ts-val">
                {#if current}<img src={current} alt="" class="w-8.5 h-8.5 rounded-md object-cover shrink-0" /><span>{$t("workshop.cover_embedded")}</span>{:else}<span class="italic text-(--rg-mu2)">{$t("source.empty")}</span>{/if}
              </span>
              <Icon icon="material-symbols:arrow-forward-rounded" width="17" class="text-(--rg-mu2)" />
              <span class="ts-val nouveau"><img src={img.src} alt="" class="w-8.5 h-8.5 rounded-md object-cover shrink-0" /><span>{img.width} × {img.height}</span></span>
              <span class="ts-effet {coverFills === coverKeys.length ? 'remplit' : 'remplace'}">
                <Icon icon={coverFills === coverKeys.length ? "material-symbols:add-rounded" : "material-symbols:warning-outline-rounded"} width="15" />
                {coverFills === coverKeys.length ? $t("source.fills").replace("{n}", String(coverFills)) : $t("source.replaces").replace("{n}", String(coverKeys.length - coverFills))}
              </span>
              <span></span>
            </div>
          </div>
        {/if}

        {#each rows as row (row.field)}
          {@const keysOf = row.diffs.map((d) => d.key)}
          {@const state = checkState(keysOf)}
          {@const froms = distinct(row.diffs.map((d) => d.from))}
          {@const tos = distinct(row.diffs.map((d) => d.to))}
          {@const open = opened.has(row.field)}
          <div class="ts-ligne {state !== 'off' ? 'on' : ''}">
            <div class="ts-champ min-h-14.5 cursor-pointer" role="presentation" onclick={() => toggleKeys(keysOf)}>
              {@render box(state)}
              <span class="text-[14px] font-bold">{$t(`tags.${row.field}`)}</span>
              <span class="ts-val">
                {#if froms.length > 1}<span class="text-(--rg-mu)">{$t("source.n_values").replace("{n}", String(froms.length))}</span>
                {:else if froms[0]}<span>{froms[0]}</span>
                {:else}<span class="italic text-(--rg-mu2)">{$t("source.empty")}</span>{/if}
              </span>
              <Icon icon="material-symbols:arrow-forward-rounded" width="17" class="text-(--rg-mu2)" />
              <span class="ts-val nouveau">
                <span>{tos.length > 1 ? `${tos[0]} …` : tos[0]}</span>
              </span>
              <span class="ts-effet {row.replaces === 0 ? 'remplit' : 'remplace'}">
                <Icon icon={row.replaces === 0 ? "material-symbols:add-rounded" : "material-symbols:warning-outline-rounded"} width="15" />
                {#if row.replaces === 0}{$t("source.fills").replace("{n}", String(row.fills))}
                {:else if row.fills === 0}{$t("source.replaces").replace("{n}", String(row.replaces))}
                {:else}{$t("source.fills_replaces").replace("{f}", String(row.fills)).replace("{r}", String(row.replaces))}{/if}
              </span>
              {#if selection.pairs.length > 1}
                <button type="button" title={$t("source.per_track")} aria-label={$t("source.per_track")} aria-expanded={open}
                        onclick={(e) => { e.stopPropagation(); toggleOpen(row.field); }}
                        class="w-7.5 h-7.5 rounded-lg flex items-center justify-center cursor-pointer transition text-(--rg-mu) hover:bg-(--rg-s2) hover:text-(--rg-tx) {open ? 'rotate-180' : ''}">
                  <Icon icon="material-symbols:expand-more-rounded" width="20" />
                </button>
              {:else}<span></span>{/if}
            </div>
            {#if open}
              <div class="pl-14.5 pr-2.5 pt-1 pb-3 bg-(--rg-creux)">
                {#each selection.pairs as pair (pair.file.path)}
                  {@const d = row.diffs.find((x) => x.pair === pair)}
                  {@const cell = pair.cells.find((c) => c.field === row.field)}
                  {@const on = !!d && selection.chosen.has(d.key)}
                  <div class="ts-detail {d ? 'cursor-pointer' : 'opacity-45'} {on ? 'on' : ''}" role="presentation"
                       onclick={() => d && toggleKeys([d.key])}>
                    {#if d}{@render box(on ? "on" : "off")}{:else}<span></span>{/if}
                    <span class="text-right tabular-nums text-(--rg-mu2)">{pair.track.disc > 1 ? `${pair.track.disc}-` : ""}{pair.track.position}</span>
                    <span class="truncate text-(--rg-mu)">{cell?.from || "—"}</span>
                    <Icon icon="material-symbols:arrow-forward-rounded" width="15" class="text-(--rg-mu2)" />
                    <span class="truncate {on ? 'text-(--rg-gtx)' : 'text-(--rg-tx)'}">
                      {cell?.state === "absent" ? $t("source.not_provided") : d ? d.to : $t("source.identical_short")}
                    </span>
                  </div>
                {/each}
              </div>
            {/if}
          </div>
        {/each}

        {#if identical.length > 0 || missing.length > 0 || (rows.length === 0 && !selection.hasCover)}
          <div class="flex flex-col gap-1.5 px-4 py-3.5 border-t border-(--rg-bd) bg-(--rg-champ) text-[13px] text-(--rg-mu)">
            {#if rows.length === 0 && !selection.hasCover}
              <span class="flex items-center gap-2.5"><Icon icon="material-symbols:done-all-rounded" width="18" class="text-(--rg-g)" />{$t("source.nothing_new")}</span>
            {/if}
            {#if identical.length > 0}
              <span class="flex items-center gap-2.5"><Icon icon="material-symbols:done-all-rounded" width="18" class="shrink-0 text-(--rg-mu2)" />
                <span>{$t("source.already_same")} <b class="font-semibold text-(--rg-tx2)">{identical.join(", ")}</b></span></span>
            {/if}
            {#if missing.length > 0}
              <span class="flex items-center gap-2.5"><Icon icon="material-symbols:block" width="18" class="shrink-0 text-(--rg-mu2)" />
                <span>{$t("source.not_provided_by").replace("{s}", sourceName)} <b class="font-semibold text-(--rg-tx2)">{missing.join(", ")}</b></span></span>
            {/if}
          </div>
        {/if}
      </div>

      <!-- Association des pistes -->
      <div class="mt-3.5 rounded-[14px] border border-(--rg-bd)">
        <button type="button" onclick={() => (mapOpen = !mapOpen)} aria-expanded={mapOpen}
                class="w-full flex items-center gap-2.5 px-4 py-3.5 text-[13.5px] text-left cursor-pointer">
          <span class="ts-pilule {allSure ? 'ok' : 'am'}">
            <Icon icon={allSure ? "material-symbols:check-rounded" : "material-symbols:warning-outline-rounded"} width="15" />{matchedCount} / {count}
          </span>
          <b class="font-bold">{$t("source.matching")}</b>
          <span class="text-(--rg-mu)">{$t("source.matching_how")}</span>
          <span class="flex-1"></span>
          <Icon icon="material-symbols:expand-more-rounded" width="20" class="text-(--rg-mu) transition-transform {mapOpen ? 'rotate-180' : ''}" />
        </button>
        {#if mapOpen}
          <div class="grid grid-cols-2 max-[820px]:grid-cols-1 gap-x-7 px-4 pb-3.5">
            {#each selection.matches as m (m.local_index)}
              {@const file = selection.localFile(m.local_index)}
              {@const track = selection.remoteTrack(m.remote_key)}
              {@const excluded = selection.excluded.has(m.local_index)}
              {@const rejected = m.confidence === "rejected" || !track}
              <div class="ts-asso {rejected ? '' : 'cursor-pointer'} {excluded ? 'opacity-45 line-through' : ''}" role="presentation"
                   title={rejected ? $t("source.no_match") : excluded ? $t("source.include") : $t("source.exclude")}
                   onclick={() => !rejected && selection.toggleExcluded(m.local_index)}>
                <span class="truncate">{file ? valueOf(file, "title") || "—" : "—"}</span>
                <Icon icon={rejected ? "material-symbols:close-rounded" : m.confidence === "doubtful" ? "material-symbols:help-outline-rounded" : "material-symbols:check-rounded"} width="15"
                      class={rejected ? "text-red-500" : m.confidence === "doubtful" ? "text-(--rg-am)" : "text-(--rg-g)"} />
                <span class="truncate text-(--rg-mu)">{track ? `${track.disc > 1 ? `${track.disc}-` : ""}${track.position}. ${track.title}` : "—"}</span>
                <span class="text-right tabular-nums text-(--rg-mu2)">{track ? formatDuration(track.duration) : ""}</span>
              </div>
            {/each}
          </div>
        {/if}
      </div>
    {/if}
  </div>

  <!-- ─────────── Pied ─────────── -->
  <div class="shrink-0 flex flex-wrap items-center gap-2.5 pl-5.5 pr-4.5 py-3.5 border-t border-(--rg-bd) bg-(--rg-champ)">
    <div class="flex-[1_1_260px] min-w-0 flex items-center gap-2.5 text-[13.5px] text-(--rg-mu)">
      {#if step === 1}
        {#if picked}
          <span class="ts-point"></span>
          <span class="truncate"><b class="text-(--rg-tx)">{picked.detail ?? picked.title}</b> · {plural(picked.track_count, $t("source.track_one"), $t("source.track_many"))}</span>
        {:else}
          {$t("source.pick_edition")}
        {/if}
      {:else if selection.changeCount > 0}
        <span class="ts-point"></span>
        <span class="truncate"><b class="text-(--rg-tx)">{plural(selection.changeCount, $t("source.value_one"), $t("source.value_many"))}</b>
          {$t("source.on_files").replace("{n}", plural(selection.fileCount, $t("workshop.file_one"), $t("workshop.file_many")))}</span>
      {:else}
        {$t("source.nothing_selected")}
      {/if}
    </div>
    {#if step === 1}
      <button type="button" class="ts-bouton" onclick={() => popinStore.requestClose()}>{$t("profil.cancel")}</button>
      <button type="button" class="ts-bouton principal" disabled={!picked || searching || matching} onclick={next}>
        {$t("source.continue")}
        <Icon icon={matching ? "material-symbols:progress-activity" : "material-symbols:arrow-forward-rounded"} width="20" class={matching ? "animate-spin" : ""} />
      </button>
    {:else}
      <button type="button" class="ts-bouton" onclick={() => (step = 1)}>
        <Icon icon="material-symbols:arrow-back-rounded" width="20" />{$t("source.back")}
      </button>
      <button type="button" class="ts-bouton principal" disabled={selection.changeCount === 0} onclick={apply}>
        <Icon icon="material-symbols:check-rounded" width="20" />{$t("source.apply")}
      </button>
    {/if}
  </div>
</div>

<style>
  .ts-etape {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 32px;
    padding: 0 12px 0 6px;
    border-radius: 16px;
    font-size: 13px;
    font-weight: 700;
    white-space: nowrap;
    color: var(--rg-mu2);
    cursor: pointer;
  }
  .ts-etape:disabled {
    cursor: default;
  }
  .ts-etape b {
    width: 22px;
    height: 22px;
    border-radius: 50%;
    border: 1.5px solid var(--rg-bd2);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 11.5px;
  }
  .ts-etape.on {
    background: var(--rg-s2);
    color: var(--rg-tx);
  }
  .ts-etape.on b {
    background: var(--rg-g);
    border-color: var(--rg-g);
    color: var(--rg-on-g);
  }
  .ts-etape.fait {
    color: var(--rg-tx2);
  }
  .ts-etape.fait b {
    border-color: var(--rg-gbd);
    color: var(--rg-g);
  }
  .ts-etape.fait:hover {
    color: var(--rg-tx);
    background: var(--rg-champ);
  }

  .ts-groupe {
    display: flex;
    align-items: baseline;
    gap: 10px;
    margin-bottom: 10px;
    font-size: 11.5px;
    font-weight: 700;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--rg-mu);
  }
  .ts-groupe small {
    font-size: 12.5px;
    font-weight: 500;
    letter-spacing: 0;
    text-transform: none;
    color: var(--rg-mu2);
  }

  .ts-res {
    display: grid;
    grid-template-columns: 20px 64px minmax(0, 1fr) auto;
    gap: 16px;
    align-items: center;
    width: 100%;
    padding: 12px 16px 12px 14px;
    border-radius: 14px;
    border: 1px solid var(--rg-bd);
    background: var(--rg-champ);
    cursor: pointer;
    transition: border-color 0.12s, background 0.12s;
  }
  .ts-res:hover {
    border-color: var(--rg-bd2);
  }
  .ts-res.on {
    border-color: var(--rg-g);
    background: color-mix(in oklch, var(--rg-g) 6%, var(--rg-champ));
    box-shadow: 0 0 0 3px color-mix(in oklch, var(--rg-g) 12%, transparent);
  }
  .ts-radio {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    border: 2px solid var(--rg-bd2);
  }
  .ts-res.on .ts-radio {
    border: 6px solid var(--rg-g);
  }

  .ts-pilule {
    height: 26px;
    padding: 0 10px;
    border-radius: 13px;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 12px;
    font-weight: 700;
    white-space: nowrap;
    border: 1px solid;
  }
  .ts-pilule.ok {
    color: var(--rg-gtx);
    background: var(--rg-gbg);
    border-color: var(--rg-gbd);
  }
  .ts-pilule.am {
    color: var(--rg-am);
    background: var(--rg-ambg);
    border-color: var(--rg-ambd);
  }
  .ts-pilule.non {
    color: var(--rg-mu);
    background: var(--rg-carte);
    border-color: var(--rg-bd);
  }

  .ts-squelette {
    height: 90px;
    border-radius: 14px;
    background: linear-gradient(90deg, var(--rg-champ), var(--rg-s2), var(--rg-champ));
    background-size: 200% 100%;
    animation: ts-lueur 1.1s linear infinite;
  }
  @keyframes ts-lueur {
    to {
      background-position: -200% 0;
    }
  }

  .ts-champ {
    display: grid;
    grid-template-columns: 44px 150px minmax(0, 1fr) 20px minmax(0, 1fr) 150px 36px;
    align-items: center;
    column-gap: 10px;
    padding: 0 10px 0 4px;
  }
  .ts-ligne + .ts-ligne {
    border-top: 1px solid var(--rg-line);
  }
  .ts-ligne > .ts-champ:hover {
    background: var(--rg-hover);
  }
  .ts-ligne.on > .ts-champ {
    background: color-mix(in oklch, var(--rg-g) 5%, transparent);
  }
  .ts-val {
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 13.5px;
    color: var(--rg-tx2);
  }
  .ts-val > span {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .ts-val.nouveau {
    color: var(--rg-tx);
    font-weight: 600;
  }
  .ts-ligne.on .ts-val.nouveau {
    color: var(--rg-gtx);
  }
  .ts-effet {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 12px;
    font-weight: 700;
    white-space: nowrap;
  }
  .ts-effet.remplit {
    color: var(--rg-gtx);
  }
  .ts-effet.remplace {
    color: var(--rg-am);
  }

  .ts-case {
    width: 20px;
    height: 20px;
    margin: 0 auto;
    border-radius: 6px;
    border: 2px solid var(--rg-bd2);
    display: flex;
    align-items: center;
    justify-content: center;
    color: transparent;
  }
  .ts-case.on,
  .ts-case.mid {
    background: var(--rg-g);
    border-color: var(--rg-g);
    color: var(--rg-on-g);
  }

  .ts-detail {
    display: grid;
    grid-template-columns: 20px 34px minmax(0, 1fr) 20px minmax(0, 1fr);
    gap: 10px;
    align-items: center;
    min-height: 36px;
    padding: 0 8px;
    border-radius: 8px;
    font-size: 13px;
  }
  .ts-detail.cursor-pointer:hover {
    background: var(--rg-hover);
  }

  .ts-asso {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 16px minmax(0, 1fr) 44px;
    gap: 8px;
    align-items: center;
    min-height: 32px;
    font-size: 12.5px;
    border-bottom: 1px solid var(--rg-line);
  }

  .ts-point {
    width: 8px;
    height: 8px;
    flex: none;
    border-radius: 50%;
    background: var(--rg-g);
    box-shadow: 0 0 0 4px color-mix(in oklch, var(--rg-g) 15%, transparent);
  }
  .ts-bouton {
    height: 42px;
    padding: 0 18px;
    border-radius: 11px;
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 14px;
    font-weight: 700;
    white-space: nowrap;
    color: var(--rg-tx2);
    cursor: pointer;
  }
  .ts-bouton:hover {
    background: var(--rg-s2);
    color: var(--rg-tx);
  }
  .ts-bouton.principal {
    background: var(--rg-g);
    color: var(--rg-on-g);
  }
  .ts-bouton.principal:hover:not(:disabled) {
    filter: brightness(1.1);
  }
  .ts-bouton.principal:disabled {
    background: var(--rg-s2);
    color: var(--rg-mu2);
    cursor: default;
  }

</style>
