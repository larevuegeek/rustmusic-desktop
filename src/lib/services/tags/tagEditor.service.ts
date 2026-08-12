/**
 * Édition des métadonnées d'un fichier audio.
 * Miroir de `src-tauri/src/commands/tag_command.rs`.
 */

import { invoke } from "@tauri-apps/api/core";

/**
 * Ce qu'on envoie au backend.
 *
 * Convention identique pour tous les champs :
 *  - champ **absent**       → ne touche pas à la valeur du fichier
 *  - chaîne **vide**        → efface la valeur
 *  - chaîne renseignée      → remplace
 *
 * C'est ce qui rend l'édition multiple possible : corriger l'album de dix
 * morceaux sans écraser leurs dix titres différents.
 */
export type TagEditPayload = {
  title?: string;
  artist?: string;
  album?: string;
  album_artist?: string;
  year?: string;
  genre?: string;
  comment?: string;
  composer?: string;
  track_number?: string;
  total_tracks?: string;
  disc_number?: string;
  total_discs?: string;
  /**
   * Liste **finale** des images voulues, ou champ absent pour n'y pas toucher.
   *
   * Décrire l'état visé plutôt que des opérations couvre les cinq gestes de
   * l'interface — ajouter, remplacer, supprimer, définir comme pochette,
   * réordonner — en une seule écriture atomique du fichier.
   */
  images?: ImageSlotPayload[];
};

/** Une image de la liste finale. `id` et `path` s'excluent. */
export type ImageSlotPayload = {
  /** Image déjà dans le fichier, désignée par son identifiant de contenu. */
  id?: string | null;
  /** Image à intégrer depuis le disque. Rust la lit et la prépare lui-même. */
  path?: string | null;
  picture_type: number;
  description?: string | null;
};

/** Les tags tels qu'ils sont dans le fichier, limités aux champs éditables. */
export type EditableTags = {
  title: string | null;
  artist: string | null;
  album: string | null;
  album_artist: string | null;
  year: string | null;
  genre: string | null;
  comment: string | null;
  composer: string | null;
  track_number: number | null;
  total_tracks: number | null;
  disc_number: number | null;
  total_discs: number | null;
};

/**
 * Relit les tags directement depuis le fichier.
 *
 * Indispensable : la vue de la bibliothèque est une projection de la base qui
 * ne stocke qu'une partie des tags. S'appuyer dessus afficherait des champs
 * vides pour le commentaire, le compositeur ou les totaux — et les effacerait
 * à l'enregistrement.
 */
export async function readTrackTags(path: string): Promise<EditableTags> {
  return invoke<EditableTags>("read_track_tags", { path });
}

/** Une image intégrée au fichier. */
export type EmbeddedImage = {
  /**
   * Identifiant tiré du contenu de l'image. C'est par lui qu'on la redésigne
   * à l'écriture : une position se décale au premier réordonnancement.
   */
  id: string;
  /** Position dans le fichier : c'est l'ordre d'affichage. */
  index: number;
  /** Type nommé (`CoverFront`, `CoverBack`, `Artist`…). */
  kind: string;
  /** Le même type sous sa forme numérique ID3 — celle qu'on renvoie. */
  picture_type: number;
  /** Vrai pour la pochette avant — calculé sur le type, pas sur la position. */
  is_cover: boolean;
  mime_type: string;
  description: string | null;
  bytes: number;
  /** Data URI directement affichable. */
  src: string;
};

/** Code ID3 de la pochette avant. Une image n'est pas la pochette parce
 *  qu'elle arrive en premier, mais parce qu'elle porte ce type. */
export const PICTURE_TYPE_COVER = 3;
export const PICTURE_TYPE_OTHER = 0;

/**
 * Une image telle que l'interface la manipule pendant l'édition.
 *
 * Elle unifie deux origines — une image déjà dans le fichier (`id`) et une
 * image choisie sur le disque (`path`) — pour que les cinq gestes s'écrivent
 * comme de simples manipulations de liste, sans distinguer les cas.
 */
export type MediaSlot = {
  /** Clé stable pour `{#each}` : l'index bougerait à chaque réordonnancement. */
  key: string;
  id: string | null;
  path: string | null;
  src: string;
  picture_type: number;
  description: string;
  mime_type: string;
  bytes: number;
  /** Renseigné pour une image ajoutée que la préparation a réduite. */
  originalBytes?: number;
  recompressed?: boolean;
};

/** Résultat de la préparation d'une image choisie sur le disque. */
export type PreparedImage = {
  src: string;
  mime_type: string;
  width: number;
  height: number;
  bytes: number;
  original_bytes: number;
  recompressed: boolean;
};

/**
 * Prépare une image du disque et renvoie de quoi l'afficher.
 *
 * L'aperçu montre le résultat réel : la préparation est déterministe, donc
 * l'image affichée est exactement celle qui sera écrite. Sans cet aller-retour
 * on validerait à l'aveugle une recompression irréversible.
 */
export async function prepareImage(path: string): Promise<PreparedImage> {
  return invoke<PreparedImage>("prepare_image", { path });
}

/** Liste les images intégrées, dans leur ordre d'apparition. */
export async function readTrackImages(path: string): Promise<EmbeddedImage[]> {
  return invoke<EmbeddedImage[]>("read_track_images", { path });
}

/**
 * Clé de traduction d'un type d'image ID3, ou `null` pour les types rares.
 *
 * Seuls les types qu'on rencontre réellement sont nommés. Les vingt et un que
 * définit la spécification ne justifient pas cent cinq traductions pour des
 * cas qu'on ne verra jamais — et l'affichage générique ne fait courir aucun
 * risque : le code numérique est conservé tel quel à l'écriture, donc un type
 * exotique survit à une édition même sans libellé.
 */
export function pictureTypeKey(type: number): string | null {
  const keys: Record<number, string> = {
    0: "tags.pic.other",
    3: "tags.pic.cover_front",
    4: "tags.pic.cover_back",
    5: "tags.pic.leaflet",
    6: "tags.pic.media_label",
    7: "tags.pic.lead_artist",
    8: "tags.pic.artist",
    11: "tags.pic.composer",
    18: "tags.pic.illustration",
  };
  return keys[type] ?? null;
}

/** Les types proposés au choix dans l'interface. */
export const OFFERED_PICTURE_TYPES = [3, 4, 5, 6, 8, 11, 18, 0];

/** Construit l'état d'édition à partir des images lues dans le fichier. */
export function slotsFromImages(images: EmbeddedImage[]): MediaSlot[] {
  return images.map((img) => ({
    key: img.id,
    id: img.id,
    path: null,
    src: img.src,
    picture_type: img.picture_type,
    description: img.description ?? "",
    mime_type: img.mime_type,
    bytes: img.bytes,
  }));
}

/** Traduit l'état d'édition en charge utile pour le backend. */
export function slotsToPayload(slots: MediaSlot[]): ImageSlotPayload[] {
  return slots.map((s) => ({
    id: s.id,
    path: s.path,
    picture_type: s.picture_type,
    description: s.description || null,
  }));
}

/**
 * Empreinte de la liste, pour savoir si les médias ont bougé.
 *
 * Elle prend l'origine, le type et l'ORDRE : sans l'ordre, permuter deux
 * images ne serait pas détecté et le bouton d'enregistrement resterait éteint.
 */
export function slotsSignature(slots: MediaSlot[]): string {
  return slots.map((s) => `${s.id ?? s.path}:${s.picture_type}`).join("|");
}

/**
 * Promeut une image en pochette et rétrograde celle qui l'était.
 *
 * L'ID3 n'admet qu'une seule image de type 3. En laisser deux rendrait
 * l'affichage imprévisible : chaque lecteur trancherait à sa façon.
 */
export function promoteToCover(slots: MediaSlot[], index: number): MediaSlot[] {
  return slots.map((s, i) => {
    if (i === index) return { ...s, picture_type: PICTURE_TYPE_COVER };
    if (s.picture_type === PICTURE_TYPE_COVER) {
      return { ...s, picture_type: PICTURE_TYPE_OTHER };
    }
    return s;
  });
}

/** Formate un poids en octets. */
export function formatBytes(bytes: number): string {
  if (bytes >= 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} Mo`;
  if (bytes >= 1024) return `${Math.round(bytes / 1024)} Ko`;
  return `${bytes} o`;
}

/** Le fichier accepte-t-il une réécriture de ses tags ? */
export async function canWriteTags(path: string): Promise<boolean> {
  try {
    return await invoke<boolean>("can_write_tags", { path });
  } catch {
    return false;
  }
}

/**
 * Écrit les tags puis resynchronise la bibliothèque.
 * Retourne la vue du morceau mise à jour.
 */
export async function writeTrackTags(path: string, edit: TagEditPayload): Promise<unknown> {
  return invoke("write_track_tags", { path, edit });
}

/**
 * Construit la charge utile en ne gardant que les champs réellement modifiés.
 *
 * C'est ici que se joue la sémantique « ne touche pas » : un champ dont la
 * valeur est inchangée est **omis**, pas envoyé vide. L'envoyer vide voudrait
 * dire « efface », ce qui détruirait les tags qu'on ne voulait pas toucher.
 */
export function buildEdit(
  original: Record<string, string>,
  current: Record<string, string>,
): TagEditPayload {
  const edit: Record<string, string> = {};
  for (const key of Object.keys(current)) {
    if ((current[key] ?? "") !== (original[key] ?? "")) {
      edit[key] = current[key] ?? "";
    }
  }
  return edit as TagEditPayload;
}
