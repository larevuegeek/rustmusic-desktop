import { invoke } from "@tauri-apps/api/core";

export type LyricsSource = "sidecar" | "lrclib" | "manual" | "server" | "none";

export type Lyrics = {
    track_id: string;
    plain: string | null;
    synced: string | null;
    source: LyricsSource;
    fetched_at: number;
    lrclib_id: number | null;
};

// Cache des requêtes en cours pour dédoublonner
// (évite que le pré-fetch + l'ouverture du panel = 2 appels en parallèle)
const inFlight = new Map<string, Promise<Lyrics | null>>();

/** Récupère les paroles (cache → sidecar → LRCLIB). null si rien trouvé. */
export async function getLyrics(path: string): Promise<Lyrics | null> {
    const existing = inFlight.get(path);
    if (existing) return existing;

    const promise = invoke<Lyrics | null>("get_lyrics", { path });
    inFlight.set(path, promise);

    try {
        return await promise;
    } finally {
        inFlight.delete(path);
    }
}

/** Force un re-fetch en vidant le cache puis en relançant. */
export async function refreshLyrics(path: string): Promise<Lyrics | null> {
    inFlight.delete(path);
    return await invoke<Lyrics | null>("refresh_lyrics", { path });
}
