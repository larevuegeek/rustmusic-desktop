import type { Profil } from "#lib/types/db/profil/Profil";
import { invoke } from "@tauri-apps/api/core";
import { get, writable } from "svelte/store";

type ProfilStoreState = {
  profilSelected: Profil | null;
  profils: Profil[];
  initialized: boolean;
};

const profilSelectorWriter = writable<ProfilStoreState>({
  profilSelected: null,
  profils: [],
  initialized: false
});

export const profilSelector = {
  subscribe: profilSelectorWriter.subscribe,
  init: async () => {
    const { initialized } = get(profilSelectorWriter);
    if (initialized) return;

    try {
      const profils = await invoke<Profil[]>('get_all_profils');

      const adminProfil = profils.find(p => p.id === 1) ?? profils[0] ?? null;

      profilSelectorWriter.set({
        profilSelected: adminProfil,
        profils,
        initialized: true
      });
    } catch (e) {
      console.error('[profilSelector] Erreur chargement profils', e);
      profilSelectorWriter.set({
        profilSelected: null,
        profils: [],
        initialized: true
      });
    }
  },
  setProfil: (profil: Profil | null) => {
    profilSelectorWriter.update((state) => ({
      ...state,
      profilSelected: profil
    }));
  },
  refresh: async () => {
    try {
      const profils = await invoke<Profil[]>('get_all_profils');
      profilSelectorWriter.update((state) => ({
        ...state,
        profils
      }));
    } catch (e) {
      console.error('[profilSelector] Erreur refresh profils', e);
    }
  },
  createProfil: async (name: string, color: string = 'violet', avatar: string | null = null) => {
    const profil = await invoke<Profil>('create_profil', {
      payload: { name: name.trim(), avatar, color }
    });

    profilSelectorWriter.update((state) => ({
      ...state,
      profils: [...state.profils, profil]
    }));

    return profil;
  },
  updateProfil: async (id: number, name: string, color: string, avatar: string | null = null) => {
    const profil = await invoke<Profil>('update_profil', {
      payload: { id, name: name.trim(), avatar, color }
    });

    profilSelectorWriter.update((state) => ({
      ...state,
      profils: state.profils.map(p => p.id === id ? profil : p),
      profilSelected: state.profilSelected?.id === id ? profil : state.profilSelected
    }));

    return profil;
  },
  deleteProfil: async (id: number) => {
    await invoke('delete_profil', { profilId: id });

    profilSelectorWriter.update((state) => {
      const profils = state.profils.filter(p => p.id !== id);
      const profilSelected = state.profilSelected?.id === id
        ? profils[0] ?? null
        : state.profilSelected;

      return { ...state, profils, profilSelected };
    });
  },
  clear: () => {
    profilSelectorWriter.set({
      profilSelected: null,
      profils: [],
      initialized: true
    });
  }
};
