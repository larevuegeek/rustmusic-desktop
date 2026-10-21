import { writable } from "svelte/store";

/** Demande d'ouverture du mode « Organiser » de la barre latérale (depuis les nouveautés). */
export const sidebarOrganize = writable(false);
