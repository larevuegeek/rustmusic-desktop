/**
 * Cache mémoire avec stale-while-revalidate, borné.
 *
 * Clés : `tracks:42`, `albums:42`, `artists:42` (listes d'une bibliothèque),
 * `album:<id>`, `artist:<id>`, `album-tracks:<id>` (fiches).
 *
 * Les fiches se remplissent au survol d'une vignette. Sur une bibliothèque de
 * six cents albums, parcourir la grille en cachait des centaines — chacune avec
 * la liste complète de ses pistes — et rien n'était jamais relâché. D'où le
 * plafond ci-dessous.
 */

type CacheEntry<T> = { data: T; timestamp: number };
type CacheResult<T> = { data: T; fresh: boolean };

/** Au-delà, la donnée est servie mais un rafraîchissement est conseillé. */
const MAX_AGE = 30_000;

/**
 * Nombre de fiches conservées. Les listes de bibliothèque n'y comptent pas :
 * elles sont peu nombreuses et coûteuses à refaire.
 */
const MAX_FICHES = 40;

const PREFIXES_LISTE = ['tracks:', 'albums:', 'artists:'];

const cache = new Map<string, CacheEntry<unknown>>();

// Une liste vide vient souvent d'une erreur avalée : on ne la met pas en cache.
const utile = <T>(r: T | null | undefined): r is T => r != null && !(Array.isArray(r) && r.length === 0);

const estListe = (key: string) =>
  PREFIXES_LISTE.some((p) => key.startsWith(p)) && !key.startsWith('album-tracks:');

/** Évince les fiches les plus anciennement utilisées au-delà du plafond. */
function elaguer() {
  let fiches = 0;
  for (const key of cache.keys()) if (!estListe(key)) fiches++;
  if (fiches <= MAX_FICHES) return;

  // `Map` garde l'ordre d'insertion, et `get` réinsère : les premières clés
  // sont donc les moins récemment utilisées.
  for (const key of [...cache.keys()]) {
    if (fiches <= MAX_FICHES) break;
    if (estListe(key)) continue;
    cache.delete(key);
    fiches--;
  }
}

export const dataCache = {
  get<T>(key: string): CacheResult<T> | null {
    const entry = cache.get(key);
    if (!entry) return null;

    // Réinsertion : marque l'entrée comme récemment utilisée pour l'élagage.
    cache.delete(key);
    cache.set(key, entry);

    return { data: entry.data as T, fresh: Date.now() - entry.timestamp < MAX_AGE };
  },

  set<T>(key: string, data: T): void {
    cache.delete(key);
    cache.set(key, { data, timestamp: Date.now() });
    elaguer();
  },

  /** Vrai si la clé est connue, fraîche ou non. */
  has(key: string): boolean {
    return cache.has(key);
  },

  invalidate(key: string): void {
    cache.delete(key);
  },

  invalidateByPrefix(prefix: string): void {
    for (const key of cache.keys()) {
      if (key.startsWith(prefix)) cache.delete(key);
    }
  },

  invalidateAll(): void {
    cache.clear();
  },

  /** Après une écriture (tags, renommage, pochette, note) : les fiches se rechargeront. */
  invalidateFiches(): void {
    for (const key of [...cache.keys()]) if (!estListe(key)) cache.delete(key);
  },

  /** Sert la fiche en cache tout de suite ; périmée, la relit et la repasse à `rafraichir`. */
  async lire<T>(key: string, charger: () => Promise<T | null | undefined>, rafraichir?: (v: T) => void): Promise<T | null> {
    const c = dataCache.get<T>(key);
    if (c && !c.fresh) {
      charger().then((r) => {
        if (!utile(r)) return;
        dataCache.set(key, r);
        rafraichir?.(r);
      }).catch(() => {});
    }
    if (c) return c.data;
    const r = await charger();
    if (utile(r)) dataCache.set(key, r);
    return r ?? null;
  },

  size(): number {
    return cache.size;
  },
};
