<script lang="ts">
  // Éditeur de métadonnées.
  //
  // # Trois principes de fond
  //
  // 1. On mémorise l'état d'origine à l'ouverture et on n'envoie au backend QUE
  //    ce qui a réellement changé. Un champ inchangé est omis, pas envoyé vide —
  //    l'envoyer vide signifierait « efface ». Les images ne sont envoyées que
  //    si la liste a bougé.
  //
  // 2. L'interface le dit. Chaque champ modifié porte une marque et un bouton
  //    de rétablissement, et le pied de page compte les modifications. Sans ça
  //    on enregistre à l'aveugle.
  //
  // 3. Les cinq gestes sur les médias — ajouter, remplacer, supprimer, définir
  //    comme pochette, réordonner — ne sont que des manipulations de la liste
  //    locale. Le backend reçoit l'état visé, pas une suite d'opérations : une
  //    seule écriture du fichier, atomique, et pas d'état intermédiaire.
  //
  // # Mise en page
  // La popin est ouverte en mode `flush` : c'est ce composant qui possède ses
  // zones de défilement. Il en faut deux indépendantes (la colonne des médias
  // et celle des champs) et un pied de page qui ne défile jamais — impossible
  // si la popin garde l'unique conteneur défilant.
  import Icon from "@iconify/svelte";
  import { fade, scale } from "svelte/transition";
  import { open } from "@tauri-apps/plugin-dialog";
  import { popinStore } from "$lib/stores/ui/popin.store";
  import { t } from "$lib/i18n";
  import {
    buildEdit,
    formatBytes,
    pictureTypeKey,
    prepareImage,
    promoteToCover,
    readTrackImages,
    readTrackTags,
    slotsFromImages,
    slotsSignature,
    slotsToPayload,
    writeTrackTags,
    PICTURE_TYPE_COVER,
    PICTURE_TYPE_OTHER,
    type MediaSlot,
  } from "$lib/services/tags/tagEditor.service";

  /* eslint-disable svelte/valid-prop-names-in-kit-pages */
  const {
    path,
    onsaved = () => {},
  }: {
    path: string;
    onsaved?: () => void;
  } = $props();

  const FIELDS = [
    "title", "artist", "album", "album_artist",
    "year", "genre", "composer", "comment",
    "track_number", "total_tracks", "disc_number", "total_discs",
  ] as const;

  /** Normalise une valeur du backend en chaîne éditable. */
  const str = (v: unknown): string =>
    v === null || v === undefined ? "" : String(v);

  // Les valeurs sont lues DANS LE FICHIER, pas dans la vue de la bibliothèque :
  // celle-ci est une projection de la base qui ignore commentaire, compositeur
  // et totaux. Les afficher vides puis les réenregistrer les effacerait.
  let original: Record<string, string> = $state({});
  let form = $state<Record<string, string>>({});
  /** Liste des médias en cours d'édition, et son état d'origine. */
  let slots: MediaSlot[] = $state([]);
  let originalSlots: MediaSlot[] = $state([]);
  /** Média affiché en grand par-dessus la popin (`null` = aucun). */
  let zoomed: MediaSlot | null = $state(null);
  let isLoading = $state(true);
  let isSubmitting = $state(false);
  /** Vrai pendant le choix et la préparation d'une image. */
  let mediaBusy = $state(false);
  let error: string | null = $state(null);
  /** Vrai quand une fermeture a été refusée faute d'enregistrement. */
  let confirmingClose = $state(false);
  /** Conteneur des champs, pour y poser le focus dès qu'ils existent. */
  let fieldsPane: HTMLElement | null = $state(null);
  /** Compteur de clés pour les images ajoutées, qui n'ont pas d'identifiant. */
  let addedCount = 0;

  // Ouvrir un éditeur et devoir cliquer dans le premier champ avant de pouvoir
  // taper est une friction gratuite. On attend la fin du chargement : avant, les
  // champs sont désactivés et ne prendraient pas le focus.
  $effect(() => {
    if (isLoading || !fieldsPane) return;
    fieldsPane.querySelector("input")?.focus();
  });

  // Fermer en perdant sa saisie sans le moindre avertissement est le défaut le
  // plus coûteux d'un éditeur : un Échap malheureux ou un clic à côté annulent
  // dix corrections. On pose donc un veto tant qu'il reste des modifications.
  //
  // `dirty` n'est lu qu'à l'appel du veto, pas pendant l'effet : celui-ci ne
  // s'exécute donc qu'une fois, et sa fonction de retour désinscrit le veto.
  $effect(() =>
    popinStore.guard(() => {
      if (!dirty || isSubmitting) return true;
      confirmingClose = true;
      return false;
    }),
  );

  $effect(() => {
    const p = path;
    isLoading = true;
    readTrackTags(p)
      .then((tags) => {
        const snapshot: Record<string, string> = {};
        for (const key of FIELDS) snapshot[key] = str((tags as any)[key]);
        original = snapshot;
        form = { ...snapshot };
      })
      .catch((e) => {
        error = String((e as any)?.message ?? e ?? "");
      })
      .finally(() => (isLoading = false));

    // Les images intégrées portent déjà la pochette : inutile d'aller la
    // chercher ailleurs, ce serait la même donnée lue deux fois. Leur absence
    // ne doit pas empêcher l'édition du texte.
    readTrackImages(p)
      .then((list) => {
        originalSlots = slotsFromImages(list);
        slots = originalSlots.map((s) => ({ ...s }));
      })
      .catch(() => {
        originalSlots = [];
        slots = [];
      });
  });

  // La pochette est celle **typée** comme telle, pas la première de la liste.
  // À défaut de type, la première fait office d'aperçu.
  let coverSlot = $derived(
    slots.find((s) => s.picture_type === PICTURE_TYPE_COVER) ?? slots[0] ?? null,
  );
  let mediaBytes = $derived(slots.reduce((sum, s) => sum + s.bytes, 0));
  let mediaDirty = $derived(
    slotsSignature(slots) !== slotsSignature(originalSlots),
  );

  let changed = $derived(
    FIELDS.filter((k) => (form[k] ?? "") !== (original[k] ?? "")),
  );
  let dirty = $derived(changed.length > 0 || mediaDirty);

  const isDirty = (key: string) => (form[key] ?? "") !== (original[key] ?? "");

  function revert(key: string) {
    form[key] = original[key] ?? "";
  }

  function revertAll() {
    form = { ...original };
    slots = originalSlots.map((s) => ({ ...s }));
  }

  // ─── Gestes sur les médias ───

  /**
   * Ouvre le sélecteur puis prépare l'image choisie.
   *
   * La préparation a lieu ici, à l'ajout, et jamais lors d'une simple édition
   * de texte : une image conservée est recopiée telle quelle, sinon chaque
   * correction de titre la réencoderait et l'abîmerait un peu plus.
   */
  async function pickImage(): Promise<MediaSlot | null> {
    const selected = await open({
      multiple: false,
      filters: [
        { name: "Images", extensions: ["jpg", "jpeg", "png", "webp", "gif", "bmp"] },
      ],
    });
    if (typeof selected !== "string") return null;

    mediaBusy = true;
    error = null;
    try {
      const prepared = await prepareImage(selected);
      addedCount += 1;
      return {
        key: `nouveau-${addedCount}`,
        id: null,
        path: selected,
        src: prepared.src,
        picture_type: PICTURE_TYPE_OTHER,
        description: "",
        mime_type: prepared.mime_type,
        bytes: prepared.bytes,
        originalBytes: prepared.original_bytes,
        recompressed: prepared.recompressed,
      };
    } catch (e) {
      error = String((e as any)?.message ?? e ?? "");
      return null;
    } finally {
      mediaBusy = false;
    }
  }

  async function addImage() {
    const slot = await pickImage();
    if (!slot) return;
    // Première image d'un fichier qui n'en avait aucune : c'est une pochette
    // dans l'immense majorité des cas, autant l'épargner à l'utilisateur.
    if (slots.length === 0) slot.picture_type = PICTURE_TYPE_COVER;
    slots = [...slots, slot];
  }

  async function replaceImage(index: number) {
    const picked = await pickImage();
    if (!picked) return;
    const current = slots[index];
    // Le rôle et la description appartiennent à l'emplacement, pas au fichier
    // qu'on y dépose : remplacer la pochette doit donner une pochette.
    slots = slots.map((s, i) =>
      i === index
        ? { ...picked, picture_type: current.picture_type, description: current.description }
        : s,
    );
  }

  function removeImage(index: number) {
    slots = slots.filter((_, i) => i !== index);
  }

  function setCover(index: number) {
    slots = promoteToCover(slots, index);
  }

  function move(index: number, delta: number) {
    const target = index + delta;
    if (target < 0 || target >= slots.length) return;
    const next = [...slots];
    [next[index], next[target]] = [next[target], next[index]];
    slots = next;
  }

  /** Libellé d'un type d'image, ou repli numérique pour les types rares. */
  function typeLabel(type: number): string {
    const key = pictureTypeKey(type);
    return key ? $t(key) : `${$t('tags.pic.other')} (${type})`;
  }

  // La popin est instanciée pour un fichier donné et n'en change pas, mais on
  // dérive tout de même : c'est ce que Svelte attend d'une valeur issue d'une
  // prop, et ça reste juste si la popin devient réutilisable.
  let filename = $derived(path.split(/[\\/]/).pop() ?? path);
  let extension = $derived((filename.split(".").pop() ?? "").toUpperCase());

  async function submit() {
    if (!dirty || isSubmitting) return;
    error = null;
    isSubmitting = true;
    try {
      const edit = buildEdit(original, form);
      // Champ absent = « ne touche pas aux images ». On ne l'ajoute donc que
      // si la liste a réellement bougé : sans ça, corriger un titre ferait
      // réécrire toutes les pochettes sans raison.
      if (mediaDirty) edit.images = slotsToPayload(slots);

      await writeTrackTags(path, edit);
      onsaved();
      popinStore.close();
    } catch (e) {
      // Le backend renvoie un message explicite (format non géré, fichier
      // verrouillé, image illisible…). On le montre tel quel : il est plus
      // utile qu'un « une erreur est survenue » générique.
      error = String((e as any)?.message ?? e ?? "");
    } finally {
      isSubmitting = false;
    }
  }

  // Écouté en phase de **capture** : la popin écoute Échap sur `window` elle
  // aussi, et son écouteur, posé en premier, gagnerait la course. En capture
  // on passe avant, ce qui permet d'arrêter l'événement quand une couche
  // au-dessus (la loupe, la confirmation) doit se fermer d'abord.
  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Escape" && zoomed) {
      e.stopPropagation();
      zoomed = null;
      return;
    }
    if (e.key === "Escape" && confirmingClose) {
      e.stopPropagation();
      confirmingClose = false;
      return;
    }
    if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) submit();
  }
</script>

<svelte:window onkeydowncapture={onKeydown} />

<!-- ══ Champ ══
     Un bloc à fond plein, **sans trait de contour** : c'est le fond qui délimite
     le champ. Les traits, eux, s'accumulent — douze cadres empilés donnent une
     grille de formulaire administratif. Et les blocs sont espacés : collés les
     uns aux autres ils formaient une dalle grise illisible.

     Le focus n'ajoute jamais de rectangle : fond teinté, intitulé coloré, halo
     diffus. `data-focus-ring` déplace vers ce bloc l'indicateur de contraste
     élevé, qui sinon dessinerait un cadre dur autour de la seule saisie
     (voir `app.css`).

     Deux signaux distincts : le **focus** allume le bloc, la **modification**
     pose une barre au bord gauche et sort le bouton de rétablissement. -->
{#snippet cell(key: string, labelKey: string, large: boolean)}
  <div
    data-focus-ring="row"
    class="group relative rounded-lg overflow-hidden transition-all duration-150
           bg-neutral-100/70 dark:bg-white/4
           hover:bg-neutral-200/50 dark:hover:bg-white/6
           focus-within:bg-emerald-500/10
           focus-within:shadow-lg focus-within:shadow-emerald-500/25"
  >
    {#if isDirty(key)}
      <span
        class="absolute left-0 inset-y-0 w-0.75 bg-emerald-500"
        transition:fade={{ duration: 120 }}
      ></span>
    {/if}

    <label class="block px-3.5 py-2 cursor-text">
      <span
        class="block text-[10px] font-semibold uppercase tracking-[0.09em] leading-none
               text-neutral-400 dark:text-neutral-500
               group-focus-within:text-emerald-600 dark:group-focus-within:text-emerald-400
               transition-colors"
      >
        {$t(labelKey)}
      </span>
      <input
        type="text"
        data-focus-ring="none"
        bind:value={form[key]}
        disabled={isSubmitting || isLoading}
        placeholder="—"
        class="w-full mt-1.5 bg-transparent leading-tight outline-none
               text-neutral-900 dark:text-neutral-50
               placeholder:text-neutral-300 dark:placeholder:text-neutral-700
               disabled:opacity-50
               {large ? 'text-[15px] font-medium' : 'text-[13px]'}
               {isDirty(key) ? 'pr-8' : ''}"
      />
    </label>

    {#if isDirty(key)}
      <button
        type="button"
        onclick={() => revert(key)}
        title="{$t('tags.revert')} : {original[key] || '—'}"
        aria-label={$t('tags.revert')}
        class="absolute right-2 top-1/2 -translate-y-1/2 w-6 h-6 rounded-md cursor-pointer
               flex items-center justify-center
               text-neutral-400 dark:text-neutral-500
               hover:text-emerald-600 dark:hover:text-emerald-400
               hover:bg-emerald-500/15 transition-colors"
      >
        <Icon icon="lucide:rotate-ccw" width="12" />
      </button>
    {/if}
  </div>
{/snippet}

<!-- ══ Cellule « n sur N » ══
     Numéro et total ne sont pas deux tags mais un seul, écrit « 7/12 ». Deux
     cases séparées mentiraient sur le format et prendraient deux fois la
     place ; la barre oblique le dit visuellement. -->
{#snippet pairCell(numKey: string, totalKey: string, labelKey: string)}
  <div
    data-focus-ring="row"
    class="group relative rounded-lg overflow-hidden transition-all duration-150
           bg-neutral-100/70 dark:bg-white/4
           hover:bg-neutral-200/50 dark:hover:bg-white/6
           focus-within:bg-emerald-500/10
           focus-within:shadow-lg focus-within:shadow-emerald-500/25"
  >
    {#if isDirty(numKey) || isDirty(totalKey)}
      <span
        class="absolute left-0 inset-y-0 w-0.75 bg-emerald-500"
        transition:fade={{ duration: 120 }}
      ></span>
    {/if}

    <div class="px-3.5 py-2">
      <span
        class="block text-[10px] font-semibold uppercase tracking-[0.09em] leading-none
               text-neutral-400 dark:text-neutral-500
               group-focus-within:text-emerald-600 dark:group-focus-within:text-emerald-400
               transition-colors"
      >
        {$t(labelKey)}
      </span>
      <div class="flex items-baseline gap-1.5 mt-1.5">
        <input
          type="text"
          inputmode="numeric"
          data-focus-ring="none"
          bind:value={form[numKey]}
          disabled={isSubmitting || isLoading}
          placeholder="—"
          class="w-7 bg-transparent text-[13px] tabular-nums leading-tight text-right outline-none
                 text-neutral-900 dark:text-neutral-50
                 placeholder:text-neutral-300 dark:placeholder:text-neutral-700
                 disabled:opacity-50"
        />
        <span class="text-neutral-300 dark:text-neutral-600 select-none text-[13px]">/</span>
        <input
          type="text"
          inputmode="numeric"
          data-focus-ring="none"
          bind:value={form[totalKey]}
          disabled={isSubmitting || isLoading}
          placeholder="—"
          class="w-7 bg-transparent text-[13px] tabular-nums leading-tight outline-none
                 text-neutral-500 dark:text-neutral-400
                 placeholder:text-neutral-300 dark:placeholder:text-neutral-700
                 disabled:opacity-50"
        />
      </div>
    </div>
  </div>
{/snippet}

<!-- ══ Commentaire ══
     Un commentaire tient rarement sur une ligne : une case simple obligerait à
     faire défiler le texte horizontalement pour se relire. -->
{#snippet areaCell(key: string, labelKey: string)}
  <div
    data-focus-ring="row"
    class="group relative rounded-lg overflow-hidden transition-all duration-150
           bg-neutral-100/70 dark:bg-white/4
           hover:bg-neutral-200/50 dark:hover:bg-white/6
           focus-within:bg-emerald-500/10
           focus-within:shadow-lg focus-within:shadow-emerald-500/25"
  >
    {#if isDirty(key)}
      <span
        class="absolute left-0 inset-y-0 w-0.75 bg-emerald-500"
        transition:fade={{ duration: 120 }}
      ></span>
    {/if}

    <label class="block px-3.5 py-2 cursor-text">
      <span
        class="block text-[10px] font-semibold uppercase tracking-[0.09em] leading-none
               text-neutral-400 dark:text-neutral-500
               group-focus-within:text-emerald-600 dark:group-focus-within:text-emerald-400
               transition-colors"
      >
        {$t(labelKey)}
      </span>
      <textarea
        rows="2"
        data-focus-ring="none"
        bind:value={form[key]}
        disabled={isSubmitting || isLoading}
        placeholder="—"
        class="w-full mt-1.5 bg-transparent text-[13px] leading-snug resize-none scrollbar-none
               outline-none
               text-neutral-900 dark:text-neutral-50
               placeholder:text-neutral-300 dark:placeholder:text-neutral-700
               disabled:opacity-50"
      ></textarea>
    </label>

    {#if isDirty(key)}
      <button
        type="button"
        onclick={() => revert(key)}
        title={$t('tags.revert')}
        aria-label={$t('tags.revert')}
        class="absolute right-1.5 top-1.5 w-6 h-6 rounded-md cursor-pointer
               flex items-center justify-center
               text-neutral-400 dark:text-neutral-500
               hover:text-emerald-600 dark:hover:text-emerald-400
               hover:bg-emerald-500/15 transition-colors"
      >
        <Icon icon="lucide:rotate-ccw" width="12" />
      </button>
    {/if}
  </div>
{/snippet}

{#snippet groupTitle(labelKey: string)}
  <div class="flex items-center gap-2.5 mb-2">
    <span class="text-[10px] font-semibold uppercase tracking-[0.14em]
                 text-neutral-400 dark:text-neutral-500">
      {$t(labelKey)}
    </span>
    <span class="flex-1 h-px bg-linear-to-r
                 from-neutral-200 to-transparent
                 dark:from-white/10 dark:to-transparent"></span>
  </div>
{/snippet}

<!-- Bouton d'action d'une ligne de média. Petit et discret par défaut, il ne
     se colore qu'au survol : cinq boutons colorés par ligne noieraient la
     vignette, qui est ce qu'on regarde. -->
{#snippet rowAction(
  icon: string,
  label: string,
  action: () => void,
  disabled: boolean,
  danger: boolean,
)}
  <button
    type="button"
    onclick={action}
    {disabled}
    title={label}
    aria-label={label}
    class="w-6 h-6 shrink-0 rounded-md flex items-center justify-center
           cursor-pointer transition-colors
           text-neutral-400 dark:text-neutral-500
           disabled:opacity-25 disabled:cursor-not-allowed
           {danger
             ? 'hover:text-red-500 hover:bg-red-500/12'
             : 'hover:text-emerald-600 dark:hover:text-emerald-400 hover:bg-emerald-500/12'}"
  >
    <Icon {icon} width="12" />
  </button>
{/snippet}

<div class="flex-1 min-h-0 flex flex-col">
  <div class="flex-1 min-h-0 flex">
    <!-- ─────────── Colonne des médias ─────────── -->
    <aside
      class="shrink-0 w-64 flex flex-col gap-3.5 p-4 overflow-y-auto scrollbar-none
             border-r border-neutral-200/70 dark:border-white/8
             bg-neutral-50/70 dark:bg-black/20"
    >
      <div
        class="relative group aspect-square rounded-xl overflow-hidden shrink-0
               ring-1 ring-black/5 dark:ring-white/10
               shadow-lg shadow-black/10 dark:shadow-black/50
               bg-neutral-200/60 dark:bg-white/5"
      >
        {#if coverSlot}
          <img src={coverSlot.src} alt="" class="w-full h-full object-cover" />
          <button
            type="button"
            onclick={() => (zoomed = coverSlot)}
            aria-label={$t('tags.zoom')}
            class="absolute inset-0 flex items-end justify-center pb-3 cursor-zoom-in
                   opacity-0 group-hover:opacity-100 transition-opacity duration-150
                   bg-linear-to-t from-black/75 via-black/10 to-transparent"
          >
            <!-- Fond opaque plutôt que translucide : le mode contraste élevé
                 neutralise tous les `backdrop-filter`, et une pastille qui ne
                 tenait que par son flou y devient illisible. -->
            <span
              class="flex items-center gap-1.5 px-2.5 h-7 rounded-lg text-[11px] font-medium
                     bg-black/75 text-white ring-1 ring-white/25"
            >
              <Icon icon="lucide:maximize-2" width="11" />
              {$t('tags.zoom')}
            </span>
          </button>
          <!-- Après le survol dans l'ordre du DOM pour rester au-dessus, et
               transparent aux clics pour ne pas trouer la zone d'agrandissement. -->
          {#if coverSlot.picture_type === PICTURE_TYPE_COVER}
            <span
              class="absolute top-2 left-2 pointer-events-none
                     flex items-center gap-1 px-1.5 py-0.5 rounded-md
                     text-[10px] font-bold uppercase tracking-wider
                     bg-emerald-500 text-white shadow-md shadow-black/30"
            >
              <Icon icon="lucide:star" width="9" />
              {$t('tags.cover')}
            </span>
          {/if}
        {:else}
          <button
            type="button"
            onclick={addImage}
            disabled={mediaBusy || isSubmitting}
            class="w-full h-full flex flex-col items-center justify-center gap-2
                   cursor-pointer transition-colors
                   text-neutral-400 dark:text-neutral-500
                   hover:text-emerald-600 dark:hover:text-emerald-400
                   hover:bg-emerald-500/8 disabled:opacity-50"
          >
            <Icon icon="lucide:image-plus" width="26" />
            <span class="text-[11px] font-medium">{$t('tags.add_image')}</span>
          </button>
        {/if}
      </div>

      <!-- Liste verticale plutôt qu'une grille de vignettes : elle nomme chaque
           image, donne son poids, et laisse la place aux cinq actions. -->
      <div class="shrink-0">
        <div class="mb-1.5 flex items-center gap-1.5">
          <span class="text-[10px] font-semibold uppercase tracking-[0.09em]
                       text-neutral-400 dark:text-neutral-500">
            {$t('tags.media')}
          </span>
          <span class="px-1 rounded text-[10px] tabular-nums
                       bg-neutral-200/70 dark:bg-white/10
                       text-neutral-500 dark:text-neutral-400">
            {slots.length}
          </span>
          <span class="flex-1"></span>
          {#if slots.length > 0}
            <button
              type="button"
              onclick={addImage}
              disabled={mediaBusy || isSubmitting}
              title={$t('tags.add_image')}
              aria-label={$t('tags.add_image')}
              class="w-6 h-6 rounded-md flex items-center justify-center
                     cursor-pointer transition-colors
                     text-neutral-400 dark:text-neutral-500
                     hover:text-emerald-600 dark:hover:text-emerald-400
                     hover:bg-emerald-500/12 disabled:opacity-30"
            >
              <Icon icon={mediaBusy ? 'lucide:loader-circle' : 'lucide:plus'}
                    width="13" class={mediaBusy ? 'animate-spin' : ''} />
            </button>
          {/if}
        </div>

        {#if slots.length === 0}
          <p class="text-[11px] text-neutral-400 dark:text-neutral-500">
            {$t('tags.no_media')}
          </p>
        {:else}
          <div class="space-y-1">
            {#each slots as slot, i (slot.key)}
              <div
                class="flex gap-2 p-1.5 rounded-lg transition-colors
                       hover:bg-neutral-200/50 dark:hover:bg-white/5
                       {slot.picture_type === PICTURE_TYPE_COVER
                         ? 'bg-emerald-500/10'
                         : ''}"
              >
                <button
                  type="button"
                  onclick={() => (zoomed = slot)}
                  title={$t('tags.zoom')}
                  class="w-11 h-11 shrink-0 rounded overflow-hidden cursor-zoom-in ring-1
                         {slot.picture_type === PICTURE_TYPE_COVER
                           ? 'ring-emerald-500/60'
                           : 'ring-black/10 dark:ring-white/10'}"
                >
                  <img src={slot.src} alt="" class="w-full h-full object-cover" />
                </button>

                <div class="min-w-0 flex-1">
                  <p
                    class="text-[11px] truncate
                           {slot.picture_type === PICTURE_TYPE_COVER
                             ? 'font-medium text-emerald-600 dark:text-emerald-400'
                             : 'text-neutral-600 dark:text-neutral-300'}"
                  >
                    {typeLabel(slot.picture_type)}
                  </p>
                  <p class="text-[10px] truncate text-neutral-400 dark:text-neutral-500">
                    {slot.mime_type.replace('image/', '').toUpperCase()} · {formatBytes(slot.bytes)}
                    {#if slot.recompressed && slot.originalBytes}
                      <span title="{formatBytes(slot.originalBytes)} → {formatBytes(slot.bytes)}"
                            class="text-amber-500">· {$t('tags.reduced')}</span>
                    {/if}
                  </p>

                  <div class="flex items-center gap-0.5 mt-0.5 -ml-1">
                    {@render rowAction(
                      'lucide:star',
                      $t('tags.set_cover'),
                      () => setCover(i),
                      slot.picture_type === PICTURE_TYPE_COVER || isSubmitting,
                      false,
                    )}
                    {@render rowAction(
                      'lucide:arrow-up',
                      $t('tags.move_up'),
                      () => move(i, -1),
                      i === 0 || isSubmitting,
                      false,
                    )}
                    {@render rowAction(
                      'lucide:arrow-down',
                      $t('tags.move_down'),
                      () => move(i, 1),
                      i === slots.length - 1 || isSubmitting,
                      false,
                    )}
                    {@render rowAction(
                      'lucide:refresh-cw',
                      $t('tags.replace_image'),
                      () => replaceImage(i),
                      mediaBusy || isSubmitting,
                      false,
                    )}
                    {@render rowAction(
                      'lucide:trash-2',
                      $t('tags.remove_image'),
                      () => removeImage(i),
                      isSubmitting,
                      true,
                    )}
                  </div>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </div>

      <!-- Identité du fichier, juste sous les médias : collée en bas de la
           colonne elle flottait toute seule au milieu du vide. -->
      <div class="pt-3 shrink-0 border-t border-neutral-200/70 dark:border-white/8">
        <p
          class="text-[11px] font-medium text-neutral-600 dark:text-neutral-300 break-all line-clamp-2"
          title={path}
        >
          {filename}
        </p>
        <div class="mt-1.5 flex flex-wrap gap-1">
          {#if extension}
            <span class="px-1.5 py-0.5 rounded text-[10px] font-semibold tracking-wide
                         bg-neutral-200/70 dark:bg-white/10
                         text-neutral-500 dark:text-neutral-400">
              {extension}
            </span>
          {/if}
          <!-- Ce poids est celui des images intégrées, pas du fichier : sans
               l'icône il se lisait comme la taille du MP3, ce qui est faux. -->
          {#if mediaBytes > 0}
            <span
              title="{$t('tags.media')} · {formatBytes(mediaBytes)}"
              class="flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-medium
                     bg-neutral-200/70 dark:bg-white/10
                     text-neutral-500 dark:text-neutral-400"
            >
              <Icon icon="lucide:image" width="9" />
              {formatBytes(mediaBytes)}
            </span>
          {/if}
        </div>
      </div>
    </aside>

    <!-- ─────────── Colonne des champs ─────────── -->
    <div
      bind:this={fieldsPane}
      class="flex-1 min-w-0 overflow-y-auto scrollbar-none px-5 py-4 space-y-5"
    >
      {#if isLoading}
        <!-- Le squelette reprend la forme et l'écart des blocs : la mise en
             page ne bouge pas quand les valeurs arrivent. -->
        <div class="space-y-5" aria-hidden="true">
          {#each [2, 4, 2] as rows, group (group)}
            <div class="space-y-2">
              <div class="h-2 w-20 rounded bg-neutral-200/80 dark:bg-white/8 animate-pulse"></div>
              <div class="space-y-1.5">
                {#each Array.from({ length: rows }) as _, i (i)}
                  <div class="h-12.5 rounded-lg animate-pulse
                              bg-neutral-100/70 dark:bg-white/4"></div>
                {/each}
              </div>
            </div>
          {/each}
        </div>
      {:else}
        <!-- Blocs espacés : c'est le titre de section et l'écart qui groupent,
             pas un cadre autour. -->
        <section>
          {@render groupTitle('tags.section_track')}
          <div class="space-y-1.5">
            {@render cell('title', 'tags.title', true)}
            {@render cell('artist', 'tags.artist', false)}
          </div>
        </section>

        <section>
          {@render groupTitle('tags.section_album')}
          <div class="space-y-1.5">
            {@render cell('album', 'tags.album', false)}
            {@render cell('album_artist', 'tags.album_artist', false)}
            <!-- L'année est courte et de longueur fixe : lui donner la même
                 largeur qu'un genre gaspillerait la ligne. -->
            <div class="grid grid-cols-3 gap-1.5">
              {@render cell('year', 'tags.year', false)}
              <div class="col-span-2">
                {@render cell('genre', 'tags.genre', false)}
              </div>
            </div>
            <div class="grid grid-cols-2 gap-1.5">
              {@render pairCell('track_number', 'total_tracks', 'tags.track_short')}
              {@render pairCell('disc_number', 'total_discs', 'tags.disc_short')}
            </div>
          </div>
        </section>

        <section>
          {@render groupTitle('tags.section_extra')}
          <div class="space-y-1.5">
            {@render cell('composer', 'tags.composer', false)}
            {@render areaCell('comment', 'tags.comment')}
          </div>
        </section>
      {/if}
    </div>
  </div>

  <!-- Le message d'erreur a sa propre bande : dans le pied de page il
       repousserait les boutons hors de portée dès qu'il est un peu long. -->
  {#if error}
    <div
      class="shrink-0 flex items-start gap-2 px-5 py-2
             bg-red-500/10 border-t border-red-500/20 text-red-500 text-[11px]"
      transition:fade={{ duration: 120 }}
    >
      <Icon icon="lucide:alert-triangle" width="13" class="shrink-0 mt-0.5" />
      <span class="min-w-0">{error}</span>
    </div>
  {/if}

  <!-- Fermeture refusée : on annonce ce qu'on s'apprête à perdre et on laisse
       le choix, plutôt que de fermer en silence. Le bouton par défaut est
       celui qui ne détruit rien. -->
  {#if confirmingClose && dirty}
    <div
      class="shrink-0 flex items-center gap-3 px-4 py-2.5
             bg-amber-500/10 border-t border-amber-500/25"
      transition:fade={{ duration: 120 }}
    >
      <Icon icon="lucide:triangle-alert" width="14" class="shrink-0 text-amber-500" />
      <span class="flex-1 min-w-0 text-[11.5px] text-amber-700 dark:text-amber-300">
        {$t('tags.unsaved')}
      </span>
      <button
        type="button"
        onclick={() => popinStore.close()}
        class="px-3 h-7 rounded-lg text-[12px] cursor-pointer transition-colors
               text-amber-700 dark:text-amber-300
               hover:bg-amber-500/15"
      >
        {$t('tags.discard')}
      </button>
      <button
        type="button"
        onclick={() => (confirmingClose = false)}
        class="px-3 h-7 rounded-lg text-[12px] font-medium cursor-pointer transition-colors
               bg-amber-500 text-white hover:bg-amber-600"
      >
        {$t('tags.keep_editing')}
      </button>
    </div>
  {/if}

  <!-- ─────────── Pied de page ─────────── -->
  <footer
    class="shrink-0 flex items-center gap-3 px-4 py-2.5
           border-t border-neutral-200/70 dark:border-white/8
           bg-neutral-50/80 dark:bg-black/25"
  >
    <div class="flex-1 min-w-0 flex items-center gap-2">
      {#if dirty}
        <span class="flex items-center gap-1.5 text-[11px] text-neutral-600 dark:text-neutral-300">
          <span class="w-1.5 h-1.5 rounded-full bg-emerald-500"></span>
          {#if changed.length}
            {changed.length}
            {changed.length > 1 ? $t('tags.changed_other') : $t('tags.changed_one')}
          {/if}
          {#if changed.length && mediaDirty}·{/if}
          {#if mediaDirty}{$t('tags.media_changed')}{/if}
        </span>
        <button
          type="button"
          onclick={revertAll}
          disabled={isSubmitting}
          class="text-[11px] cursor-pointer underline decoration-dotted underline-offset-2
                 text-neutral-400 dark:text-neutral-500
                 hover:text-neutral-700 dark:hover:text-neutral-200
                 disabled:opacity-40 transition-colors"
        >
          {$t('tags.reset_all')}
        </button>
      {:else}
        <span class="text-[11px] text-neutral-400 dark:text-neutral-500">
          {$t('tags.no_change')}
        </span>
      {/if}

      <!-- La règle « vider = effacer » est essentielle mais se lit une fois :
           en info-bulle elle reste accessible sans occuper la place. -->
      <span
        title={$t('tags.clear_hint')}
        class="text-neutral-300 dark:text-neutral-600 cursor-help
               hover:text-neutral-500 dark:hover:text-neutral-400 transition-colors"
      >
        <Icon icon="lucide:info" width="12" />
      </span>
    </div>

    <kbd
      class="hidden sm:block px-1.5 py-0.5 rounded text-[9.5px] font-medium tabular-nums
             bg-neutral-200/60 dark:bg-white/8
             text-neutral-400 dark:text-neutral-500
             ring-1 ring-inset ring-neutral-300/50 dark:ring-white/8"
    >
      Ctrl ↵
    </kbd>

    <button
      type="button"
      onclick={() => popinStore.requestClose()}
      disabled={isSubmitting}
      class="flex items-center gap-1.5 px-3 h-8 rounded-lg text-[13px]
             cursor-pointer transition-colors
             text-neutral-600 dark:text-neutral-300
             hover:bg-neutral-200/70 dark:hover:bg-white/8 disabled:opacity-50"
    >
      <Icon icon="lucide:x" width="13" />
      {$t('profil.cancel')}
    </button>
    <button
      type="button"
      onclick={submit}
      disabled={!dirty || isSubmitting}
      class="flex items-center gap-1.5 px-3.5 h-8 rounded-lg text-[13px] font-medium
             cursor-pointer transition-all
             bg-emerald-500 text-white
             hover:bg-emerald-600 hover:shadow-lg hover:shadow-emerald-500/40
             disabled:opacity-35 disabled:cursor-not-allowed disabled:shadow-none"
    >
      {#if isSubmitting}
        <Icon icon="lucide:loader-circle" width="13" class="animate-spin" />
      {:else}
        <Icon icon="lucide:check" width="13" />
      {/if}
      {$t('profil.save')}
    </button>
  </footer>
</div>

<!-- ─────────── Loupe ───────────
     Superposée à la popin (z-index supérieur) : on consulte une image en grand
     sans perdre la saisie en cours. -->
{#if zoomed}
  <div
    role="presentation"
    class="fixed inset-0 z-60 flex flex-col items-center justify-center gap-3 p-10
           bg-black/88 backdrop-blur-md cursor-zoom-out"
    transition:fade={{ duration: 120 }}
    onclick={() => (zoomed = null)}
  >
    <img
      src={zoomed.src}
      alt={typeLabel(zoomed.picture_type)}
      class="max-w-full max-h-[72vh] object-contain rounded-xl shadow-2xl ring-1 ring-white/10"
      transition:scale={{ duration: 140, start: 0.94 }}
    />
    <div class="text-center">
      <p class="text-sm font-medium text-white/90">{typeLabel(zoomed.picture_type)}</p>
      <p class="text-[11px] text-white/50">
        {zoomed.mime_type} · {formatBytes(zoomed.bytes)}
        {#if zoomed.description}· {zoomed.description}{/if}
      </p>
    </div>
  </div>
{/if}
