/**
 * ═══════════════════════════════════════════════════════════
 * preloadAction — Preload des données au survol (hover)
 * ═══════════════════════════════════════════════════════════
 *
 * POURQUOI ?
 * Entre le moment où l'utilisateur survole un album et le moment
 * où il clique, il se passe ~200-500ms. C'est suffisant pour
 * lancer un fetch en avance. Quand il arrive sur la page,
 * les données sont déjà dans le cache → affichage instantané.
 *
 * COMMENT ÇA MARCHE ?
 * On utilise une "Svelte action" — c'est une fonction qui reçoit
 * un élément DOM et lui attache des event listeners automatiquement.
 *
 * Usage dans un composant :
 * ```svelte
 * <a href="/album/123" use:preload={() => preloadAlbum('123')}>
 *   Mon Album
 * </a>
 * ```
 *
 * Quand la souris entre sur le <a>, la fonction preloadAlbum('123')
 * est appelée. Elle fetch les données et les met en cache.
 * Quand l'utilisateur clique et arrive sur la page album,
 * le cache est déjà rempli → 0ms de latence.
 *
 * SÉCURITÉS :
 * - Debounce de 100ms pour éviter les fetches sur un survol rapide
 * - Un Set<string> de "déjà en cours" pour ne pas lancer 2x le même fetch
 */

import { dataCache } from "$lib/stores/cache/dataCache.store";
import { loadAlbum, loadArtist, loadTracksByAlbum } from "$lib/services/library/library.service";
import { invoke } from "@tauri-apps/api/core";
import type { TrackListView } from "$lib/types/ui/library/track/TrackListView";

/**
 * Set des clés en cours de preload.
 * Empêche de lancer le même fetch 2 fois si l'utilisateur
 * survole rapidement le même élément plusieurs fois.
 */
const inflight = new Set<string>();

/**
 * Svelte Action : preload au survol
 *
 * @param node — l'élément DOM (ajouté automatiquement par Svelte)
 * @param fetchFn — la fonction à appeler au survol
 *
 * Une "Svelte action" est une fonction de la forme :
 *   function(node: HTMLElement, params) { return { destroy() {} } }
 *
 * - Svelte l'appelle quand l'élément est monté dans le DOM
 * - Elle retourne un objet avec destroy() appelé au démontage
 * - C'est le pattern idéal pour attacher/détacher des listeners
 */
export function preload(node: HTMLElement, fetchFn: () => void) {
    let timeout: ReturnType<typeof setTimeout>;

    function onEnter() {
        // 250 ms : en dessous, un simple balayage de la grille déclenche une
        // requête par vignette survolée.
        timeout = setTimeout(fetchFn, 250);
    }

    function onLeave() {
        // La souris est partie → annuler le fetch prévu
        clearTimeout(timeout);
    }

    // Attacher les listeners
    // 'pointerenter' est plus moderne que 'mouseenter'
    // et fonctionne aussi avec le tactile
    node.addEventListener('pointerenter', onEnter);
    node.addEventListener('pointerleave', onLeave);

    return {
        // Svelte appelle destroy() quand le composant est démonté
        destroy() {
            clearTimeout(timeout);
            node.removeEventListener('pointerenter', onEnter);
            node.removeEventListener('pointerleave', onLeave);
        }
    };
}

// ─── Fonctions de preload spécifiques ───

/**
 * Preload un album : ses détails + ses tracks.
 *
 * Appelé au survol d'un AlbumListItem.
 * Quand l'utilisateur clique et arrive sur la page album,
 * dataCache.get("album:xxx") retourne instantanément les données.
 */
export function preloadAlbumData(libraryId: number, albumId: string) {
    const cacheKey = `album:${albumId}`;

    // Connu, même périmé : la page s'en sert telle quelle et se rafraîchit
    // seule. Repartir sur `fresh` relançait deux requêtes par vignette toutes
    // les trente secondes, à chaque passage de souris.
    if (dataCache.has(cacheKey) || inflight.has(cacheKey)) return;
    inflight.add(cacheKey);

    // Lancer les 2 fetches en parallèle
    Promise.all([
        loadAlbum(albumId),
        loadTracksByAlbum(libraryId, albumId),
    ]).then(([album, tracks]) => {
        if (album) dataCache.set(cacheKey, album);
        if (tracks) dataCache.set(`album-tracks:${albumId}`, tracks);
    }).catch(() => {
        // Silencieux — c'est du preload, pas critique
    }).finally(() => {
        inflight.delete(cacheKey);
    });
}

/**
 * Preload un artiste : ses détails.
 */
export function preloadArtistData(artistId: string) {
    const cacheKey = `artist:${artistId}`;

    if (dataCache.has(cacheKey) || inflight.has(cacheKey)) return;
    inflight.add(cacheKey);

    loadArtist(artistId).then(artist => {
        if (artist) dataCache.set(cacheKey, artist);
    }).catch(() => {
        // Silencieux
    }).finally(() => {
        inflight.delete(cacheKey);
    });
}

/**
 * Preload les tracks d'un artiste.
 */
export function preloadArtistTracks(libraryId: number, artistId: string) {
    const cacheKey = `artist-tracks:${artistId}`;

    const cached = dataCache.get(cacheKey);
    if (cached?.fresh) return;

    if (inflight.has(cacheKey)) return;
    inflight.add(cacheKey);

    invoke<TrackListView[]>('get_tracks_by_artist', {
        libraryId, artistId
    }).then(tracks => {
        dataCache.set(cacheKey, tracks);
    }).catch(() => {
        // Silencieux
    }).finally(() => {
        inflight.delete(cacheKey);
    });
}
