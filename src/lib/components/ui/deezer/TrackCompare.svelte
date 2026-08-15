<script lang="ts">
  // Ce que le fichier contient, ce que la source propose, et ce qu'on retient.
  //
  // # Deux colonnes, et le choix au milieu
  // La version précédente versait tout dans le formulaire d'un coup : on
  // découvrait après coup ce qui avait bougé, en relisant douze champs. Ici
  // rien n'est appliqué avant d'avoir vu les deux valeurs côte à côte et coché
  // ce qu'on garde — pochette comprise.
  //
  // # Trois états, pas un
  // Un champ ne « diffère » pas ou ne diffère pas : la source peut ne rien en
  // dire. Confondre « identique » et « non fourni » ferait croire que Deezer
  // confirme une valeur qu'il ignore. Les lignes le disent séparément et ne
  // sont cochables que dans le cas où il y a un choix à faire.
  import Icon from "@iconify/svelte";
  import { untrack } from "svelte";
  import { fade } from "svelte/transition";
  import { t } from "$lib/i18n";
  import {
    formatBytes,
    prepareImageFromUrl,
    type DownloadedImage,
    type MediaSlot,
  } from "$lib/services/tags/tagEditor.service";
  import {
    formatDuration,
    type TrackHit,
    type TrackValues,
  } from "$lib/services/tags/metadata.service";

  const {
    current,
    currentCover,
    values,
    hit,
    onback,
    onapply,
  }: {
    /** Les valeurs telles qu'elles sont **à l'écran**, pas telles qu'elles
     *  étaient dans le fichier : c'est à celles-là qu'on se compare. */
    current: Record<string, string>;
    currentCover: MediaSlot | null;
    values: TrackValues;
    hit: TrackHit;
    onback: () => void;
    onapply: (fields: Record<string, string>, cover: DownloadedImage | null) => void;
  } = $props();

  const FIELDS = [
    "title",
    "artist",
    "album",
    "album_artist",
    "year",
    "genre",
    "track_number",
    "total_tracks",
    "disc_number",
  ] as const;

  /** Ce que la source propose, ramené au format du formulaire. */
  let proposed: Record<string, string> = $derived({
    title: values.title,
    artist: values.artist,
    album: values.album,
    album_artist: values.album_artist ?? "",
    year: values.year ?? "",
    genre: values.genre ?? "",
    track_number: values.track_number ? String(values.track_number) : "",
    total_tracks: values.total_tracks ? String(values.total_tracks) : "",
    disc_number: values.disc_number ? String(values.disc_number) : "",
  });

  type RowState = "differs" | "same" | "absent";

  const stateOf = (key: string): RowState => {
    const to = proposed[key] ?? "";
    if (!to) return "absent";
    return to === (current[key] ?? "") ? "same" : "differs";
  };

  let rows = $derived(
    FIELDS.map((key) => ({
      key,
      from: current[key] ?? "",
      to: proposed[key] ?? "",
      state: stateOf(key),
    })),
  );

  // Cochés d'emblée : ce sont les lignes qu'on est venu chercher. Décocher les
  // deux ou trois qu'on veut garder est plus rapide que d'en cocher huit.
  //
  // Une seule fois, à l'ouverture : recalculer la sélection à chaque fois que
  // le formulaire bouge recocherait ce qu'on vient de décocher.
  let selected: Set<string> = $state(
    untrack(() => new Set<string>(FIELDS.filter((key) => stateOf(key) === "differs"))),
  );

  let remoteCover: DownloadedImage | null = $state(null);
  let coverLoading = $state(false);
  let coverError: string | null = $state(null);
  /** La pochette ne se remplace que si on le demande — voir plus bas. */
  let takeCover = $state(false);

  // La pochette distante est téléchargée et apprêtée tout de suite : son poids
  // réel et son rendu après préparation font partie de la décision, et les
  // découvrir après l'enregistrement serait trop tard.
  $effect(() => {
    if (!values.cover) return;
    untrack(() => {
      if (coverLoading || remoteCover) return;
      coverLoading = true;
      prepareImageFromUrl(values.cover!)
        .then((img) => {
          remoteCover = img;
          // Un fichier sans pochette n'a rien à perdre : on la prend. S'il en
          // a déjà une, l'écraser est une décision, pas un défaut.
          takeCover = currentCover === null;
        })
        .catch((e) => (coverError = String((e as any)?.message ?? e ?? "")))
        .finally(() => (coverLoading = false));
    });
  });

  function toggle(key: string) {
    const next = new Set(selected);
    if (next.has(key)) next.delete(key);
    else next.add(key);
    selected = next;
  }

  let selectable = $derived(rows.filter((r) => r.state === "differs"));
  let changeCount = $derived(
    selectable.filter((r) => selected.has(r.key)).length + (takeCover ? 1 : 0),
  );
  let allChecked = $derived(
    selectable.length > 0 && selectable.every((r) => selected.has(r.key)),
  );

  function toggleAll() {
    selected = allChecked ? new Set() : new Set(selectable.map((r) => r.key));
  }

  function apply() {
    const fields: Record<string, string> = {};
    for (const row of selectable) {
      if (selected.has(row.key)) fields[row.key] = row.to;
    }
    onapply(fields, takeCover ? remoteCover : null);
  }
</script>

<div class="flex flex-col min-h-0 h-full">
  <!-- ─── La piste retenue ─── -->
  <div class="shrink-0 flex items-center gap-2.5 px-4 py-2.5
              border-b border-neutral-200/70 dark:border-white/8">
    <button
      type="button"
      onclick={onback}
      title={$t('source.back_to_results')}
      aria-label={$t('source.back_to_results')}
      class="shrink-0 w-7 h-7 rounded-lg flex items-center justify-center
             cursor-pointer transition-colors
             text-neutral-500 dark:text-neutral-400
             hover:bg-neutral-200/70 dark:hover:bg-white/8"
    >
      <Icon icon="lucide:arrow-left" width="13" />
    </button>
    <span class="min-w-0 flex-1">
      <span class="block text-[12.5px] font-medium truncate
                   text-neutral-800 dark:text-neutral-100">{hit.title}</span>
      <span class="block text-[10.5px] truncate text-neutral-400 dark:text-neutral-500">
        {hit.artist} · {hit.album} · {formatDuration(hit.duration)}
      </span>
    </span>
  </div>

  <div class="flex-1 min-h-0 overflow-y-auto scrollbar-none px-4 py-3">
    <!-- ─── En-têtes des deux colonnes ───
         Nommées une fois en haut : les répéter sur chaque ligne doublerait le
         texte pour ne rien ajouter. -->
    <div class="flex items-center gap-2.5 px-2 pb-1.5">
      <span class="shrink-0 w-4"></span>
      <span class="shrink-0 w-24"></span>
      <span class="flex-1 min-w-0 text-[9.5px] font-semibold uppercase tracking-[0.1em]
                   text-neutral-400 dark:text-neutral-500">
        {$t('source.your_file')}
      </span>
      <span class="shrink-0 w-3"></span>
      <span class="flex-1 min-w-0 text-[9.5px] font-semibold uppercase tracking-[0.1em]
                   text-emerald-600/70 dark:text-emerald-400/70">
        Deezer
      </span>
    </div>

    <!-- ─── Pochette ───
         En tête et sur deux vignettes : c'est la seule ligne qu'on juge à
         l'œil et non en lisant. -->
    {#if values.cover || currentCover}
      <div class="flex items-center gap-2.5 px-2 py-2 mb-1 rounded-lg
                  {takeCover ? 'bg-emerald-500/8' : 'bg-neutral-100/70 dark:bg-white/3'}">
        <input
          type="checkbox"
          bind:checked={takeCover}
          disabled={!remoteCover}
          aria-label={$t('tags.cover')}
          class="checkbox-app"
        />
        <span class="shrink-0 w-24 text-[11px] font-medium
                     text-neutral-600 dark:text-neutral-300">
          {$t('tags.cover')}
        </span>

        <span class="flex-1 min-w-0 flex items-center gap-2">
          <span class="shrink-0 w-12 h-12 rounded overflow-hidden
                       ring-1 ring-black/5 dark:ring-white/10
                       bg-neutral-200/60 dark:bg-white/5">
            {#if currentCover}
              <img src={currentCover.src} alt="" class="w-full h-full object-cover" />
            {/if}
          </span>
          <span class="min-w-0 text-[10.5px] leading-tight
                       text-neutral-400 dark:text-neutral-500">
            {#if currentCover}
              {currentCover.mime_type.replace('image/', '').toUpperCase()}<br />
              {formatBytes(currentCover.bytes)}
            {:else}
              <span class="italic">{$t('tags.no_media')}</span>
            {/if}
          </span>
        </span>

        <Icon icon="lucide:arrow-right" width="11"
              class="shrink-0 text-neutral-300 dark:text-neutral-600" />

        <span class="flex-1 min-w-0 flex items-center gap-2">
          <span class="shrink-0 w-12 h-12 rounded overflow-hidden
                       ring-1 ring-black/5 dark:ring-white/10
                       bg-neutral-200/60 dark:bg-white/5">
            {#if remoteCover}
              <img src={remoteCover.src} alt="" class="w-full h-full object-cover" />
            {:else if coverLoading}
              <span class="w-full h-full flex items-center justify-center">
                <Icon icon="lucide:loader-circle" width="14"
                      class="animate-spin text-neutral-400 dark:text-neutral-500" />
              </span>
            {/if}
          </span>
          <span class="min-w-0 text-[10.5px] leading-tight
                       text-neutral-500 dark:text-neutral-400">
            {#if remoteCover}
              <!-- Les dimensions et le poids **après préparation** : c'est ce
                   qui sera écrit, pas ce que Deezer a servi. -->
              {remoteCover.width}×{remoteCover.height}<br />
              {formatBytes(remoteCover.bytes)}
              {#if remoteCover.recompressed}
                <span class="text-amber-500">· {$t('tags.reduced')}</span>
              {/if}
            {:else if coverError}
              <span class="text-red-500">{coverError}</span>
            {:else if !values.cover}
              <span class="italic text-neutral-400 dark:text-neutral-500">
                {$t('source.not_provided')}
              </span>
            {/if}
          </span>
        </span>
      </div>
    {/if}

    <!-- ─── Les champs ─── -->
    <div class="space-y-0.5">
      {#each rows as row (row.key)}
        {@const on = row.state === 'differs' && selected.has(row.key)}
        <div
          class="flex items-center gap-2.5 px-2 py-1.5 rounded-lg
                 {on ? 'bg-emerald-500/8' : 'bg-neutral-100/70 dark:bg-white/3'}
                 {row.state === 'differs' ? '' : 'opacity-60'}"
        >
          {#if row.state === 'differs'}
            <input
              type="checkbox"
              checked={selected.has(row.key)}
              onchange={() => toggle(row.key)}
              aria-label={$t(`tags.${row.key}`)}
              class="checkbox-app"
            />
          {:else}
            <span class="shrink-0 w-4 flex items-center justify-center">
              <Icon
                icon={row.state === 'same' ? 'lucide:check' : 'lucide:minus'}
                width="11"
                class="text-neutral-300 dark:text-neutral-600"
              />
            </span>
          {/if}

          <span class="shrink-0 w-24 text-[11px] truncate
                       text-neutral-600 dark:text-neutral-300">
            {$t(`tags.${row.key}`)}
          </span>

          <span class="flex-1 min-w-0 text-[12px] truncate
                       {row.from
                         ? 'text-neutral-600 dark:text-neutral-300'
                         : 'italic text-neutral-400 dark:text-neutral-600'}"
                title={row.from}>
            {row.from || $t('workshop.was_empty')}
          </span>

          <Icon
            icon={row.state === 'differs' ? 'lucide:arrow-right' : 'lucide:equal'}
            width="11"
            class="shrink-0 {row.state === 'differs'
              ? 'text-neutral-300 dark:text-neutral-600'
              : 'text-transparent'}"
          />

          <span class="flex-1 min-w-0 text-[12px] truncate
                       {row.state === 'differs'
                         ? 'font-medium text-emerald-600 dark:text-emerald-400'
                         : row.state === 'same'
                           ? 'text-neutral-400 dark:text-neutral-500'
                           : 'italic text-neutral-400 dark:text-neutral-600'}"
                title={row.to}>
            {#if row.state === 'absent'}
              {$t('source.not_provided')}
            {:else}
              {row.to}
            {/if}
          </span>
        </div>
      {/each}
    </div>

    <!-- Le compositeur et le commentaire ne figurent pas dans la liste : les
         y montrer vides laisserait croire que Deezer propose de les effacer. -->
    <p class="mt-2.5 px-2 text-[10px] leading-relaxed
              text-neutral-400 dark:text-neutral-500">
      {$t('source.limits')}
    </p>
  </div>

  <!-- ─── Pied de la comparaison ─── -->
  <div class="shrink-0 flex items-center gap-2 px-4 py-2.5
              border-t border-neutral-200/70 dark:border-white/8">
    {#if selectable.length > 0}
      <button
        type="button"
        onclick={toggleAll}
        class="text-[11px] cursor-pointer underline decoration-dotted underline-offset-2
               text-neutral-400 dark:text-neutral-500
               hover:text-neutral-700 dark:hover:text-neutral-200 transition-colors"
      >
        {allChecked ? $t('source.select_none') : $t('source.select_all')}
      </button>
    {/if}

    <span class="flex-1 min-w-0 text-[11.5px] text-neutral-500 dark:text-neutral-400">
      {#if changeCount > 0}
        <span transition:fade={{ duration: 120 }}>
          {changeCount} {$t('source.selected_changes')}
        </span>
      {:else}
        {$t('source.nothing_selected')}
      {/if}
    </span>

    <button
      type="button"
      onclick={apply}
      disabled={changeCount === 0}
      class="flex items-center gap-1.5 px-3.5 h-8 rounded-lg text-[12.5px] font-medium
             cursor-pointer transition-all
             bg-emerald-500 text-white
             hover:bg-emerald-600 hover:shadow-lg hover:shadow-emerald-500/40
             disabled:opacity-35 disabled:cursor-not-allowed disabled:shadow-none"
    >
      <Icon icon="lucide:check" width="12" />
      {$t('source.apply')}
    </button>
  </div>
</div>
