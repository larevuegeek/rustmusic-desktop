// Filtre de source passé aux commandes (`null` : toutes). Tenu à jour par `source.store` ;
// n'importe rien, pour que J'aime, récents et playlists s'en servent sans boucle d'imports.
import { writable } from "svelte/store";

export const sourceFilter = writable<number | null>(null);
