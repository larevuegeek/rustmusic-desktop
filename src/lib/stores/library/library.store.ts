import type { Library } from "$lib/types/db/library/Library"
import { profilSelector } from "$lib/stores/profil/profil.store";
import { writable, get } from "svelte/store"
import { invoke } from "@tauri-apps/api/core";

export type LibraryState = {
    librarySelected: Library | null
    libraries: Library[],
    isLoading: boolean,
    isImporting: boolean,
}

const initialState: LibraryState = {
    librarySelected: null,
    libraries: [],
    isLoading: false,
    isImporting: false
}

const libraryWriter = writable<LibraryState>(initialState);

/**
 * La bibliothèque à ouvrir : celle marquée par défaut, sinon la première.
 *
 * Le repli sur la première n'est pas censé servir — la base garantit un défaut
 * par profil — mais il évite de démarrer sur rien si la marque venait à
 * manquer (base restaurée à la main, migration interrompue).
 */
function pickDefault(libraries: Library[]): Library | null {
  return libraries.find(l => l.is_default) ?? libraries[0] ?? null;
}

export const libraryStore = {
  subscribe: libraryWriter.subscribe,
  init: async () => {
    libraryWriter.update(state => ({
      ...state,
      isLoading: true
    }));

     const profilState = get(profilSelector);
     const profilId = profilState.profilSelected?.id

    if (!profilId) {
        console.warn("No profil selected, skip library init");
        return;
    }

    try {
      const libraries = await invoke<Library[]>('get_libraries', { 
          profilId: profilId,
      });

      libraryWriter.update(state => ({
        ...state,
        libraries,
        librarySelected: pickDefault(libraries),
        isLoading: false
      }));
    } catch (error) {
      console.error("Failed to load libraries", error);
      libraryWriter.update(state => ({
        ...state,
        isLoading: false
      }));
    }

  },
  refresh: async () => {
    const profilState = get(profilSelector);
    const profilId = profilState.profilSelected?.id;

    if (!profilId) return;

    try {
      const libraries = await invoke<Library[]>('get_libraries', { 
          profilId: profilId,
      });

      libraryWriter.update(state => ({
        ...state,
        libraries,
        // La sélection courante prime : un rafraîchissement ne doit pas
        // ramener l'utilisateur ailleurs. Le défaut ne sert que si elle a
        // disparu entre-temps.
        librarySelected:
          libraries.find(l => l.id === state.librarySelected?.id) ?? pickDefault(libraries)
      }));
    } catch (error) {
      console.error("Failed to refresh libraries", error);
    }
  },
  selectLibrary: (library: Library | null) => {
    libraryWriter.update(state => ({
      ...state,
      librarySelected: library
    }));
  },
  /**
   * Désigne la bibliothèque ouverte au démarrage.
   *
   * L'état local est recalculé plutôt que rechargé : l'unicité du défaut est
   * tenue par la base (index partiel), on se contente de la refléter.
   */
  setDefault: async (library: Library) => {
    if (library.id === null) return;

    await invoke('set_default_library', { libraryId: library.id });

    libraryWriter.update(state => ({
      ...state,
      libraries: state.libraries.map(l => ({ ...l, is_default: l.id === library.id })),
      librarySelected:
        state.librarySelected && state.librarySelected.id === library.id
          ? { ...state.librarySelected, is_default: true }
          : state.librarySelected
    }));
  },
  setImporting: (importing: boolean) => {
    libraryWriter.update(state => ({
      ...state,
      isImporting: importing
    }));
  },
  addLibrary: async (profilId: number, name: string, description: string | null)  => {

    const newLibrary: Library = await invoke<Library>('create_library', { 
      payload: {
        profil_id: profilId,
        name: name.trim(),
        description: description?.trim() || null
      }
    });

    libraryWriter.update(state => {
      const libraries = [...state.libraries, newLibrary];

      return {
        ...state,
        libraries,
        librarySelected: state.librarySelected ?? newLibrary
      };
    });
  },
  removeLibrary: async (library: Library) => {
    
    if(library.id) {
      await invoke<Library>('remove_library', {
        libraryId: library.id
      });
    }

    libraryWriter.update(state => {
      const libraries = state.libraries.filter(l => l.id !== library.id);

      // Supprimer celle qu'on regardait renvoie sur le défaut du profil — le
      // backend vient d'en promouvoir un si c'est celui-ci qui a disparu.
      const librarySelected =
        state.librarySelected?.id === library.id
          ? pickDefault(libraries)
          : state.librarySelected;

      return {
        ...state,
        libraries,
        librarySelected
      };
    });
  },
  clear: () => {
    libraryWriter.set(initialState);
  }
};