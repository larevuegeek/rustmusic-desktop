<script lang="ts">
  // Métadonnées Deezer pour la sélection : rien n'est écrit, Appliquer remplit les modifs en attente.
  // Choix par valeur, morceau ou champ ; seules les cases vides sont cochées d'office.
  import PopinFooter from "#lib/components/ui/popin/PopinFooter.svelte";
  import PopinError from "#lib/components/ui/popin/PopinError.svelte";
  import { messageErreur } from "#lib/helper/tools/errorTools";
  import { popinStore } from "#lib/stores/ui/popin.store";
  import { t } from "#lib/i18n";
  import { tagWorkshop, valueOf } from "#lib/stores/tags/tagWorkshop.store";
  import {
    matchAlbum,
    searchAlbums,
    type AlbumHit,
  } from "#lib/services/tags/metadata.service";
  import SourceAlbumPanel from "./source/SourceAlbumPanel.svelte";
  import SourceTrackList from "./source/SourceTrackList.svelte";
  import { SourceSelection } from "./source/sourceSelection.svelte.js";

  /* eslint-disable svelte/valid-prop-names-in-kit-pages */
  const { onapplied = () => {} }: { onapplied?: () => void } = $props();

  let workshop = $derived($tagWorkshop);
  let targets = $derived(
    workshop.files.filter((f) => f.readable && workshop.selected.has(f.path)),
  );

  let query = $state("");
  let hits: AlbumHit[] = $state([]);
  let searching = $state(false);
  let matching = $state(false);
  let searched = $state(false);
  let error: string | null = $state(null);

  // La requête se devine : l'artiste et l'album qu'on est venu corriger.
  $effect(() => {
    if (query !== "" || targets.length === 0) return;
    const first = targets[0];
    query = [valueOf(first, "artist"), valueOf(first, "album")]
      .filter(Boolean)
      .join(" ")
      .trim();
  });

  const selection = new SourceSelection(() => targets);

  async function runSearch() {
    if (!query.trim() || searching) return;
    searching = true;
    error = null;
    selection.proposal = null;
    try {
      hits = await searchAlbums(query, 12);
      searched = true;
    } catch (e) {
      error = messageErreur(e);
    } finally {
      searching = false;
    }
  }

  async function pick(hit: AlbumHit) {
    matching = true;
    error = null;
    try {
      selection.load(
        await matchAlbum(
          hit.id,
          targets.map((file, index) => ({
            index,
            title: valueOf(file, "title"),
            track_number: Number(valueOf(file, "track_number")) || null,
            duration: null,
          })),
        ),
      );
    } catch (e) {
      error = messageErreur(e);
    } finally {
      matching = false;
    }
  }

  function apply() {
    for (const { pair, cell } of selection.picked) {
      tagWorkshop.set(pair.file.path, cell.field, cell.to);
    }
    const { remoteCover, coverPicked } = selection;
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

<div class="flex-1 min-h-0 flex flex-col">
  <div class="flex-1 min-h-0 flex">
    <SourceAlbumPanel
      {selection}
      bind:query
      {hits}
      {searching}
      {matching}
      {searched}
      onsearch={runSearch}
      onpick={pick}
    />
    <SourceTrackList {selection} />
  </div>

  {#if error || selection.coverError}
    <PopinError message={error ?? selection.coverError ?? ""} />
  {/if}

  <!-- ─────────── Pied de page ─────────── -->
  <PopinFooter cancelLabel={$t('profil.cancel')} oncancel={() => popinStore.requestClose()}
               submitLabel={$t('source.apply')} onsubmit={apply} submitDisabled={selection.changeCount === 0}>
    {#snippet start()}
      {#if selection.changeCount > 0}
        <span class="flex items-center gap-1.5 text-[11.5px]
                     text-neutral-600 dark:text-neutral-300">
          <span class="w-1.5 h-1.5 rounded-full bg-emerald-500"></span>
          <!-- Deux nombres : quantité de valeurs et étendue des fichiers touchés. -->
          {selection.changeCount} {$t('source.values_changed')} · {selection.fileCount} {$t('tags.files')}
        </span>
      {:else}
        <span class="text-[11.5px] text-neutral-400 dark:text-neutral-500">
          {$t('source.nothing_selected')}
        </span>
      {/if}
    {/snippet}
  </PopinFooter>
</div>
