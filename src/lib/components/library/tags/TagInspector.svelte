<script lang="ts">
  // Panneau d'édition : les champs de la sélection, une valeur posée sur tous les morceaux choisis.
  // Mêmes champs que « Modifier les tags » (TagField). Valeurs divergentes laissées vides : conservées.
  import Icon from "@iconify/svelte";
  import TagField from "#lib/components/ui/input/TagField.svelte";
  import { t } from "#lib/i18n";
  import { tagWorkshop, isDirty, valueOf, type WorkshopField } from "#lib/stores/tags/tagWorkshop.store";
  import { fileName } from "#lib/services/batch/batch.service";
  import { FLAGGED_WHEN_EMPTY } from "./TagTable.svelte";

  let {
    source,
    coverBusy,
    onpickcover,
    onnotice,
  }: {
    /** Ce qu'on corrige (l'album), affiché quand plusieurs morceaux sont choisis. */
    source: string;
    coverBusy: boolean;
    onpickcover: () => void;
    onnotice: (message: string, undoable: boolean) => void;
  } = $props();

  let workshop = $derived($tagWorkshop);
  let targets = $derived(workshop.files.filter((f) => f.readable && workshop.selected.has(f.path)));
  let one = $derived(targets.length === 1);

  /** Valeur commune, ou `null` quand les morceaux divergent. */
  function common(field: WorkshopField): string | null {
    if (targets.length === 0) return "";
    const first = valueOf(targets[0], field);
    return targets.every((f) => valueOf(f, field) === first) ? first : null;
  }

  let cover = $derived(targets.find((f) => f.pendingCover)?.pendingCover?.src ?? targets.find((f) => f.cover)?.cover ?? null);
  let coverState = $derived(
    targets.some((f) => f.pendingCover)
      ? $t("workshop.cover_pending")
      : targets.every((f) => f.cover)
        ? $t("workshop.cover_embedded")
        : targets.some((f) => f.cover)
          ? $t("workshop.cover_partial")
          : $t("tags.no_cover"),
  );

  // Genres déjà présents dans l'atelier : les plus probables pour cet album.
  let genres = $derived.by(() => {
    const counts = new Map<string, number>();
    for (const f of workshop.files) {
      const g = valueOf(f, "genre").trim();
      if (g) counts.set(g, (counts.get(g) ?? 0) + 1);
    }
    const current = common("genre");
    return [...counts.entries()].sort((a, b) => b[1] - a[1]).map(([g]) => g).filter((g) => g !== current).slice(0, 5);
  });

  const plural = (n: number) => `${n} ${n > 1 ? $t("workshop.track_many") : $t("workshop.track_one")}`;

  function apply(field: WorkshopField, raw: string, input?: HTMLInputElement) {
    const value = raw.replace(/\s+/g, " ").trim();
    // « Mixte » laissé vide : on conserve les valeurs de chacun.
    if (common(field) === null && value === "") {
      if (input) input.value = "";
      return;
    }
    const before = $tagWorkshop.revision;
    tagWorkshop.setOnSelected(field, value);
    if (targets.length > 1 && $tagWorkshop.revision !== before) {
      onnotice(`${$t(`tags.${field}`)} · ${plural(targets.length)}`, true);
    }
  }

  /** Rétablit la valeur d'origine du champ sur toute la sélection. */
  function revert(field: WorkshopField) {
    tagWorkshop.group(() => targets.forEach((f) => tagWorkshop.revertField(f.path, field)));
  }

  /** Numérote la sélection de 1 à N, depuis le panneau. */
  function numberAll() {
    const before = $tagWorkshop.revision;
    tagWorkshop.numberSelected();
    if ($tagWorkshop.revision !== before) onnotice($t("workshop.numbered").replace("{n}", String(targets.length)), true);
    else onnotice($t("workshop.already_numbered"), false);
  }

  type Field = { field: WorkshopField; half?: boolean; perFile?: boolean };
  const GROUPS: { title: string; fields: Field[] }[] = [
    { title: "tags.section_track", fields: [{ field: "title", perFile: true }, { field: "artist" }, { field: "track_number", half: true, perFile: true }, { field: "disc_number", half: true }] },
    { title: "tags.section_album", fields: [{ field: "album" }, { field: "album_artist" }, { field: "year", half: true }, { field: "genre", half: true }, { field: "total_tracks", half: true }, { field: "total_discs", half: true }] },
    { title: "workshop.credits", fields: [{ field: "composer" }, { field: "comment" }] },
  ];
</script>

<aside class="w-80 shrink-0 overflow-y-auto scrollbar-app border-l border-(--rg-bd) bg-(--rg-creux) px-5 pt-5 pb-32">
  {#if targets.length === 0}
    <p class="px-2.5 py-16 text-center text-[14px] leading-relaxed text-(--rg-mu)">{$t("workshop.select_to_edit")}</p>
  {:else}
    <div class="flex items-center gap-3.5 pb-4.5 border-b border-(--rg-bd)">
      <button type="button" onclick={onpickcover} disabled={coverBusy}
              class="group relative w-18 h-18 shrink-0 rounded-xl overflow-hidden bg-(--rg-s2) shadow-[0_8px_22px_rgba(0,0,0,0.4)] cursor-pointer
                     {targets.some((f) => f.pendingCover) ? 'ring-2 ring-(--rg-g)' : ''}"
              title={$t("workshop.set_cover_hint")} aria-label={$t("workshop.set_cover")}>
        {#if cover}
          <img src={cover} alt="" class="w-full h-full object-cover" />
        {:else}
          <Icon icon="material-symbols:hide-image-outline-rounded" width="26" class="absolute inset-0 m-auto text-(--rg-mu2)" />
        {/if}
        <span class="absolute inset-0 flex flex-col items-center justify-center gap-0.5 bg-black/60 text-white text-[11.5px] font-bold
                     transition-opacity {coverBusy ? 'opacity-100' : 'opacity-0 group-hover:opacity-100'}">
          <Icon icon={coverBusy ? "material-symbols:progress-activity" : "material-symbols:image-outline-rounded"} width="20" class={coverBusy ? "animate-spin" : ""} />
          {$t("tags.replace_image")}
        </span>
      </button>
      <div class="min-w-0">
        <p class="text-[16px] font-extrabold tracking-[-0.01em] truncate text-(--rg-tx)">
          {one ? valueOf(targets[0], "title") || fileName(targets[0].path) : plural(targets.length)}
        </p>
        <p class="mt-1 text-[12.5px] leading-[1.45] text-(--rg-mu)">
          <span class="block truncate">{one ? fileName(targets[0].path) : source}</span>
          {coverState}
        </p>
      </div>
    </div>

    {#each GROUPS as group (group.title)}
      <section class="pt-4.5 pb-1">
        <h4 class="mb-3 text-[11px] font-bold tracking-widest uppercase text-(--rg-mu)">{$t(group.title)}</h4>
        <div class="grid grid-cols-2 gap-2">
          {#each group.fields as { field, half, perFile } (field)}
            {@const value = common(field)}
            {@const locked = !!perFile && !one}
            {@const missing = value === "" && FLAGGED_WHEN_EMPTY.includes(field)}
            {@const dirty = targets.some((f) => isDirty(f, field))}
            <div class="min-w-0 {half && !locked ? '' : 'col-span-2'}">
              <TagField label={$t(`tags.${field}`)} {dirty}
                        warning={missing ? $t("workshop.missing") : ""}
                        revertLabel={$t("tags.revert")} onrevert={() => revert(field)}>
                {#if locked}
                  <!-- Propre à chaque morceau : une phrase et, pour le numéro, le bon geste. -->
                  <span class="mt-1.5 flex items-center gap-2 text-[13px] leading-tight text-(--rg-mu)">
                    <span class="flex-1 min-w-0 truncate">{field === "track_number" ? $t("workshop.per_track_short") : $t("workshop.per_track")}</span>
                    {#if field === "track_number"}
                      <button type="button" onclick={numberAll}
                              class="shrink-0 -my-1 h-6.5 px-2 rounded-md flex items-center gap-1 text-[12px] font-semibold cursor-pointer transition-colors
                                     text-(--rg-gtx) hover:bg-(--rg-gbg)">
                        <Icon icon="material-symbols:format-list-numbered-rounded" width="15" />{$t("workshop.number_short")}
                      </button>
                    {/if}
                  </span>
                {:else}
                  <input
                    data-focus-ring="none"
                    value={value ?? ""}
                    inputmode={field === "year" || field.includes("number") || field.includes("total") ? "numeric" : undefined}
                    placeholder={value === null ? $t("workshop.mixed_keep") : "—"}
                    onchange={(e) => apply(field, e.currentTarget.value, e.currentTarget)}
                    onkeydown={(e) => e.key === "Enter" && e.currentTarget.blur()}
                    class="w-full mt-1.5 bg-transparent leading-tight outline-none text-[13px] text-(--rg-tx) placeholder:text-(--rg-ph) {dirty ? 'pr-8' : ''}"
                  />
                {/if}
              </TagField>
            </div>
          {/each}
          {#if group.title === "tags.section_album" && genres.length > 0}
            <div class="col-span-2 flex flex-wrap gap-1.5 -mt-1">
              {#each genres as genre (genre)}
                <button type="button" onclick={() => apply("genre", genre)}
                        class="h-6.5 px-2.5 rounded-full border border-(--rg-bd) text-[12px] font-semibold cursor-pointer transition-colors
                               text-(--rg-mu) hover:border-(--rg-gbd) hover:bg-(--rg-gbg) hover:text-(--rg-gtx)">{genre}</button>
              {/each}
            </div>
          {/if}
        </div>
      </section>
    {/each}

    <p class="mt-4.5 text-[12.5px] leading-normal text-(--rg-mu)">
      {#if one}
        {$t("workshop.applies_one")}
      {:else}
        {@html $t("workshop.applies_many").replace("{n}", `<b class="text-(--rg-tx2)">${plural(targets.length)}</b>`)}
      {/if}
    </p>
  {/if}
</aside>

