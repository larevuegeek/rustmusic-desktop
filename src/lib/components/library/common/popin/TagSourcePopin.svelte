<script lang="ts">
  // Récupération des métadonnées depuis Deezer, pour une sélection de fichiers.
  //
  // # Rien n'est écrit ici
  // Appliquer ne touche pas aux fichiers : les valeurs vont dans les
  // modifications en attente de l'atelier. On les relit ensuite dans le
  // tableur, ancienne valeur sous la nouvelle, et c'est « Écrire » qui tranche.
  //
  // # L'unité, c'est le morceau
  // Les versions précédentes agrégeaient par champ : « Artiste · 19 déjà
  // renseignés, conservés ». On y lisait des totaux, jamais son propre disque,
  // et on ne pouvait ni voir ni choisir ce qui arrivait à un morceau donné.
  //
  // Ici chaque fichier apparié est une ligne, dépliable, qui met ses valeurs en
  // regard de celles de Deezer. Trois niveaux de choix, du plus fin au plus
  // large : une valeur, un morceau, un champ sur toute la sélection.
  //
  // # Plus d'option « ne remplir que les champs vides »
  // Elle tenait lieu de garde-fou global et bloquait sept lignes sur huit sans
  // recours. Elle est devenue inutile : une valeur qui écraserait du texte
  // existant est visible, décochée, et se coche à la main. Le seul défaut
  // appliqué à l'ouverture est de cocher les cases vides — remplir un trou ne
  // détruit rien.
  import Icon from "@iconify/svelte";
  import { untrack } from "svelte";
  import { fade, slide } from "svelte/transition";
  import { popinStore } from "$lib/stores/ui/popin.store";
  import { t } from "$lib/i18n";
  import {
    tagWorkshop,
    valueOf,
    type WorkshopFile,
  } from "$lib/stores/tags/tagWorkshop.store";
  import {
    formatBytes,
    prepareImageFromUrl,
    type DownloadedImage,
  } from "$lib/services/tags/tagEditor.service";
  import {
    formatDuration,
    matchAlbum,
    proposedValues,
    searchAlbums,
    SOURCE_FIELDS,
    type AlbumHit,
    type AlbumTrack,
    type MatchProposal,
    type SourceField,
    type TrackMatch,
  } from "$lib/services/tags/metadata.service";

  /* eslint-disable svelte/valid-prop-names-in-kit-pages */
  const { onapplied = () => {} }: { onapplied?: () => void } = $props();

  let workshop = $derived($tagWorkshop);
  let targets = $derived(
    workshop.files.filter((f) => f.readable && workshop.selected.has(f.path)),
  );

  let query = $state("");
  let hits: AlbumHit[] = $state([]);
  let proposal: MatchProposal | null = $state(null);
  let searching = $state(false);
  let matching = $state(false);
  let searched = $state(false);
  let error: string | null = $state(null);

  /**
   * Les valeurs retenues, désignées par fichier **et** par champ.
   *
   * C'est le seul état de décision de l'écran : cocher un morceau ou un champ
   * n'est qu'une façon d'en ajouter ou d'en retirer plusieurs d'un coup.
   */
  let chosen = $state(new Set<string>());
  let excluded = $state(new Set<number>());
  /** Morceaux dont le détail est ouvert. */
  let opened = $state(new Set<number>());
  /** Les fichiers sans correspondance sont repliés : ils ne se lisent pas. */
  let showUnmatched = $state(false);

  /** La pochette de l'album, téléchargée et apprêtée. */
  let remoteCover: DownloadedImage | null = $state(null);
  let coverLoading = $state(false);
  let coverError: string | null = $state(null);


  /**
   * La pochette se désigne comme un champ, mais n'en est pas un.
   *
   * Elle ne vit pas dans les tags textuels et ne passe pas par le même chemin
   * d'écriture (`ImagePlan::SetCover`). Lui donner une clé du même genre permet
   * au reste de l'écran — cocher un morceau, cocher une colonne, compter — de
   * l'ignorer complètement.
   */
  const COVER = "cover";
  type CellField = SourceField | typeof COVER;

  const cellKey = (path: string, field: CellField) => `${path} ${field}`;
  const coverKey = (path: string) => cellKey(path, COVER);

  // La requête se devine : l'artiste et l'album qu'on est venu corriger sont
  // presque toujours la bonne recherche.
  $effect(() => {
    if (query !== "" || targets.length === 0) return;
    const first = targets[0];
    query = [valueOf(first, "artist"), valueOf(first, "album")]
      .filter(Boolean)
      .join(" ")
      .trim();
  });

  async function runSearch() {
    if (!query.trim() || searching) return;
    searching = true;
    error = null;
    proposal = null;
    try {
      hits = await searchAlbums(query, 12);
      searched = true;
    } catch (e) {
      error = String((e as any)?.message ?? e ?? "");
    } finally {
      searching = false;
    }
  }

  async function pick(hit: AlbumHit) {
    matching = true;
    error = null;
    try {
      proposal = await matchAlbum(
        hit.id,
        targets.map((file, index) => ({
          index,
          title: valueOf(file, "title"),
          track_number: Number(valueOf(file, "track_number")) || null,
          duration: null,
        })),
      );
      excluded = new Set();
      opened = new Set();
    } catch (e) {
      error = String((e as any)?.message ?? e ?? "");
    } finally {
      matching = false;
    }
  }

  // `$derived.by` et non `$derived` : la propriété est réassignée dans le
  // gabarit (« Changer »), ce qui suffit à faire perdre son type à l'inférence.
  let matches: TrackMatch[] = $derived.by(() => proposal?.matches ?? []);
  /** Ce qui n'a pas trouvé sa piste, ou a été mis de côté à la main. */
  let unmatched = $derived(
    matches.filter(
      (m) =>
        m.confidence === "rejected" ||
        m.remote_position === null ||
        excluded.has(m.local_index),
    ),
  );

  function remoteTrack(position: number | null): AlbumTrack | null {
    if (position === null || !proposal) return null;
    return proposal.album.tracks.find((tk) => tk.position === position) ?? null;
  }

  const localFile = (index: number): WorkshopFile | null => targets[index] ?? null;
  const shortName = (path: string) => path.split(/[\\/]/).pop() ?? path;

  /** Un champ d'un morceau, avec sa valeur d'ici et celle de là-bas. */
  type Cell = {
    field: SourceField;
    from: string;
    to: string;
    /** `absent` : la source ne dit rien. `same` : elle dit la même chose. */
    state: "differs" | "same" | "absent";
  };

  /** Un fichier et la piste distante à laquelle il a été apparié. */
  type Pair = {
    index: number;
    file: WorkshopFile;
    track: AlbumTrack;
    /** Douteux : l'appariement mérite un coup d'œil avant d'être suivi. */
    doubtful: boolean;
    cells: Cell[];
    diffs: Cell[];
  };

  let pairs: Pair[] = $derived.by(() => {
    const album = proposal?.album;
    if (!album) return [];

    const out: Pair[] = [];
    for (const match of matches) {
      if (match.confidence === "rejected" || excluded.has(match.local_index)) continue;
      const file = localFile(match.local_index);
      const track = remoteTrack(match.remote_position);
      if (!file || !track) continue;

      const values = proposedValues(album, track);
      const cells: Cell[] = SOURCE_FIELDS.map((field) => {
        const to = values[field] ?? "";
        const from = valueOf(file, field).trim();
        return {
          field,
          from,
          to,
          // Une valeur absente n'est pas une valeur vide : proposer du vide
          // effacerait ce que le fichier contient déjà.
          state: !to ? "absent" : to === from ? "same" : "differs",
        } as Cell;
      });

      out.push({
        index: match.local_index,
        file,
        track,
        doubtful: match.confidence === "doubtful",
        cells,
        diffs: cells.filter((c) => c.state === "differs"),
      });
    }
    return out;
  });

  // Cochage d'ouverture : les cases vides seulement. Remplir un trou ne détruit
  // rien, écraser du texte est une décision — donc visible et décochée.
  $effect(() => {
    const current = proposal;
    untrack(() => {
      if (!current) {
        chosen = new Set();
        return;
      }
      const next = new Set<string>();
      for (const pair of pairs) {
        for (const cell of pair.diffs) {
          if (cell.from === "") next.add(cellKey(pair.file.path, cell.field));
        }
      }
      chosen = next;
    });
  });

  // La pochette est téléchargée dès qu'un album est retenu : son poids réel et
  // son rendu après préparation font partie de la décision, et les découvrir
  // après l'écriture serait trop tard.
  $effect(() => {
    const url = proposal?.album.cover ?? null;
    untrack(() => {
      remoteCover = null;
      coverError = null;
      if (!url) return;

      coverLoading = true;
      prepareImageFromUrl(url)
        .then((img) => {
          remoteCover = img;
          // Cochée d'office sur les seuls fichiers qui n'en ont aucune :
          // remplir un trou ne détruit rien, écraser est une décision.
          const next = new Set(chosen);
          for (const pair of pairs) {
            if (!pair.file.cover) next.add(coverKey(pair.file.path));
          }
          chosen = next;
        })
        .catch((e) => (coverError = String((e as any)?.message ?? e ?? "")))
        .finally(() => (coverLoading = false));
    });
  });

  /** La source propose-t-elle une pochette exploitable ? */
  let hasCover = $derived(remoteCover !== null);
  /** Les fichiers qui recevront la pochette. */
  let coverPicked = $derived(
    hasCover ? pairs.filter((pair) => chosen.has(coverKey(pair.file.path))) : [],
  );
  /** Parmi eux, ceux dont la pochette actuelle sera écrasée. */
  let coversReplaced = $derived(coverPicked.filter((pair) => pair.file.cover).length);

  /** Toutes les valeurs modifiables, tous morceaux confondus. */
  let allDiffs = $derived(
    pairs.flatMap((pair) => pair.diffs.map((cell) => ({ pair, cell }))),
  );
  let picked = $derived(
    allDiffs.filter(({ pair, cell }) => chosen.has(cellKey(pair.file.path, cell.field))),
  );
  let changeCount = $derived(picked.length + coverPicked.length);
  let fileCount = $derived(
    new Set([
      ...picked.map(({ pair }) => pair.file.path),
      ...coverPicked.map((pair) => pair.file.path),
    ]).size,
  );

  /** Les champs qui ont au moins une divergence, avec leur état de sélection. */
  let fieldSummary = $derived(
    [
      ...SOURCE_FIELDS.map((field) => {
        const cells = allDiffs.filter(({ cell }) => cell.field === field);
        return {
          field: field as CellField,
          label: `tags.${field}`,
          total: cells.length,
          picked: cells.filter(({ pair }) => chosen.has(cellKey(pair.file.path, field)))
            .length,
        };
      }),
      {
        field: COVER as CellField,
        label: "tags.cover",
        total: hasCover ? pairs.length : 0,
        picked: coverPicked.length,
      },
    ].filter((f) => f.total > 0),
  );

  function toggleCell(path: string, field: SourceField) {
    const next = new Set(chosen);
    const key = cellKey(path, field);
    if (next.has(key)) next.delete(key);
    else next.add(key);
    chosen = next;
  }

  /** Bascule un ensemble de valeurs : tout décocher si tout était coché. */
  function toggleKeys(keys: string[]) {
    if (keys.length === 0) return;
    const next = new Set(chosen);
    if (keys.every((k) => next.has(k))) keys.forEach((k) => next.delete(k));
    else keys.forEach((k) => next.add(k));
    chosen = next;
  }

  /** Tout ce qu'un morceau peut recevoir : ses champs divergents et sa pochette. */
  const pairKeys = (pair: Pair) => [
    ...pair.diffs.map((c) => cellKey(pair.file.path, c.field)),
    ...(hasCover ? [coverKey(pair.file.path)] : []),
  ];

  /** Coche ou décoche un champ sur toute la sélection. */
  function toggleField(field: CellField) {
    toggleKeys(
      field === COVER
        ? pairs.map((pair) => coverKey(pair.file.path))
        : allDiffs
            .filter(({ cell }) => cell.field === field)
            .map(({ pair }) => cellKey(pair.file.path, field)),
    );
  }

  /** `all` : tout. `empty` : les trous seulement. `none` : rien. */
  function selectAll(mode: "all" | "empty" | "none") {
    if (mode === "none") {
      chosen = new Set();
      return;
    }
    const next = new Set<string>();
    for (const { pair, cell } of allDiffs) {
      if (mode === "all" || cell.from === "") {
        next.add(cellKey(pair.file.path, cell.field));
      }
    }
    if (hasCover) {
      // « Les vides » vaut aussi pour l'image : un fichier sans pochette est
      // un trou à combler, un fichier qui en a une n'est pas concerné.
      for (const pair of pairs) {
        if (mode === "all" || !pair.file.cover) next.add(coverKey(pair.file.path));
      }
    }
    chosen = next;
  }

  function toggleOpen(index: number) {
    const next = new Set(opened);
    if (next.has(index)) next.delete(index);
    else next.add(index);
    opened = next;
  }

  let allOpen = $derived(pairs.length > 0 && opened.size === pairs.length);
  function toggleAllOpen() {
    opened = allOpen ? new Set() : new Set(pairs.map((p) => p.index));
  }

  function toggleExcluded(index: number) {
    const next = new Set(excluded);
    if (next.has(index)) next.delete(index);
    else next.add(index);
    excluded = next;
  }

  function apply() {
    for (const { pair, cell } of picked) {
      tagWorkshop.set(pair.file.path, cell.field, cell.to);
    }
    if (remoteCover && coverPicked.length > 0) {
      tagWorkshop.setCoverOn(
        coverPicked.map((pair) => pair.file.path),
        { path: remoteCover.path, src: remoteCover.src },
      );
    }
    onapplied();
    popinStore.close();
  }
</script>

<!-- ══ Une valeur, dans le détail d'un morceau ══
     Case à cocher, libellé, valeur d'ici, valeur de là-bas. Les trois états
     sont distingués : ce qui diffère se coche, ce qui coïncide se constate, ce
     que la source ignore se dit — les confondre laisserait croire que Deezer
     confirme une valeur dont il ne sait rien. -->
{#snippet cellRow(pair: Pair, cell: Cell)}
  {@const on = chosen.has(cellKey(pair.file.path, cell.field))}
  <div
    class="flex items-center gap-2.5 px-2 py-1 rounded-md
           {on ? 'bg-emerald-500/10' : ''}
           {cell.state === 'differs' ? '' : 'opacity-55'}"
  >
    {#if cell.state === 'differs'}
      <input
        type="checkbox"
        checked={on}
        onchange={() => toggleCell(pair.file.path, cell.field)}
        aria-label={$t(`tags.${cell.field}`)}
        class="checkbox-app"
      />
    {:else}
      <span class="shrink-0 w-4 flex items-center justify-center">
        <Icon
          icon={cell.state === 'same' ? 'lucide:check' : 'lucide:minus'}
          width="10"
          class="text-neutral-300 dark:text-neutral-600"
        />
      </span>
    {/if}

    <span class="shrink-0 w-24 text-[11px] truncate
                 text-neutral-500 dark:text-neutral-400">
      {$t(`tags.${cell.field}`)}
    </span>

    <span class="flex-1 min-w-0 text-[11.5px] truncate
                 {cell.from
                   ? 'text-neutral-600 dark:text-neutral-300'
                   : 'italic text-neutral-400 dark:text-neutral-600'}"
          title={cell.from}>
      {cell.from || $t('workshop.was_empty')}
    </span>

    <Icon
      icon="lucide:arrow-right"
      width="10"
      class="shrink-0 {cell.state === 'differs'
        ? 'text-neutral-300 dark:text-neutral-600'
        : 'text-transparent'}"
    />

    <span class="flex-1 min-w-0 text-[11.5px] truncate
                 {cell.state === 'differs'
                   ? on
                     ? 'font-medium text-emerald-600 dark:text-emerald-400'
                     : 'text-neutral-500 dark:text-neutral-400'
                   : cell.state === 'same'
                     ? 'text-neutral-400 dark:text-neutral-500'
                     : 'italic text-neutral-400 dark:text-neutral-600'}"
          title={cell.to}>
      {cell.state === 'absent' ? $t('source.not_provided') : cell.to}
    </span>
  </div>
{/snippet}

<div class="flex-1 min-h-0 flex flex-col">
  <div class="flex-1 min-h-0 flex">
    <!-- ─────────── Colonne de la source ─────────── -->
    <aside
      class="shrink-0 w-64 flex flex-col min-h-0 overflow-y-auto scrollbar-none
             border-r border-neutral-200/70 dark:border-white/8
             bg-neutral-50/70 dark:bg-black/20"
    >
      {#if !proposal}
        <div class="shrink-0 p-4 space-y-2">
          <div class="flex gap-1.5">
            <input
              type="text"
              bind:value={query}
              onkeydown={(e) => e.key === 'Enter' && runSearch()}
              placeholder={$t('source.search_placeholder')}
              class="flex-1 min-w-0 h-8 px-2.5 rounded-lg text-[12px] outline-none
                     bg-white dark:bg-white/5
                     ring-1 ring-inset ring-neutral-200 dark:ring-white/10
                     focus:ring-emerald-500/60
                     text-neutral-900 dark:text-neutral-100
                     placeholder:text-neutral-400 dark:placeholder:text-neutral-600"
            />
            <button
              type="button"
              onclick={runSearch}
              disabled={searching || !query.trim()}
              aria-label={$t('source.search')}
              class="shrink-0 w-8 h-8 rounded-lg flex items-center justify-center
                     cursor-pointer transition-colors disabled:opacity-40
                     bg-emerald-500 text-white hover:bg-emerald-600"
            >
              <Icon icon={searching ? 'lucide:loader-circle' : 'lucide:search'}
                    width="13" class={searching ? 'animate-spin' : ''} />
            </button>
          </div>
          <p class="text-[10.5px] text-neutral-400 dark:text-neutral-500">
            {targets.length} {$t('tags.files')}
          </p>
        </div>

        <div class="flex-1 min-h-0 px-2 pb-3 space-y-0.5">
          {#each hits as hit (hit.id)}
            <button
              type="button"
              onclick={() => pick(hit)}
              disabled={matching}
              class="w-full flex items-center gap-2.5 p-1.5 rounded-lg text-left
                     cursor-pointer transition-colors disabled:opacity-50
                     hover:bg-neutral-200/60 dark:hover:bg-white/6"
            >
              <span class="shrink-0 w-10 h-10 rounded overflow-hidden
                           bg-neutral-200 dark:bg-white/5">
                {#if hit.cover}
                  <img src={hit.cover} alt="" class="w-full h-full object-cover" />
                {/if}
              </span>
              <span class="min-w-0 flex-1">
                <span class="block text-[12px] font-medium truncate
                             text-neutral-800 dark:text-neutral-100">{hit.title}</span>
                <!-- Le nombre de pistes est le premier indice qu'on tient le
                     bon album : une réédition ou une compilation n'en a pas
                     autant. -->
                <span class="block text-[10.5px] truncate
                             text-neutral-400 dark:text-neutral-500">
                  {hit.artist} · {hit.track_count} {$t('source.tracks')}
                  {#if hit.track_count === targets.length}
                    <span class="text-emerald-600 dark:text-emerald-400">✓</span>
                  {/if}
                </span>
              </span>
            </button>
          {:else}
            {#if searched && !searching}
              <p class="px-2 py-8 text-center text-[12px]
                        text-neutral-400 dark:text-neutral-500">
                {$t('source.no_result')}
              </p>
            {/if}
          {/each}
        </div>
      {:else}
        <!-- L'album retenu, en grand : c'est la pochette qui dit d'un coup
             d'œil si on s'est trompé de disque. -->
        <div class="shrink-0 p-4 space-y-3">
          <div class="aspect-square rounded-xl overflow-hidden
                      ring-1 ring-black/5 dark:ring-white/10
                      shadow-lg shadow-black/10 dark:shadow-black/50
                      bg-neutral-200/60 dark:bg-white/5">
            {#if proposal.album.cover}
              <img src={proposal.album.cover} alt="" class="w-full h-full object-cover" />
            {/if}
          </div>

          <div>
            <p class="text-[13px] font-medium leading-tight
                      text-neutral-800 dark:text-neutral-100">
              {proposal.album.title}
            </p>
            <p class="mt-0.5 text-[11.5px] text-neutral-500 dark:text-neutral-400">
              {proposal.album.artist}
            </p>
            <div class="mt-1.5 flex flex-wrap gap-1">
              {#each [proposal.album.year, proposal.album.genre].filter(Boolean) as badge (badge)}
                <span class="px-1.5 py-0.5 rounded text-[10px]
                             bg-neutral-200/70 dark:bg-white/10
                             text-neutral-500 dark:text-neutral-400">{badge}</span>
              {/each}
              <!-- Le nombre de pistes n'est pas un badge comme les autres :
                   c'est lui qui dit si on a désigné le bon disque. -->
              <span
                title="{proposal.album.tracks.length} {$t('source.tracks')} · {targets.length} {$t('tags.files')}"
                class="flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] tabular-nums
                       {proposal.album.tracks.length === targets.length
                         ? 'bg-emerald-500/15 text-emerald-600 dark:text-emerald-400'
                         : 'bg-amber-500/15 text-amber-600 dark:text-amber-400'}"
              >
                <Icon
                  icon={proposal.album.tracks.length === targets.length
                    ? 'lucide:check'
                    : 'lucide:triangle-alert'}
                  width="9"
                />
                {proposal.album.tracks.length} / {targets.length}
              </span>
            </div>
          </div>

          <button
            type="button"
            onclick={() => (proposal = null)}
            class="w-full flex items-center justify-center gap-1.5 px-2 h-7 rounded-lg
                   text-[11.5px] cursor-pointer transition-colors
                   text-neutral-500 dark:text-neutral-400
                   hover:bg-neutral-200/70 dark:hover:bg-white/8"
          >
            <Icon icon="lucide:repeat" width="12" />
            {$t('source.change')}
          </button>
        </div>

        <!-- ─── Raccourcis par champ ───
             Le geste par lot que la vue par morceau ne donne pas : « toutes
             les années », d'un clic, sans parcourir dix-neuf lignes. -->
        {#if fieldSummary.length > 0}
          <div class="shrink-0 mt-auto px-4 py-3 space-y-2
                      border-t border-neutral-200/70 dark:border-white/8">
            <p class="text-[10px] font-semibold uppercase tracking-widest
                      text-neutral-400 dark:text-neutral-500">
              {$t('source.by_field')}
            </p>
            <div class="flex flex-wrap gap-1">
              {#each fieldSummary as summary (summary.field)}
                {@const full = summary.picked === summary.total}
                {@const partial = summary.picked > 0 && !full}
                <button
                  type="button"
                  onclick={() => toggleField(summary.field)}
                  title="{summary.picked} / {summary.total}"
                  class="flex items-center gap-1 px-1.5 h-6 rounded-md text-[10.5px]
                         cursor-pointer transition-colors
                         {full
                           ? 'bg-emerald-500/18 text-emerald-600 dark:text-emerald-400'
                           : partial
                             ? 'bg-emerald-500/8 text-emerald-600/80 dark:text-emerald-400/80'
                             : 'bg-neutral-200/60 dark:bg-white/6 text-neutral-400 dark:text-neutral-500'}"
                >
                  {$t(summary.label)}
                  <span class="tabular-nums opacity-70">
                    {partial ? `${summary.picked}/${summary.total}` : summary.total}
                  </span>
                </button>
              {/each}
            </div>

            <p class="text-[10px] leading-relaxed text-neutral-400 dark:text-neutral-500">
              {$t('source.limits')}
            </p>
          </div>
        {/if}
      {/if}
    </aside>

    <!-- ─────────── Colonne des morceaux ─────────── -->
    <div class="flex-1 min-w-0 flex flex-col min-h-0">
      {#if !proposal}
        <div class="flex-1 flex flex-col items-center justify-center gap-2.5">
          <Icon icon="lucide:disc-3" width="30"
                class="text-neutral-300 dark:text-neutral-700" />
          <p class="text-[12.5px] text-neutral-400 dark:text-neutral-500">
            {$t('source.pick_album_first')}
          </p>
        </div>
      {:else}
        <!-- ─── Barre d'outils ─── -->
        <div class="shrink-0 flex items-center gap-1.5 px-4 py-2
                    border-b border-neutral-200/70 dark:border-white/8">
          <span class="text-[10px] font-semibold uppercase tracking-widest
                       text-neutral-400 dark:text-neutral-500">
            {$t('source.select')}
          </span>
          {#each [['all', 'source.all'], ['empty', 'source.empty_only'], ['none', 'source.none']] as [mode, key] (mode)}
            <button
              type="button"
              onclick={() => selectAll(mode as 'all' | 'empty' | 'none')}
              class="px-2 h-6 rounded-md text-[11px] cursor-pointer transition-colors
                     text-neutral-500 dark:text-neutral-400
                     hover:bg-neutral-200/70 dark:hover:bg-white/8"
            >
              {$t(key)}
            </button>
          {/each}

          <span class="flex-1"></span>

          <button
            type="button"
            onclick={toggleAllOpen}
            class="flex items-center gap-1.5 px-2 h-6 rounded-md text-[11px]
                   cursor-pointer transition-colors
                   text-neutral-500 dark:text-neutral-400
                   hover:bg-neutral-200/70 dark:hover:bg-white/8"
          >
            <Icon icon={allOpen ? 'lucide:chevrons-down-up' : 'lucide:chevrons-up-down'}
                  width="12" />
            {allOpen ? $t('source.collapse_all') : $t('source.expand_all')}
          </button>
        </div>

        <div class="flex-1 min-h-0 overflow-y-auto scrollbar-none px-3 py-2.5 space-y-1.5">
          <!-- ─── Portée ───
               Sans cette ligne, l'écran laissait croire qu'il parlait des
               trente-trois fichiers sélectionnés alors qu'il n'en concernait
               que dix-neuf. -->
          {#if unmatched.length > 0}
            <div class="rounded-lg bg-amber-500/10">
              <button
                type="button"
                onclick={() => (showUnmatched = !showUnmatched)}
                class="w-full flex items-center gap-2.5 px-3 py-2 text-left
                       rounded-lg cursor-pointer transition-colors
                       hover:bg-amber-500/8"
              >
                <Icon icon="lucide:triangle-alert" width="14"
                      class="shrink-0 text-amber-600 dark:text-amber-400" />
                <span class="min-w-0 flex-1">
                  <span class="block text-[12px] font-medium
                               text-amber-700 dark:text-amber-300">
                    {pairs.length} {$t('source.matched_of')} {targets.length}
                    {$t('tags.files')}
                  </span>
                  <!-- La cause tient presque toujours en un nombre : l'album
                       retenu n'a pas autant de pistes que la sélection. -->
                  <span class="block text-[10.5px] text-amber-600/80 dark:text-amber-400/70">
                    {$t('source.album_has')} {proposal.album.tracks.length}
                    {$t('source.tracks')} · {unmatched.length} {$t('source.left_untouched')}
                  </span>
                </span>
                <Icon icon={showUnmatched ? 'lucide:chevron-up' : 'lucide:chevron-down'}
                      width="13" class="shrink-0 text-amber-600/70 dark:text-amber-400/60" />
              </button>

              {#if showUnmatched}
                <div class="px-3 pb-2 space-y-0.5" transition:slide={{ duration: 140 }}>
                  {#each unmatched as match (match.local_index)}
                    {@const file = localFile(match.local_index)}
                    <div class="flex items-center gap-2.5 py-0.5">
                      <span class="flex-1 min-w-0 text-[11.5px] truncate
                                   text-neutral-600 dark:text-neutral-300">
                        {file ? valueOf(file, 'title') || shortName(file.path) : '—'}
                      </span>
                      {#if excluded.has(match.local_index)}
                        <button
                          type="button"
                          onclick={() => toggleExcluded(match.local_index)}
                          class="shrink-0 text-[10.5px] cursor-pointer transition-colors
                                 text-neutral-400 dark:text-neutral-500
                                 hover:text-neutral-800 dark:hover:text-neutral-100"
                        >
                          {$t('source.include')}
                        </button>
                      {/if}
                    </div>
                  {/each}
                </div>
              {/if}
            </div>
          {/if}

          <!-- ─── Les morceaux ─── -->
          {#each pairs as pair (pair.file.path)}
            {@const keys = pairKeys(pair)}
            {@const count = keys.filter((k) => chosen.has(k)).length}
            {@const open = opened.has(pair.index)}
            <div
              class="rounded-lg transition-colors
                     {count > 0 ? 'bg-emerald-500/8' : 'bg-neutral-100/70 dark:bg-white/3'}"
            >
              <div class="flex items-center gap-2.5 px-2.5 py-2">
                <input
                  type="checkbox"
                  checked={count > 0 && count === keys.length}
                  indeterminate={count > 0 && count < keys.length}
                  disabled={keys.length === 0}
                  onchange={() => toggleKeys(keys)}
                  aria-label={valueOf(pair.file, 'title') || shortName(pair.file.path)}
                  class="checkbox-app"
                />

                <!-- Le titre d'ici et celui de là-bas sur la même ligne : c'est
                     ce qui permet de voir sans déplier que la piste en face est
                     bien la sienne. -->
                <span class="flex-1 min-w-0 flex items-center gap-2">
                  <!-- La pochette du fichier, sur sa ligne : c'est ce qui
                       permet de repérer d'un coup d'œil ceux qui n'en ont
                       pas, sans déplier dix-neuf morceaux. -->
                  <span class="shrink-0 w-7 h-7 rounded overflow-hidden
                               ring-1 ring-black/5 dark:ring-white/10
                               bg-neutral-200/60 dark:bg-white/5">
                    {#if pair.file.cover}
                      <img src={pair.file.cover} alt="" class="w-full h-full object-cover" />
                    {:else}
                      <span class="w-full h-full flex items-center justify-center">
                        <Icon icon="lucide:image-off" width="11"
                              class="text-neutral-400 dark:text-neutral-600" />
                      </span>
                    {/if}
                  </span>

                  <span class="flex-1 min-w-0 text-[12px] truncate
                               text-neutral-700 dark:text-neutral-200"
                        title={shortName(pair.file.path)}>
                    {valueOf(pair.file, 'title') || shortName(pair.file.path)}
                  </span>
                  <Icon icon="lucide:arrow-right" width="10"
                        class="shrink-0 text-neutral-300 dark:text-neutral-600" />

                  <!-- Celle de Deezer n'apparaît que si elle est retenue pour
                       ce fichier : sinon elle promettrait un changement qui
                       n'aura pas lieu. -->
                  {#if remoteCover && chosen.has(coverKey(pair.file.path))}
                    <span class="shrink-0 w-7 h-7 rounded overflow-hidden
                                 ring-1 ring-emerald-500/60">
                      <img src={remoteCover.src} alt="" class="w-full h-full object-cover" />
                    </span>
                  {/if}
                  <span class="flex-1 min-w-0 text-[12px] truncate
                               text-neutral-500 dark:text-neutral-400"
                        title={pair.track.title}>
                    <span class="tabular-nums text-neutral-400 dark:text-neutral-500">
                      {pair.track.position}.
                    </span>
                    {pair.track.title}
                    <span class="text-[10px] text-neutral-400 dark:text-neutral-600">
                      {formatDuration(pair.track.duration)}
                    </span>
                  </span>
                </span>

                <!-- Un appariement douteux se signale ici et nulle part
                     ailleurs : c'est la ligne qu'on relira avant de la cocher. -->
                {#if pair.doubtful}
                  <span title={$t('source.doubtful')} class="shrink-0 flex">
                    <Icon icon="lucide:circle-help" width="12"
                          class="text-amber-600 dark:text-amber-400" />
                  </span>
                {/if}

                <span class="shrink-0 px-1.5 rounded text-[10px] tabular-nums
                             {count > 0
                               ? 'bg-emerald-500/15 text-emerald-600 dark:text-emerald-400'
                               : 'bg-neutral-200/70 dark:bg-white/10 text-neutral-400 dark:text-neutral-500'}">
                  {#if keys.length === 0}
                    {$t('source.identical_short')}
                  {:else}
                    {count}/{keys.length}
                  {/if}
                </span>

                <button
                  type="button"
                  onclick={() => toggleOpen(pair.index)}
                  aria-label={$t('source.detail')}
                  title={$t('source.detail')}
                  class="shrink-0 w-6 h-6 rounded-md flex items-center justify-center
                         cursor-pointer transition-colors
                         text-neutral-400 dark:text-neutral-500
                         hover:text-neutral-800 dark:hover:text-neutral-100
                         hover:bg-neutral-200/70 dark:hover:bg-white/8"
                >
                  <Icon icon={open ? 'lucide:chevron-up' : 'lucide:chevron-down'} width="13" />
                </button>
              </div>

              {#if open}
                <div class="px-2.5 pb-2" transition:slide={{ duration: 140 }}>
                  <!-- Les colonnes nommées dans chaque morceau déplié : au
                       moment où l'on compare deux valeurs, il faut savoir
                       laquelle est la sienne. -->
                  <div class="flex items-center gap-2.5 px-2 pb-1">
                    <span class="shrink-0 w-4"></span>
                    <span class="shrink-0 w-24"></span>
                    <span class="flex-1 min-w-0 text-[9.5px] font-semibold uppercase
                                 tracking-widest text-neutral-400 dark:text-neutral-500">
                      {$t('source.your_file')}
                    </span>
                    <span class="shrink-0 w-2.5"></span>
                    <span class="flex-1 min-w-0 text-[9.5px] font-semibold uppercase
                                 tracking-widest text-emerald-600/70 dark:text-emerald-400/70">
                      Deezer
                    </span>
                  </div>

                  <div class="space-y-px">
                    <!-- La pochette d'abord : c'est la seule ligne qu'on juge
                         à l'œil et non en lisant. -->
                    {#if remoteCover}
                      {@const on = chosen.has(coverKey(pair.file.path))}
                      <div class="flex items-center gap-2.5 px-2 py-1 rounded-md
                                  {on ? 'bg-emerald-500/10' : ''}">
                        <input
                          type="checkbox"
                          checked={on}
                          onchange={() => toggleKeys([coverKey(pair.file.path)])}
                          aria-label={$t('tags.cover')}
                          class="checkbox-app"
                        />
                        <span class="shrink-0 w-24 text-[11px] truncate
                                     text-neutral-500 dark:text-neutral-400">
                          {$t('tags.cover')}
                        </span>

                        <span class="flex-1 min-w-0 flex items-center gap-2">
                          <span class="shrink-0 w-9 h-9 rounded overflow-hidden
                                       ring-1 ring-black/5 dark:ring-white/10
                                       bg-neutral-200/60 dark:bg-white/5">
                            {#if pair.file.cover}
                              <img src={pair.file.cover} alt=""
                                   class="w-full h-full object-cover" />
                            {/if}
                          </span>
                          <span class="min-w-0 text-[11px] truncate
                                       {pair.file.cover
                                         ? 'text-neutral-500 dark:text-neutral-400'
                                         : 'italic text-neutral-400 dark:text-neutral-600'}">
                            {#if !pair.file.cover}
                              {$t('tags.no_media')}
                            {:else if on}
                              <!-- Le seul geste de cet écran qui détruit
                                   quelque chose : il se dit avant. -->
                              <span class="text-amber-600 dark:text-amber-400">
                                {$t('source.cover_replaced')}
                              </span>
                            {/if}
                          </span>
                        </span>

                        <Icon icon="lucide:arrow-right" width="10"
                              class="shrink-0 text-neutral-300 dark:text-neutral-600" />

                        <span class="flex-1 min-w-0 flex items-center gap-2">
                          <span class="shrink-0 w-9 h-9 rounded overflow-hidden
                                       ring-1 {on
                                         ? 'ring-emerald-500/60'
                                         : 'ring-black/5 dark:ring-white/10'}">
                            <img src={remoteCover.src} alt=""
                                 class="w-full h-full object-cover" />
                          </span>
                          <!-- Dimensions et poids **après préparation** : c'est
                               ce qui sera écrit, pas ce que Deezer a servi. -->
                          <span class="min-w-0 text-[10.5px] leading-tight truncate
                                       text-neutral-500 dark:text-neutral-400">
                            {remoteCover.width}×{remoteCover.height}<br />
                            {formatBytes(remoteCover.bytes)}
                          </span>
                        </span>
                      </div>
                    {/if}

                    {#each pair.cells as cell (cell.field)}
                      {@render cellRow(pair, cell)}
                    {/each}
                  </div>
                </div>
              {/if}
            </div>
          {/each}
        </div>
      {/if}
    </div>
  </div>

  {#if error || coverError}
    <div
      class="shrink-0 flex items-start gap-2 px-5 py-2
             bg-red-500/10 border-t border-red-500/20 text-red-500 text-[11px]"
      transition:fade={{ duration: 120 }}
    >
      <Icon icon="lucide:alert-triangle" width="13" class="shrink-0 mt-0.5" />
      <span class="min-w-0">{error ?? coverError}</span>
    </div>
  {/if}

  <!-- ─────────── Pied de page ─────────── -->
  <footer
    class="shrink-0 flex items-center gap-3 px-4 py-2.5
           border-t border-neutral-200/70 dark:border-white/8
           bg-neutral-50/80 dark:bg-black/25"
  >
    <div class="flex-1 min-w-0">
      {#if changeCount > 0}
        <span class="flex items-center gap-1.5 text-[11.5px]
                     text-neutral-600 dark:text-neutral-300">
          <span class="w-1.5 h-1.5 rounded-full bg-emerald-500"></span>
          <!-- Deux nombres, parce qu'ils ne disent pas la même chose : la
               quantité de valeurs et l'étendue des fichiers touchés. -->
          {changeCount} {$t('source.values_changed')} · {fileCount} {$t('tags.files')}
        </span>
      {:else}
        <span class="text-[11.5px] text-neutral-400 dark:text-neutral-500">
          {$t('source.nothing_selected')}
        </span>
      {/if}
    </div>

    <button
      type="button"
      onclick={() => popinStore.requestClose()}
      class="flex items-center gap-1.5 px-3 h-8 rounded-lg text-[13px]
             cursor-pointer transition-colors
             text-neutral-600 dark:text-neutral-300
             hover:bg-neutral-200/70 dark:hover:bg-white/8"
    >
      <Icon icon="lucide:x" width="13" />
      {$t('profil.cancel')}
    </button>
    <button
      type="button"
      onclick={apply}
      disabled={changeCount === 0}
      class="flex items-center gap-1.5 px-3.5 h-8 rounded-lg text-[13px] font-medium
             cursor-pointer transition-all
             bg-emerald-500 text-white
             hover:bg-emerald-600 hover:shadow-lg hover:shadow-emerald-500/40
             disabled:opacity-35 disabled:cursor-not-allowed disabled:shadow-none"
    >
      <Icon icon="lucide:check" width="13" />
      {$t('source.apply')}
    </button>
  </footer>
</div>
