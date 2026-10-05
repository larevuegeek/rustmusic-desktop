/**
 * ═══════════════════════════════════════════════════════════
 * libraryContent.store — Store du contenu d'une bibliothèque
 * ═══════════════════════════════════════════════════════════
 *
 * Ce store gère les listes d'albums et d'artistes
 * d'une bibliothèque. C'est lui qui alimente les pages
 * /library/[id]/albums, /artists. Les titres, eux, se chargent page par page (onglet Morceaux).
 *
 * AVANT : chaque appel à load() faisait 3 requêtes SQL.
 * APRÈS : on utilise dataCache pour servir les données
 *         instantanément si elles sont en cache.
 *
 * Le pattern "Stale-While-Revalidate" :
 *   1. Si le cache est "fresh" (< 30s) → on affiche, c'est tout
 *   2. Si le cache est "stale" (> 30s) → on affiche ET on refresh en background
 *   3. Si pas de cache → on charge normalement (avec loader)
 */

import { loadAlbums, loadArtists } from "#lib/services/library/library.service";
import type { AlbumListView } from "#lib/types/ui/library/album/AlbumListView";
import type { ArtistListView } from "#lib/types/ui/library/artist/ArtistListView";
import { get, writable } from "svelte/store";
import { libraryStore } from "./library.store";
import { dataCache } from "#lib/stores/cache/dataCache.store";

// ─── Types ───

export type LibraryContentState = {
    albums: AlbumListView[];
    artists: ArtistListView[];
    isLoading: boolean;
    missingAlbumCover: boolean;
};

// ─── État initial ───

const initialState: LibraryContentState = {
    albums: [],
    artists: [],
    isLoading: false,
    missingAlbumCover: false,
};

// ─── Le writable Svelte (ce que les composants lisent via $libraryContentStore) ───

const libraryContentWritable = writable<LibraryContentState>(initialState);

// ─── Fonctions internes ───

/**
 * Charge les 3 types de données depuis SQLite via Tauri.
 * C'est la même chose qu'avant, mais extrait dans une fonction
 * pour pouvoir l'appeler depuis load() ET depuis le refresh background.
 */
async function fetchFromBackend(libraryId: number) {
    const state = get(libraryContentWritable);
    // Promise.all lance les 3 requêtes EN PARALLÈLE
    // → 3x plus rapide que les faire séquentiellement
    const [albums, artists] = await Promise.all([
        loadAlbums(libraryId, state.missingAlbumCover),
        loadArtists(libraryId)
    ]);
    return { albums, artists };
}

/**
 * Met à jour le cache ET le store Svelte avec les nouvelles données.
 * Appelé après chaque fetch réussi.
 */
function updateStoreAndCache(libraryId: number, albums: AlbumListView[], artists: ArtistListView[], n: number) {
    // 1. Stocker dans le cache mémoire
    //    Chaque type a sa propre clé pour pouvoir être invalidé indépendamment
    dataCache.set(`albums:${libraryId}`, albums);
    dataCache.set(`artists:${libraryId}`, artists);

    // Une réponse dépassée (autre bibliothèque ouverte entre-temps) reste en cache sans s'afficher.
    if (n !== jeton || libraryId !== courante) return;

    // 2. Mettre à jour le store Svelte → les composants se re-rendent
    libraryContentWritable.update(s => ({
        ...s,
        albums,
        artists,
        isLoading: false
    }));
}

// Bibliothèque affichée et numéro de la dernière demande : seule la plus récente s'affiche.
let courante: number | null = null;
let jeton = 0;

// ─── API publique (le store exporté) ───

export const libraryContentStore = {

    // subscribe permet aux composants de lire le store avec $libraryContentStore
    subscribe: libraryContentWritable.subscribe,

    /**
     * Charge le contenu d'une bibliothèque.
     *
     * C'est LA fonction clé du cache. Voici sa logique :
     *
     * ┌──────────────────────────────────────────┐
     * │  Cache existe ET frais (< 30s) ?         │
     * │  → OUI : afficher le cache, terminé ✅   │
     * │  → Cache existe mais stale (> 30s) ?     │
     * │    → Afficher le cache immédiatement     │
     * │    → Lancer un refresh en background 🔄  │
     * │  → Pas de cache du tout ?                │
     * │    → Afficher le loader ⏳               │
     * │    → Charger depuis SQLite               │
     * └──────────────────────────────────────────┘
     */
    load: async (libraryId: number) => {
        courante = libraryId;
        const n = ++jeton;

        // Essayer de lire les 2 caches
        const cachedAlbums  = dataCache.get<AlbumListView[]>(`albums:${libraryId}`);
        const cachedArtists = dataCache.get<ArtistListView[]>(`artists:${libraryId}`);

        const allCached = cachedAlbums && cachedArtists;

        if (allCached) {
            // ✅ Cache disponible → affichage INSTANTANÉ (0ms)
            libraryContentWritable.update(s => ({
                ...s,
                albums:  cachedAlbums.data,
                artists: cachedArtists.data,
                isLoading: false
            }));

            // Vérifie si tout est encore frais
            const allFresh = cachedAlbums.fresh && cachedArtists.fresh;

            if (!allFresh) {
                // 🔄 Stale → refresh silencieux en background
                // Le .then() signifie : "lance ça, mais n'attend pas"
                // L'utilisateur ne voit aucun loader
                fetchFromBackend(libraryId).then(({ albums, artists }) => {
                    updateStoreAndCache(libraryId, albums, artists, n);
                }).catch(err => {
                    console.warn('[cache] Background refresh failed:', err);
                    // Pas grave, les données stale sont toujours affichées
                });
            }
            // Si tout est fresh, on ne fait RIEN. Zéro requête SQL.
            return;
        }

        // ⏳ Pas de cache → chargement classique avec loader
        libraryContentWritable.update(s => ({ ...s, isLoading: true }));

        try {
            const { albums, artists } = await fetchFromBackend(libraryId);
            updateStoreAndCache(libraryId, albums, artists, n);
        } catch(error) {
            console.error("Failed to load library content", error);
            if (n === jeton) libraryContentWritable.update(s => ({ ...s, isLoading: false }));
        }
    },

    /**
     * Force un refresh (ignore le cache).
     * Utilisé après un import, un rescan, ou un changement de données.
     */
    // La bibliothèque affichée, pas la sélectionnée : un lien peut ouvrir une autre bibliothèque.
    refresh: async (libraryId?: number) => {
        dataCache.invalidateFiches();
        const cible = libraryId ?? courante ?? get(libraryStore).librarySelected?.id;
        if (!cible) return;
        if (cible !== courante) {
            // Pas affichée : son cache est périmé, elle se rechargera à l'ouverture.
            for (const k of ["albums", "artists"]) dataCache.invalidate(`${k}:${cible}`);
            return;
        }
        const n = ++jeton;

        libraryContentWritable.update(s => ({ ...s, isLoading: true }));

        try {
            const { albums, artists } = await fetchFromBackend(cible as number);
            updateStoreAndCache(cible as number, albums, artists, n);
        } catch(error) {
            console.error("Failed to refresh library content", error);
            if (n === jeton) libraryContentWritable.update(s => ({ ...s, isLoading: false }));
        }
    },

    /**
     * Active/désactive le filtre "albums sans cover" et recharge les albums.
     */
    setMissingAlbumCover: async (value: boolean) => {
        libraryContentWritable.update(s => ({ ...s, missingAlbumCover: value }));

        const libraryId = courante ?? get(libraryStore).librarySelected?.id;
        if (!libraryId) return;

        try {
            const albums = await loadAlbums(libraryId as number, value);
            dataCache.set(`albums:${libraryId}`, albums);
            if (libraryId === courante) libraryContentWritable.update(s => ({ ...s, albums }));
        } catch (e) {
            console.error('Failed to reload albums with filter:', e);
        }
    },

    /**
     * Vide le store ET le cache pour cette bibliothèque.
     * Appelé au changement de profil ou de bibliothèque.
     */
    clear: () => {
        courante = null;
        jeton++;
        libraryContentWritable.set(initialState);
        // On ne vide pas tout le cache ici, juste le store Svelte.
        // Le cache sera naturellement invalidé par invalidateAll()
        // lors du changement de profil.
    }
};
