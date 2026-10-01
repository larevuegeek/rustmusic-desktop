// Albums dans l'ordre affiché par la vue Albums (tri, filtres) : précédent / suivant sur la page d'un album.
import { writable } from "svelte/store";

export const ordreAlbums = writable<{ libraryId: number; ids: string[] } | null>(null);
