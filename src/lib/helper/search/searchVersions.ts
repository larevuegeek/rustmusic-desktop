// « Toutes les sources » : un même album (ou morceau) en local et sur un serveur ne fait qu'un résultat.
// Les morceaux ne se fusionnent pas (deux fichiers, deux bibliothèques) : on garde ses versions.
import type { Library } from "#lib/types/db/library/Library";
import { isRemotePath } from "#lib/types/db/library/Library";

export type SearchResult = {
  id: string;
  result_type: string;
  title: string;
  subtitle: string | null;
  thumbnail_path: string | null;
  path: string | null;
  library_id: number | null;
};

export type Version = { result: SearchResult; remote: boolean };
/** Le résultat affiché (sa version préférée) et toutes ses versions, préférée en tête. */
export type SearchHit = SearchResult & { versions: Version[] };

const norm = (s: string | null) =>
  (s ?? "").normalize("NFD").replace(/\p{M}/gu, "").toLowerCase().replace(/\s+/g, " ").trim();

/** `grouper` : vue « Toutes les sources » ; `preferServer` : la version du serveur passe devant. */
export function groupVersions(results: SearchResult[], libraries: Library[], grouper: boolean, preferServer: boolean): SearchHit[] {
  const remote = (r: SearchResult) => isRemotePath(r.path) || !!libraries.find((l) => l.id === r.library_id)?.server_id;
  if (!grouper) return results.map((r) => ({ ...r, versions: [{ result: r, remote: remote(r) }] }));
  const hits = new Map<string, Version[]>();
  for (const r of results) {
    // Un artiste se reconnaît à son nom ; un album ou un morceau, à son titre et à son artiste.
    const key = [r.result_type, norm(r.title), r.result_type === "artist" ? "" : norm(r.subtitle)].join("|");
    const v = { result: r, remote: remote(r) };
    const versions = hits.get(key);
    if (!versions) hits.set(key, [v]);
    else if (!versions.some((x) => x.remote === v.remote)) versions.push(v);
  }
  return [...hits.values()].map((versions) => {
    versions.sort((a, b) => Number(a.remote !== preferServer) - Number(b.remote !== preferServer));
    return { ...versions[0].result, versions };
  });
}
