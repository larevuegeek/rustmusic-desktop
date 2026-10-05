import { invoke } from "@tauri-apps/api/core";
import { convertFileSrc } from "@tauri-apps/api/core";

export type CoverSize = '1x' | '2x' | 'full';

const COVER_MODE: 'asset' | 'base64' = 'asset';

/** convertFileSrc encode les / en %2F sur Linux → 403. On les décode + fix double-slash. */
function assetSrc(path: string): string {
    return convertFileSrc(path)
        .replaceAll('%2F', '/');
}

// ── Mode base64 (fallback) ──
const MAX_CACHE = 300;
const pending = new Map<string, Promise<string>>();
const resolved = new Map<string, string>();

function lruGet(path: string): string | undefined {
    if (!resolved.has(path)) return undefined;
    const val = resolved.get(path)!;
    resolved.delete(path);
    resolved.set(path, val);
    return val;
}

function lruSet(path: string, val: string) {
    if (resolved.size >= MAX_CACHE) {
        resolved.delete(resolved.keys().next().value!);
    }
    resolved.set(path, val);
}

// ── Miniatures : une seule question au backend par chemin ──
const MAX_MINIATURES = 2000;
const miniatures = new Map<string, Promise<string>>();

function miniature(chemin: string, original: string): Promise<string> {
    const connue = miniatures.get(chemin);
    if (connue) {
        miniatures.delete(chemin);
        miniatures.set(chemin, connue);
        return connue;
    }
    const p = invoke<string>("resolve_cover_thumbnail", { path: chemin })
        .then((reel) => {
            // Miniature encore en génération : le backend rend la grande, on redemandera.
            if (reel !== chemin) miniatures.delete(chemin);
            return assetSrc(reel);
        })
        .catch(() => {
            miniatures.delete(chemin);
            return assetSrc(original);
        });
    if (miniatures.size >= MAX_MINIATURES) miniatures.delete(miniatures.keys().next().value!);
    miniatures.set(chemin, p);
    return p;
}

// ── Fonction publique ──
// ── Pochettes floutées (fonds) : générées une fois par le backend ──
const MAX_FLOUES = 500;
const floues = new Map<string, Promise<string | null>>();

/** Version pré-floutée et saturée d'une pochette du disque ; `null` si impossible (distante, intégrée). */
export function resolveBlurredCover(
    path: string | null | undefined,
    saturation: number,
    flou: number,
): Promise<string | null> {
    if (!path || path.startsWith("http") || path.startsWith("data:") || path.startsWith("blob:") || path.startsWith("/images/")) {
        return Promise.resolve(null);
    }
    const cle = `${path}|${saturation}|${flou}`;
    const connue = floues.get(cle);
    if (connue) return connue;
    const p = invoke<string>("resolve_blurred_cover", { path, saturation, flou })
        .then(assetSrc)
        .catch(() => {
            floues.delete(cle);
            return null;
        });
    if (floues.size >= MAX_FLOUES) floues.delete(floues.keys().next().value!);
    floues.set(cle, p);
    return p;
}

export async function resolveCoverSrc(path: string | null | undefined, size: CoverSize = 'full'): Promise<string | null> {
    if (!path) return null;

    const resolvedPath = path.replace(/[\/\\]full[\/\\]/, `/${size}/`);

    if (resolvedPath.startsWith("http") || resolvedPath.startsWith("data:") || resolvedPath.startsWith("blob:")) return resolvedPath;
    // Image de l'interface (« pas de CD ») : servie telle quelle, ce n'est pas un fichier du disque.
    if (path.startsWith("/images/")) return path;

    // Mode asset protocol (rapide)
    if (COVER_MODE === 'asset') {
        // Miniature (1x/2x) générée à la volée ; la grande en cas d'échec.
        if (size !== 'full') return miniature(resolvedPath, path);
        return assetSrc(resolvedPath);
    }

    // Mode base64 (fallback)
    const cached = lruGet(resolvedPath);
    if (cached) return cached;

    if (!pending.has(resolvedPath)) {
        const promise = invoke<string>("read_cover_as_base64", { path: resolvedPath })
            .then(dataUri => {
                lruSet(resolvedPath, dataUri);
                pending.delete(resolvedPath);
                return dataUri;
            });
        pending.set(resolvedPath, promise);
    }
    return pending.get(resolvedPath)!;
}
