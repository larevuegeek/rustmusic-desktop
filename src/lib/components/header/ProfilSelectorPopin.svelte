<script lang="ts">
  import { profilSelector } from "$lib/stores/profil/profil.store";
  import { profilPopinStore } from "$lib/stores/profil/profilPopin.store";
  import { libraryStore } from "$lib/stores/library/library.store";
  import { queueState } from "$lib/stores/queue/queueState.store";
  import { liked } from "$lib/stores/playlist/like.store";
  import { Modal, Avatar, Badge } from "@karbonjs/ui-svelte";

  const PALETTE = [
    '#22c55e',
    '#8b5cf6',
    '#0ea5e9',
    '#f59e0b',
    '#f43f5e',
    '#6366f1',
    '#84cc16',
    '#f97316',
    '#ec4899',
    '#14b8a6',
  ];

  function findPaletteIndex(hex: string): number {
    const idx = PALETTE.indexOf(hex);
    return idx >= 0 ? idx : 0;
  }

  let mode: 'select' | 'create' | 'edit' = $state('select');
  let name = $state("");
  let editId: number | null = $state(null);
  let editIndex: number = $state(0);
  let isSubmitting = $state(false);
  let errorName: string | null = $state(null);
  let selectedColorIndex = $state(0);
  let showDeleteConfirm = $state(false);
  let deleteConfirmText = $state("");

  async function reloadAfterSwitch() {
    libraryStore.clear();
    await libraryStore.init();
    await queueState.init();
    await liked.refresh();
  }

  function switchToCreate() {
    mode = 'create';
    name = "";
    editId = null;
    errorName = null;
  }

  function switchToEdit(profil: { id: number; name: string; color: string }) {
    mode = 'edit';
    name = profil.name;
    editId = profil.id;
    editIndex = findPaletteIndex(profil.color);
    if (editIndex < 0) editIndex = 0;
    errorName = null;
  }

  function back() {
    mode = 'select';
    name = "";
    editId = null;
    errorName = null;
    showDeleteConfirm = false;
    deleteConfirmText = "";
  }

  function close() {
    profilPopinStore.close();
    setTimeout(() => { mode = 'select'; name = ""; editId = null; errorName = null; }, 200);
  }

  async function selectProfil(profil: any) {
    const current = $profilSelector.profilSelected;
    if (current?.id === profil.id) {
      close();
      return;
    }

    profilSelector.setProfil(profil);
    await reloadAfterSwitch();
    close();
  }

  async function submit() {
    errorName = null;
    if (!name.trim()) {
      errorName = "Le nom est obligatoire";
      return;
    }

    isSubmitting = true;
    try {
      if (mode === 'create') {
        await profilSelector.createProfil(name, PALETTE[selectedColorIndex]);
        await profilSelector.refresh();
        back();
      } else if (mode === 'edit' && editId) {
        await profilSelector.updateProfil(editId, name, PALETTE[editIndex]);
        back();
      }
    } catch (error: any) {
      const message = String(error?.message ?? error ?? "");
      if (message.includes("UNIQUE") || message.includes("duplicate")) {
        errorName = "Ce nom est déjà utilisé";
      } else {
        errorName = "Une erreur est survenue";
      }
    } finally {
      isSubmitting = false;
    }
  }

  async function handleDelete(id: number) {
    try {
      await profilSelector.deleteProfil(id);
      back();
    } catch (error) {
      console.error("Failed to delete profil", error);
    }
  }
</script>

<Modal
  open={$profilPopinStore}
  size="full"
  backdrop="blur"
  closable={false}
  closeOnOverlay={false}
  onclose={close}
  classes={{
    overlay: 'bg-black/95',
    content: 'bg-transparent border-none shadow-none',
    body: 'flex items-center justify-center min-h-screen'
  }}
>
  <!-- Croix fixe coin supérieur droit -->
  <button
    type="button"
    class="fixed top-6 right-6 z-200 w-11 h-11 rounded-full flex items-center justify-center
           bg-neutral-200/60 dark:bg-white/10 backdrop-blur-md border border-neutral-200 dark:border-white/10
           text-neutral-500 dark:text-neutral-400 dark:text-white/50 hover:text-neutral-900 dark:hover:text-white hover:bg-white/20
           transition-all duration-200 cursor-pointer"
    aria-label="Fermer"
    onclick={close}
  >
    <svg class="w-5 h-5" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24">
      <path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" />
    </svg>
  </button>

  {#if mode === 'select'}
    <!-- SÉLECTION PROFIL -->
    <div class="flex flex-col items-center gap-12">
      <!-- Titre -->
      <div class="text-center space-y-3">
        <h2 class="text-4xl font-bold text-neutral-900 dark:text-white tracking-tight">
          Qui écoute ?
        </h2>
        <p class="text-sm text-neutral-500 dark:text-neutral-400 dark:text-white/35 max-w-md">
          Sélectionne ton profil pour retrouver tes bibliothèques et playlists
        </p>
      </div>

      <!-- Grille des profils -->
      <div class="flex flex-wrap items-center justify-center gap-10">
        {#each $profilSelector.profils as profil}
          <div class="group relative flex flex-col items-center gap-4">
            <!-- Vignette cliquable -->
            <button
              type="button"
              class="relative cursor-pointer transition-all duration-300 ease-out
                     rounded-2xl overflow-hidden
                     {$profilSelector.profilSelected?.id === profil.id
                       ? 'scale-110 ring-[3px] ring-green-400 shadow-[0_0_40px_rgba(74,222,128,0.25)]'
                       : 'hover:scale-108 hover:shadow-2xl ring-[3px] ring-transparent hover:ring-white/20'}"
              onclick={() => selectProfil(profil)}
            >
              <Avatar
                src={profil.avatar ?? undefined}
                name={profil.name}
                size="xl"
                color={profil.color}
                classes={{ root: '!w-32 !h-32 !rounded-2xl !text-5xl' }}
              />
            </button>

            <!-- Nom + badge -->
            <div class="flex flex-col items-center gap-1.5">
              <span class="text-sm font-medium text-neutral-600 dark:text-white/60 group-hover:text-white
                           transition-colors duration-200 text-center truncate w-32">
                {profil.name}
              </span>
              {#if profil.role === 'admin'}
                <Badge color="amber" variant="soft" size="xs">admin</Badge>
              {/if}
            </div>

            <!-- Bouton modifier -->
            <button
              type="button"
              class="flex items-center gap-1.5 px-3 py-1 rounded-full cursor-pointer
                     text-[11px] font-medium text-neutral-600 dark:text-neutral-300 dark:text-white/20
                     hover:text-neutral-500 dark:text-neutral-400 dark:text-white/50 hover:bg-neutral-100 dark:bg-white/5
                     transition-all duration-200"
              onclick={() => switchToEdit(profil)}
            >
              <svg class="w-3 h-3" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" d="m16.862 4.487 1.687-1.688a1.875 1.875 0 1 1 2.652 2.652L10.582 16.07a4.5 4.5 0 0 1-1.897 1.13L6 18l.8-2.685a4.5 4.5 0 0 1 1.13-1.897l8.932-8.931Z" />
              </svg>
              Modifier
            </button>
          </div>
        {/each}

        <!-- Bouton ajouter -->
        <div class="flex flex-col items-center gap-4">
          <button
            type="button"
            class="w-32 h-32 rounded-2xl cursor-pointer
                   border-[3px] border-dashed border-neutral-200 dark:border-white/12
                   hover:border-white/25 hover:bg-neutral-100 dark:bg-white/5
                   flex items-center justify-center
                   transition-all duration-300 ease-out hover:scale-105"
            aria-label="Ajouter un profil"
            onclick={switchToCreate}
          >
            <svg class="w-10 h-10 text-neutral-600 dark:text-neutral-300 dark:text-white/20" fill="none" stroke="currentColor" stroke-width="1.5" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" />
            </svg>
          </button>
          <span class="text-sm font-medium text-neutral-500 dark:text-neutral-400 dark:text-white/35">
            Ajouter
          </span>
        </div>
      </div>
    </div>

  {:else}
    <!-- FORMULAIRE CREATE / EDIT PREMIUM -->
    {@const selectedHex = mode === 'edit' ? PALETTE[editIndex] : PALETTE[selectedColorIndex]}
    <div class="w-full max-w-lg">
      <div class="relative rounded-3xl border border-neutral-200 dark:border-white/8 bg-neutral-100 dark:bg-white/3 backdrop-blur-2xl
                  shadow-[0_8px_64px_rgba(0,0,0,0.6)] overflow-hidden">

        <!-- Glow supérieur dynamique -->
        <div
          class="absolute top-0 left-1/2 -translate-x-1/2 w-80 h-40 rounded-full blur-[80px] opacity-25 pointer-events-none transition-all duration-500"
          style="background: {selectedHex};"
        ></div>

        <!-- Bouton retour -->
        <button
          type="button"
          class="absolute top-5 left-6 z-10 text-neutral-500 dark:text-neutral-400 dark:text-white/25 hover:text-neutral-600 dark:text-white/60
                 transition-all duration-200 cursor-pointer flex items-center gap-1.5 text-xs font-medium
                 hover:gap-2"
          onclick={back}
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" d="M15.75 19.5 8.25 12l7.5-7.5" />
          </svg>
          Retour
        </button>

        <!-- Header — Avatar preview + titre -->
        <div class="relative flex flex-col items-center gap-5 pt-14 pb-8 px-8">
          <!-- Avatar preview live -->
          <div class="relative">
            <div
              class="absolute inset-0 rounded-2xl blur-2xl opacity-40 scale-125 transition-all duration-500"
              style="background: {selectedHex};"
            ></div>
            <div class="relative transition-all duration-300">
              <Avatar
                name={name || '?'}
                size="xl"
                color={selectedHex}
                classes={{ root: '!w-28 !h-28 !rounded-2xl !text-5xl !font-black ring-2 ring-white/15 shadow-2xl' }}
              />
            </div>
          </div>

          <div class="text-center space-y-1">
            <h2 class="text-2xl font-bold text-neutral-900 dark:text-white tracking-tight">
              {mode === 'create' ? 'Nouveau profil' : 'Modifier le profil'}
            </h2>
            <p class="text-xs text-neutral-500 dark:text-neutral-400 dark:text-white/30">
              {mode === 'create' ? 'Crée un profil pour personnaliser ton expérience' : 'Modifie les informations de ce profil'}
            </p>
          </div>
        </div>

        <!-- Séparateur subtil -->
        <div class="mx-8 h-px bg-linear-to-r from-transparent via-white/10 to-transparent"></div>

        <!-- Body formulaire -->
        <div class="px-10 pb-10 pt-8 space-y-8">

          <!-- Champ nom -->
          <div class="space-y-2">
            <label for="profil_name" class="block text-[11px] font-semibold uppercase tracking-widest text-neutral-500 dark:text-neutral-400 dark:text-white/40 mb-1">
              Nom du profil
            </label>
            <input
              id="profil_name"
              type="text"
              class="w-full rounded-xl px-5 py-3.5 text-sm font-medium
                     bg-neutral-100 dark:bg-white/6 border border-neutral-200 dark:border-white/8
                     text-neutral-900 dark:text-white placeholder-white/25
                     outline-none transition-all duration-200
                     hover:bg-neutral-200/50 dark:bg-white/8 hover:border-neutral-200 dark:border-white/12
                     focus:bg-neutral-200/50 dark:bg-white/8 focus:border-white/20 focus:ring-1
                     {errorName
                       ? 'border-red-400/50 focus:ring-red-400/30'
                       : 'focus:ring-white/10'}"
              style={name ? `border-color: ${selectedHex}30; box-shadow: 0 0 0 1px ${selectedHex}20;` : ''}
              placeholder="Entrer un nom…"
              bind:value={name}
              oninput={() => errorName = null}
              disabled={isSubmitting}
            />
            {#if errorName}
              <p class="flex items-center gap-1.5 text-xs text-red-400/90 pt-0.5">
                <svg class="w-3.5 h-3.5 shrink-0" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24">
                  <circle cx="12" cy="12" r="10" /><path d="M12 8v4m0 4h.01" stroke-linecap="round" />
                </svg>
                {errorName}
              </p>
            {/if}
          </div>

          <!-- Sélecteur de couleur -->
          <div class="space-y-2">
            <span class="block text-[11px] font-semibold uppercase tracking-widest text-neutral-500 dark:text-neutral-400 dark:text-white/40 mb-4">
              Couleur du profil
            </span>
            <div class="flex items-center gap-2.5">
              {#each PALETTE as color, i}
                {@const activeIndex = mode === 'edit' ? editIndex : selectedColorIndex}
                <button
                  type="button"
                  class="w-7 h-7 shrink-0 aspect-square rounded-full cursor-pointer transition-all duration-200
                         {activeIndex === i
                           ? 'scale-110'
                           : 'hover:scale-105 opacity-50 hover:opacity-100'}"
                  style="background: {color};
                         {activeIndex === i
                           ? `box-shadow: 0 0 16px ${color}50, 0 0 0 2.5px rgba(0,0,0,0.4), 0 0 0 4.5px ${color}80;`
                           : ''}"
                  aria-label="Couleur {color}"
                  onclick={() => {
                    if (mode === 'edit') {
                      editIndex = i;
                    } else {
                      selectedColorIndex = i;
                    }
                  }}
                ></button>
              {/each}
            </div>
          </div>

          <!-- Séparateur -->
          <div class="h-px" style="background: linear-gradient(to right, transparent, {selectedHex}25, transparent);"></div>

          <!-- Actions -->
          <div class="flex items-center justify-between pt-1">
            <!-- Supprimer (gauche) -->
            <div>
              {#if mode === 'edit' && editId && editId !== 1}
                {#if !showDeleteConfirm}
                  <button
                    type="button"
                    class="flex items-center gap-1.5 text-sm text-red-400/40 hover:text-red-400
                           transition-colors cursor-pointer"
                    onclick={() => showDeleteConfirm = true}
                    disabled={isSubmitting}
                  >
                    <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" d="m14.74 9-.346 9m-4.788 0L9.26 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673a2.25 2.25 0 0 1-2.244 2.077H8.084a2.25 2.25 0 0 1-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 0 0-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 0 1 3.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 0 0-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 0 0-7.5 0" />
                    </svg>
                    Supprimer
                  </button>
                {:else}
                  <span class="text-xs text-red-400/60">Confirmez ci-dessous</span>
                {/if}
              {/if}
            </div>

            <!-- Annuler / Sauvegarder (droite) -->
            <div class="flex items-center gap-4">
              <button
                type="button"
                class="text-sm font-medium text-neutral-500 dark:text-neutral-400 dark:text-white/30 hover:text-neutral-600 dark:text-white/60
                       transition-colors cursor-pointer disabled:opacity-50"
                onclick={back}
                disabled={isSubmitting}
              >
                Annuler
              </button>

              <button
                type="button"
                class="flex items-center gap-2 px-5 py-2 rounded-full text-sm font-semibold text-neutral-900 dark:text-white
                       transition-all duration-200 cursor-pointer
                       hover:brightness-110 hover:shadow-lg
                       active:scale-95
                       disabled:opacity-50 disabled:cursor-not-allowed"
                style="background: {selectedHex};
                       box-shadow: 0 4px 24px {selectedHex}40;"
                onclick={submit}
                disabled={isSubmitting}
              >
                {#if isSubmitting}
                  <svg class="w-4 h-4 animate-spin" fill="none" viewBox="0 0 24 24">
                    <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                    <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"></path>
                  </svg>
                  {mode === 'create' ? 'Création…' : 'Sauvegarde…'}
                {:else}
                  <svg class="w-4 h-4" fill="none" stroke="currentColor" stroke-width="2.5" viewBox="0 0 24 24">
                    {#if mode === 'create'}
                      <path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" />
                    {:else}
                      <path stroke-linecap="round" stroke-linejoin="round" d="m4.5 12.75 6 6 9-13.5" />
                    {/if}
                  </svg>
                  {mode === 'create' ? 'Créer le profil' : 'Sauvegarder'}
                {/if}
              </button>
            </div>
          </div>

          <!-- Zone de confirmation suppression -->
          {#if showDeleteConfirm}
            <div class="rounded-xl border border-red-500/15 bg-red-500/5 p-4 space-y-3">
              <div class="flex items-start gap-2.5">
                <svg class="w-4 h-4 text-red-400 shrink-0 mt-0.5" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M12 9v3.75m-9.303 3.376c-.866 1.5.217 3.374 1.948 3.374h14.71c1.73 0 2.813-1.874 1.948-3.374L13.949 3.378c-.866-1.5-3.032-1.5-3.898 0L2.697 16.126Z" />
                </svg>
                <p class="text-[11px] text-red-300/70 leading-relaxed">
                  Toutes les données de ce profil seront définitivement supprimées (bibliothèques, playlists, favoris). Tapez <strong class="text-red-300">supprimer</strong> pour confirmer.
                </p>
              </div>

              <input
                type="text"
                class="w-full rounded-lg px-3.5 py-2 text-xs
                       bg-black/20 border border-red-500/15 text-neutral-900 dark:text-white placeholder-white/15
                       outline-none focus:border-red-400/40 focus:ring-1 focus:ring-red-400/15
                       transition-all duration-200"
                placeholder="Tapez supprimer"
                bind:value={deleteConfirmText}
              />

              <div class="flex items-center gap-2">
                <button
                  type="button"
                  class="flex-1 py-2 rounded-lg text-xs font-semibold cursor-pointer
                         transition-all duration-200
                         {deleteConfirmText.toLowerCase().trim() === 'supprimer'
                           ? 'bg-red-500 text-white hover:bg-red-600 active:scale-97'
                           : 'bg-neutral-100 text-neutral-300 dark:bg-white/4 dark:text-white/15 cursor-not-allowed'}"
                  disabled={deleteConfirmText.toLowerCase().trim() !== 'supprimer' || isSubmitting}
                  onclick={() => editId && handleDelete(editId)}
                >
                  Supprimer définitivement
                </button>
                <button
                  type="button"
                  class="px-4 py-2 rounded-lg text-xs text-neutral-500 dark:text-neutral-400 dark:text-white/30 hover:text-neutral-600 dark:text-white/60
                         hover:bg-neutral-100 dark:bg-white/5 transition-all duration-200 cursor-pointer"
                  onclick={() => { showDeleteConfirm = false; deleteConfirmText = ""; }}
                >
                  Annuler
                </button>
              </div>
            </div>
          {/if}
        </div>
      </div>
    </div>
  {/if}
</Modal>
