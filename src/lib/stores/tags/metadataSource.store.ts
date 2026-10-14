// Base de métadonnées choisie (Deezer ou MusicBrainz), retenue d'une ouverture à l'autre.
import { writable } from "svelte/store";
import { lireLocal, ecrireLocal } from "#lib/helper/tools/stockage";
import type { MetadataSource } from "#lib/services/tags/metadata.service";

const CLE = "tags:source";

/** Les sources, dans l'ordre d'affichage. */
export const SOURCE_OPTIONS: { value: MetadataSource; label: string; icon: string }[] = [
  { value: "musicbrainz", label: "MusicBrainz", icon: "simple-icons:musicbrainz" },
  { value: "deezer", label: "Deezer", icon: "simple-icons:deezer" },
];

export const metadataSource = writable<MetadataSource>(lireLocal(CLE, "deezer") === "musicbrainz" ? "musicbrainz" : "deezer");

metadataSource.subscribe((v) => ecrireLocal(CLE, v));
