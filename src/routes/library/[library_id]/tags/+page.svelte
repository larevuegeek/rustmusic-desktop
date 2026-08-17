<script lang="ts">
  // Atelier de tags.
  //
  // # Pourquoi une page et pas une popin
  // Corriger une bibliothèque n'est pas une action ponctuelle qu'on valide et
  // qu'on referme : on trie, on compare, on écoute pour vérifier, on revient.
  // Une fenêtre modale interdit tout ça. La page a une adresse, un bouton
  // retour, et laisse le lecteur accessible.
  //
  // # Deux vues, un seul état
  // Le tableur sert à **repérer** — trente fichiers à l'écran, les manques
  // sautent aux yeux. Le panneau sert à **corriger en profondeur** — champs
  // longs confortables, une valeur posée sur toute la sélection. Les deux
  // écrivent dans le même magasin, fichier par fichier, donc la bascule ne
  // perd jamais rien.
  import Icon from "@iconify/svelte";
  import { page } from "$app/state";
  import { goto, beforeNavigate } from "$app/navigation";
  import { t } from "$lib/i18n";
  import TagGrid from "$lib/components/library/tags/TagGrid.svelte";
  import TagPanelView from "$lib/components/library/tags/TagPanelView.svelte";
  import TagAuditView from "$lib/components/library/tags/TagAuditView.svelte";
  import TagSourcePopin from "$lib/components/library/common/popin/TagSourcePopin.svelte";
  import RenamePopin from "$lib/components/library/common/popin/RenamePopin.svelte";
  import BatchHistoryPopin from "$lib/components/library/common/popin/BatchHistoryPopin.svelte";
  import CleanTagsPopin from "$lib/components/library/common/popin/CleanTagsPopin.svelte";
  import { popinStore } from "$lib/stores/ui/popin.store";
  import {
    tagWorkshop,
    fileIsDirty,
    isIncomplete,
    DEFAULT_COLUMNS,
    WORKSHOP_FIELDS,
    type WorkshopField,
  } from "$lib/stores/tags/tagWorkshop.store";
  import { batchStore } from "$lib/stores/ui/batch.store";
  import { writeTagsEach } from "$lib/services/batch/batch.service";
  import { loadTracksByAlbum } from "$lib/services/library/library.service";
  import { libraryContentStore } from "$lib/stores/library/libraryContent.store";
  import type { AuditGroup } from "$lib/services/tags/audit.service";
  import type { TrackListView } from "$lib/types/ui/library/track/TrackListView";
  import { prepareImage } from "$lib/services/tags/tagEditor.service";
  import { open } from "@tauri-apps/plugin-dialog";

  let workshop = $derived($tagWorkshop);
  let libraryId = $derived(Number(page.params.library_id));

  let columns: WorkshopField[] = $state([...DEFAULT_COLUMNS]);
  let showAllColumns = $state(false);
  /** Ne montrer que ce qui manque : le mode de travail réel de l'atelier. */
  let onlyIncomplete = $state(false);
  let launching = $state(false);
  let launchError: string | null = $state(null);

  $effect(() => {
    columns = showAllColumns ? [...WORKSHOP_FIELDS] : [...DEFAULT_COLUMNS];
  });

  /**
   * Album déjà chargé, pour savoir quand recharger.
   *
   * Se fier au fait que le magasin contient des fichiers ne marche pas : en
   * revenant depuis un **autre** album, il en contient toujours — ceux du
   * précédent — et la page restait bloquée dessus.
   */
  let loadedAlbum: string | null = $state(null);

  // Un album passé dans l'adresse rend la page rechargeable et partageable ;
  // une sélection, elle, ne peut venir que du magasin, rempli avant l'arrivée.
  $effect(() => {
    const albumId = page.url.searchParams.get("album");
    if (!albumId || albumId === loadedAlbum) return;
    loadedAlbum = albumId;
    loadAlbum(albumId);
  });

  /**
   * Retient la sortie tant qu'il reste des corrections non écrites.
   *
   * Le bouton de retour posait déjà la question, mais on peut quitter par la
   * barre latérale, un lien d'album ou le retour du navigateur — et cent six
   * corrections disparaissaient alors sans un mot.
   */
  beforeNavigate((navigation) => {
    if (pending.length === 0 || confirmingLeave) return;
    navigation.cancel();
    confirmingLeave = true;
  });

  /**
   * Verse une catégorie de l'inventaire dans l'atelier.
   *
   * Pas d'origine : contrairement à un album, une catégorie n'est pas un
   * endroit où revenir — le bouton retour ramènerait sur une liste qui n'est
   * plus à jour une fois les corrections écrites.
   */
  async function loadFromAudit(group: AuditGroup, label: string) {
    loadedAlbum = null;
    await tagWorkshop.load(group.paths, label);
  }

  async function loadAlbum(albumId: string) {
    const tracks: TrackListView[] = await loadTracksByAlbum(libraryId, albumId);
    const paths = tracks
      .map((track) => track.path)
      .filter((path): path is string => !!path);

    // Le test de réinscriptibilité se fait dans la même passe que la lecture
    // des tags : cent appels séparés coûtaient plus cher que la lecture.
    const label = tracks[0]?.album ?? $t("workshop.title");
    await tagWorkshop.load(paths, label);
  }

  /**
   * Où le bouton de retour ramène.
   *
   * L'album se déduit de l'adresse, ce qui marche même après un rechargement.
   * Le dossier et la sélection ont enregistré leur origine en arrivant. Faute
   * des deux, on retombe sur l'historique du navigateur.
   */
  let backHref = $derived.by(() => {
    const albumId = page.url.searchParams.get("album");
    if (albumId) return `/library/${libraryId}/albums/${albumId}`;
    return workshop.origin;
  });

  /** Vrai quand quitter la page ferait perdre des corrections. */
  let confirmingLeave = $state(false);

  function leave() {
    // Partir en silence avec des modifications en attente les perdrait sans
    // que rien ne l'ait annoncé — le défaut le plus coûteux d'un éditeur.
    if (pending.length > 0 && !confirmingLeave) {
      confirmingLeave = true;
      return;
    }
    tagWorkshop.clear();
    loadedAlbum = null;
    if (backHref) goto(backHref);
    else history.back();
  }

  let coverBusy = $state(false);
  /** Panneau de récupération en ligne, ouvert à la demande. */
  /**
   * Ouvre la récupération Deezer.
   *
   * En popin et non plus en panneau latéral : à 400 px de large, l'écran de
   * revue ne pouvait montrer que la nouvelle valeur, jamais celle qu'elle
   * remplace — ce qui est précisément ce qu'on a besoin de vérifier.
   */
  /**
   * Ouvre l'aperçu de renommage.
   *
   * Sur les lignes cochées, ou sur tout l'atelier si rien ne l'est : renommer
   * un album entier est le cas courant, et exiger un « tout sélectionner »
   * préalable serait une friction gratuite.
   */
  /**
   * Recale l'application après un déplacement de fichiers.
   *
   * Deux choses à reprendre, et les oublier rendait les morceaux introuvables :
   * l'atelier tient ses chemins en mémoire — recharger avec les anciens ne
   * trouve plus rien —, et le cache de la bibliothèque garde les vues d'album
   * telles qu'elles étaient, donc le lecteur y prenait encore l'ancien chemin.
   */
  async function resync(moved: [string, string][]) {
    if (moved.length === 0) return;

    const replacement = new Map(moved);
    const paths = workshop.files.map((f) => replacement.get(f.path) ?? f.path);

    await libraryContentStore.refresh();
    await tagWorkshop.load(paths, workshop.source);
  }

  function openRename() {
    popinStore.open(
      $t('rename.title'),
      RenamePopin,
      { libraryId, onapplied: resync },
      { size: "xl", flush: true, icon: "lucide:file-pen-line" },
    );
  }

  /**
   * Ouvre l'historique des lots.
   *
   * Le bouton « Annuler » du compte rendu ne vit que le temps d'une popin ; on
   * se rend compte d'une erreur en regardant sa bibliothèque, pas dans la
   * seconde qui suit.
   */
  function openHistory() {
    popinStore.open(
      $t('history.title'),
      BatchHistoryPopin,
      { onundone: resync },
      { size: "lg", flush: true, icon: "lucide:history" },
    );
  }

  function openClean() {
    popinStore.open($t('clean.title'), CleanTagsPopin, {}, {
      size: "xl",
      flush: true,
      icon: "lucide:sparkles",
    });
  }

  function openSource() {
    popinStore.open($t('source.title'), TagSourcePopin, {}, {
      size: "xl",
      flush: true,
      icon: "lucide:cloud-download",
    });
  }

  /**
   * Pose une pochette sur les lignes cochées.
   *
   * Elle ne remplace que la pochette avant : livrets et pochettes arrière de
   * chaque fichier restent en place, ce qui compte d'autant plus qu'on ignore
   * ce que contiennent les cent autres.
   */
  async function pickCover() {
    const selected = await open({
      multiple: false,
      filters: [{ name: "Images", extensions: ["jpg", "jpeg", "png", "webp", "gif", "bmp"] }],
    });
    if (typeof selected !== "string") return;

    coverBusy = true;
    launchError = null;
    try {
      const prepared = await prepareImage(selected);
      tagWorkshop.setCoverOnSelected({ path: selected, src: prepared.src });
    } catch (e) {
      launchError = String((e as any)?.message ?? e ?? "");
    } finally {
      coverBusy = false;
    }
  }

  let selectedCount = $derived(workshop.selected.size);

  /**
   * Pochette du jeu de travail, telle qu'on la verra après écriture.
   *
   * Une pochette en attente prime sur celle du fichier : c'est ce qui rend le
   * changement d'ensemble visible avant de l'écrire.
   */
  let commonCover = $derived(
    workshop.files.find((f) => f.pendingCover)?.pendingCover?.src ??
      workshop.files.find((f) => f.cover)?.cover ??
      null,
  );
  let coverPending = $derived(workshop.files.some((f) => f.pendingCover));
  let pending = $derived(workshop.files.filter((f) => f.readable && fileIsDirty(f)));
  let incomplete = $derived(workshop.files.filter((f) => f.readable && isIncomplete(f)));

  /**
   * Écrit les modifications en attente.
   *
   * Chaque fichier porte les siennes : c'est `writeTagsEach` et non le lot
   * partagé. Le suivi part dans le panneau flottant, la page reste utilisable.
   */
  async function write() {
    if (pending.length === 0 || launching) return;
    if (batchStore.isRunning()) {
      launchError = $t("tags.batch_already_running");
      return;
    }

    launching = true;
    launchError = null;
    try {
      const items = tagWorkshop.pendingWrites();
      const written = items.map((item) => item.path);

      const jobId = await writeTagsEach(items);
      batchStore.begin(jobId, $t("batch.writing_tags"));

      // Les modifications deviennent l'état d'origine : sans ça la page
      // proposerait de réécrire ce qu'elle vient d'envoyer.
      tagWorkshop.markWritten(written);
    } catch (e) {
      launchError = String((e as any)?.message ?? e ?? "");
    } finally {
      launching = false;
    }
  }
</script>

<div class="flex flex-col h-full min-h-0">
  <!-- ─────────── En-tête ─────────── -->
  <header
    class="shrink-0 px-5 py-3 border-b border-neutral-200/70 dark:border-white/8
           bg-white/80 dark:bg-neutral-950/80"
  >
    <div class="flex items-center gap-3">
      <button
        type="button"
        onclick={leave}
        title={$t('workshop.back')}
        aria-label={$t('workshop.back')}
        class="shrink-0 w-8 h-8 rounded-lg flex items-center justify-center
               cursor-pointer transition-colors
               text-neutral-400 dark:text-neutral-500
               hover:text-neutral-800 dark:hover:text-neutral-100
               hover:bg-neutral-100 dark:hover:bg-white/8"
      >
        <Icon icon="lucide:arrow-left" width="16" />
      </button>

      <!-- La pochette d'ensemble, cliquable : c'est l'endroit où l'on s'attend
           à la changer pour tout le lot, bien plus qu'un bouton dans une barre
           d'actions. Elle remplace l'icône décorative, qui n'apprenait rien. -->
      <button
        type="button"
        onclick={pickCover}
        disabled={coverBusy || selectedCount === 0}
        title={$t('workshop.set_cover_hint')}
        aria-label={$t('workshop.set_cover')}
        class="group shrink-0 relative w-11 h-11 rounded-lg overflow-hidden
               cursor-pointer transition-all disabled:cursor-not-allowed
               ring-1 {coverPending
                 ? 'ring-emerald-500'
                 : 'ring-black/10 dark:ring-white/10 hover:ring-emerald-500/60'}"
      >
        {#if commonCover}
          <img src={commonCover} alt="" class="w-full h-full object-cover" />
        {:else}
          <span class="w-full h-full flex items-center justify-center
                       bg-neutral-100 dark:bg-white/5">
            <Icon icon="lucide:image-off" width="16"
                  class="text-neutral-300 dark:text-neutral-600" />
          </span>
        {/if}

        {#if selectedCount > 0}
          <span
            class="absolute inset-0 flex items-center justify-center
                   bg-black/60 opacity-0 group-hover:opacity-100 transition-opacity"
          >
            <Icon
              icon={coverBusy ? 'lucide:loader-circle' : 'lucide:image-plus'}
              width="15"
              class="text-white {coverBusy ? 'animate-spin' : ''}"
            />
          </span>
        {/if}
      </button>

      <div class="min-w-0 flex-1">
        <h1 class="text-base font-semibold tracking-tight truncate
                   text-neutral-900 dark:text-white">
          {$t('workshop.title')}
        </h1>
        <p class="text-[11px] truncate text-neutral-500 dark:text-neutral-400">
          {#if workshop.source}{workshop.source} · {/if}
          {workshop.files.length} {$t('tags.files')}
          {#if incomplete.length > 0}
            · <span class="text-amber-500">
                {incomplete.length} {$t('workshop.to_fix')}
              </span>
          {/if}
        </p>
      </div>

      <!-- Bascule de vue -->
      <div class="shrink-0 flex rounded-lg p-0.5 bg-neutral-100 dark:bg-white/6">
        {#each [['grid', 'lucide:table-2'], ['panel', 'lucide:panel-left']] as [mode, icon] (mode)}
          <button
            type="button"
            onclick={() => tagWorkshop.setView(mode as 'grid' | 'panel')}
            title={$t(`workshop.view_${mode}`)}
            aria-label={$t(`workshop.view_${mode}`)}
            class="w-7 h-7 rounded-md flex items-center justify-center
                   cursor-pointer transition-colors
                   {workshop.view === mode
                     ? 'bg-white dark:bg-neutral-800 text-emerald-600 dark:text-emerald-400 shadow-sm'
                     : 'text-neutral-400 dark:text-neutral-500 hover:text-neutral-700 dark:hover:text-neutral-200'}"
          >
            <Icon {icon} width="14" />
          </button>
        {/each}
      </div>
    </div>

    <!-- ─────────── Actions ─────────── -->
    <div class="mt-2.5 flex flex-wrap items-center gap-1.5">
      {#if workshop.view === 'grid'}
        <button
          type="button"
          onclick={() => (showAllColumns = !showAllColumns)}
          class="flex items-center gap-1.5 px-2.5 h-7 rounded-lg text-[11px]
                 cursor-pointer transition-colors
                 text-neutral-500 dark:text-neutral-400
                 hover:bg-neutral-100 dark:hover:bg-white/6"
        >
          <Icon icon={showAllColumns ? 'lucide:minimize-2' : 'lucide:columns-3'} width="12" />
          {showAllColumns ? $t('workshop.fewer_columns') : $t('workshop.more_columns')}
        </button>
      {/if}

      <button
        type="button"
        onclick={() => tagWorkshop.numberSelected()}
        class="flex items-center gap-1.5 px-2.5 h-7 rounded-lg text-[11px]
               cursor-pointer transition-colors
               text-neutral-500 dark:text-neutral-400
               hover:bg-neutral-100 dark:hover:bg-white/6"
      >
        <Icon icon="lucide:list-ordered" width="12" />
        {$t('tags.number_tracks')}
      </button>

      <button
        type="button"
        onclick={openSource}
        disabled={selectedCount === 0}
        class="flex items-center gap-1.5 px-2.5 h-7 rounded-lg text-[11px]
               cursor-pointer transition-colors disabled:opacity-40
               text-neutral-500 dark:text-neutral-400
               hover:bg-neutral-100 dark:hover:bg-white/6"
      >
        <Icon icon="lucide:cloud-download" width="12" />
        {$t('source.short')}
      </button>

      <button
        type="button"
        onclick={openRename}
        class="flex items-center gap-1.5 px-2.5 h-7 rounded-lg text-[11px]
               cursor-pointer transition-colors
               text-neutral-500 dark:text-neutral-400
               hover:bg-neutral-100 dark:hover:bg-white/6"
      >
        <Icon icon="lucide:file-pen-line" width="12" />
        {$t('rename.short')}
      </button>

      <button
        type="button"
        onclick={openClean}
        class="flex items-center gap-1.5 px-2.5 h-7 rounded-lg text-[11px]
               cursor-pointer transition-colors
               text-neutral-500 dark:text-neutral-400
               hover:bg-neutral-100 dark:hover:bg-white/6"
      >
        <Icon icon="lucide:sparkles" width="12" />
        {$t('clean.short')}
      </button>

      <button
        type="button"
        onclick={openHistory}
        title={$t('history.title')}
        aria-label={$t('history.title')}
        class="flex items-center justify-center w-7 h-7 rounded-lg
               cursor-pointer transition-colors
               text-neutral-500 dark:text-neutral-400
               hover:bg-neutral-100 dark:hover:bg-white/6"
      >
        <Icon icon="lucide:history" width="13" />
      </button>

      <button
        type="button"
        onclick={() => (onlyIncomplete = !onlyIncomplete)}
        class="flex items-center gap-1.5 px-2.5 h-7 rounded-lg text-[11px]
               cursor-pointer transition-colors
               {onlyIncomplete
                 ? 'bg-amber-500/15 text-amber-600 dark:text-amber-400'
                 : 'text-neutral-500 dark:text-neutral-400 hover:bg-neutral-100 dark:hover:bg-white/6'}"
      >
        <Icon icon="lucide:filter" width="12" />
        {$t('workshop.only_incomplete')}
      </button>

      <span class="flex-1"></span>

      {#if pending.length > 0}
        <button
          type="button"
          onclick={() => tagWorkshop.revertAll()}
          class="px-2.5 h-7 rounded-lg text-[11px] cursor-pointer transition-colors
                 text-neutral-400 dark:text-neutral-500
                 hover:text-neutral-700 dark:hover:text-neutral-200"
        >
          {$t('tags.reset_all')}
        </button>
      {/if}

      <button
        type="button"
        onclick={write}
        disabled={pending.length === 0 || launching}
        class="flex items-center gap-1.5 px-3 h-7 rounded-lg text-[12px] font-medium
               cursor-pointer transition-all
               bg-emerald-500 text-white
               hover:bg-emerald-600 hover:shadow-lg hover:shadow-emerald-500/40
               disabled:opacity-35 disabled:cursor-not-allowed disabled:shadow-none"
      >
        {#if launching}
          <Icon icon="lucide:loader-circle" width="12" class="animate-spin" />
        {:else}
          <Icon icon="lucide:check" width="12" />
        {/if}
        {$t('workshop.write')}
        {#if pending.length > 0}({pending.length}){/if}
      </button>
    </div>

    <!-- Sortie refusée : on dit ce qu'on s'apprête à perdre, et on laisse le
         choix. Le bouton par défaut est celui qui ne détruit rien. -->
    {#if confirmingLeave}
      <div class="mt-2 flex items-center gap-3 px-3 py-2 rounded-lg
                  bg-amber-500/10 ring-1 ring-amber-500/25">
        <Icon icon="lucide:triangle-alert" width="14" class="shrink-0 text-amber-500" />
        <span class="flex-1 min-w-0 text-[11.5px] text-amber-700 dark:text-amber-300">
          {pending.length}
          {pending.length > 1 ? $t('workshop.pending_files') : $t('workshop.pending_file')}
        </span>
        <button
          type="button"
          onclick={leave}
          class="px-3 h-7 rounded-lg text-[12px] cursor-pointer transition-colors
                 text-amber-700 dark:text-amber-300 hover:bg-amber-500/15"
        >
          {$t('tags.discard')}
        </button>
        <button
          type="button"
          onclick={() => (confirmingLeave = false)}
          class="px-3 h-7 rounded-lg text-[12px] font-medium cursor-pointer transition-colors
                 bg-amber-500 text-white hover:bg-amber-600"
        >
          {$t('tags.keep_editing')}
        </button>
      </div>
    {/if}

    {#if launchError}
      <p class="mt-2 flex items-start gap-1.5 text-[11px] text-red-500">
        <Icon icon="lucide:alert-triangle" width="12" class="shrink-0 mt-0.5" />
        {launchError}
      </p>
    {/if}
  </header>

  <!-- ─────────── Corps ─────────── -->
  {#if workshop.loading}
    <div class="flex-1 flex items-center justify-center">
      <Icon icon="lucide:loader-circle" width="24" class="animate-spin text-emerald-500" />
    </div>
  {:else if workshop.files.length === 0}
    <!-- L'atelier sans sélection affichait « aucun fichier chargé » et un
         conseil : un cul-de-sac, puisqu'il fallait déjà savoir quel album
         ouvrir. L'inventaire prend sa place — il dit ce qui cloche, et un clic
         verse les fichiers concernés ici même. -->
    <TagAuditView {libraryId} onpick={loadFromAudit} />
  {:else}
    <div class="flex-1 min-h-0 flex">
      <div class="flex-1 min-w-0 flex flex-col">
        {#if workshop.view === 'grid'}
          <TagGrid {columns} {onlyIncomplete} />
        {:else}
          <TagPanelView />
        {/if}
      </div>
    </div>
  {/if}
</div>
