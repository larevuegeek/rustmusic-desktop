import { get } from "svelte/store";
import { profilSelector } from "$lib/stores/profil/profil.store";
import type { QueueTrack } from "$lib/types/db/queue/QueueTrack";

/**
 * Ce qu'une liste sait déjà d'un morceau — assez pour le mettre en file sans
 * relire le fichier. `open_file` reste réservé au morceau qu'on lance.
 */
export type SourceDeFile = {
  /** `null` toléré : certaines vues listent des morceaux sans chemin connu. */
  path: string | null;
  title?: string | null;
  filename?: string | null;
  name?: string | null;
  artist?: string | null;
  duration?: number | null;
  cover?: string | null;
  thumbnail_path?: string | null;
};

export function versQueueTrack(source: SourceDeFile, position = 0): QueueTrack {
  const profil = get(profilSelector);
  return {
    queueId: crypto.randomUUID(),
    profilId: profil.profilSelected?.id ?? 1,
    path: source.path as string,
    title: source.title ?? source.name ?? source.filename ?? "Inconnu",
    artist: source.artist ?? undefined,
    duration: source.duration ?? undefined,
    cover: source.cover ?? source.thumbnail_path ?? undefined,
    position,
  };
}

/** Une liste affichée devient la file d'attente, dans le même ordre. */
export function versFileDAttente(sources: SourceDeFile[]): QueueTrack[] {
  return sources.filter((s) => s?.path).map((s, i) => versQueueTrack(s, i));
}

/**
 * L'ordre dans lequel la file va se lire à partir du morceau cliqué.
 *
 * `next()` avance de +1 sur une liste déjà mélangée : en aléatoire, le morceau
 * cliqué ouvre donc le bal et le reste est tiré au sort derrière lui.
 */
export function ordonnerPourLecture(
  tracks: QueueTrack[],
  path: string,
  melanger: boolean,
  tirage: (n: number) => number = (n) => Math.floor(Math.random() * n),
): { liste: QueueTrack[]; index: number } {

  const numeroter = (l: QueueTrack[]) => l.map((t, i) => ({ ...t, position: i }));

  if (!melanger) {
    const liste = numeroter(tracks);
    const trouve = liste.findIndex((t) => t.path === path);
    return { liste, index: trouve === -1 ? 0 : trouve };
  }

  const choisi = tracks.find((t) => t.path === path);
  const autres = tracks.filter((t) => t !== choisi);

  // Fisher-Yates, avec un tirage injectable pour que le test soit reproductible.
  for (let i = autres.length - 1; i > 0; i--) {
    const j = tirage(i + 1);
    [autres[i], autres[j]] = [autres[j], autres[i]];
  }

  return { liste: numeroter(choisi ? [choisi, ...autres] : autres), index: 0 };
}
