// Nouveautés présentées après une mise à jour : quelques diapos par version.
// Textes dans `whats_new.slides.<id>` (titre, texte) des fichiers de langue.
import { goto } from "$app/navigation";
import { get } from "svelte/store";
import { settingsStore } from "#lib/stores/settings/settings.store";
import { sidebarOrganize } from "#lib/stores/ui/sidebarOrganize.store";
import { libraryStore } from "#lib/stores/library/library.store";

export type WhatsNewSlide = {
  id: string;
  /** Illustration : icône principale, dégradé, petites icônes autour. */
  icon: string;
  colors: [string, string];
  satellites?: string[];
  /** Capture ou image (dans `static/`) à la place de l'icône. */
  image?: string;
  /** Ce qui est pris en charge : pastilles icône + nom qui défilent (à la place des satellites). */
  logos?: { icon: string; label: string }[];
  /** Bouton « Essayer » : mène à la nouveauté. */
  action?: () => void;
};

const podcastsEnabled = () => get(settingsStore).podcasts === "true";
const radiosEnabled = () => get(settingsStore).radios === "true";


const RELEASES: Record<string, WhatsNewSlide[]> = {
  "0.4.0": [
    {
      id: "servers",
      icon: "material-symbols:cloud-outline-rounded",
      colors: ["#0ea5e9", "#6366f1"],
      satellites: ["material-symbols:dns-outline-rounded", "material-symbols:cloud-download-outline-rounded", "material-symbols:favorite-rounded"],
      logos: [
        { icon: "simple-icons:jellyfin", label: "Jellyfin" },
        { icon: "material-symbols:cloud-outline-rounded", label: "Navidrome" },
        { icon: "simple-icons:dlna", label: "DLNA" },
        { icon: "material-symbols:cloud-outline-rounded", label: "Subsonic" },
        { icon: "simple-icons:truenas", label: "TrueNAS" },
        { icon: "simple-icons:synology", label: "Synology" },
      ],
      action: () => goto("/settings/storage"),
    },
    {
      id: "sources",
      icon: "material-symbols:stacks-outline-rounded",
      colors: ["#8b5cf6", "#0ea5e9"],
      satellites: ["material-symbols:computer-outline-rounded", "material-symbols:cloud-outline-rounded", "material-symbols:swap-horiz-rounded"],
    },
    {
      id: "radios",
      icon: "material-symbols:radio-outline-rounded",
      colors: ["#f97316", "#e11d48"],
      satellites: ["material-symbols:history-rounded", "material-symbols:high-quality-outline-rounded", "material-symbols:cast-rounded"],
      logos: [
        { icon: "material-symbols:public", label: "Radio Browser" },
        { icon: "material-symbols:history-rounded", label: "Historique" },
        { icon: "material-symbols:high-quality-outline-rounded", label: "AAC · MP3 · HLS" },
        { icon: "material-symbols:cast-rounded", label: "UPnP" },
        { icon: "simple-icons:sonos", label: "Sonos" },
        { icon: "material-symbols:music-note-rounded", label: "Titre en direct" },
      ],
      action: () => goto(radiosEnabled() ? "/radios" : "/settings/online"),
    },
    {
      id: "podcasts",
      icon: "material-symbols:podcasts-rounded",
      colors: ["#8b5cf6", "#c026d3"],
      satellites: ["material-symbols:search-rounded", "material-symbols:rss-feed-rounded", "material-symbols:trophy-outline-rounded"],
      logos: [
        { icon: "simple-icons:applepodcasts", label: "Apple Podcasts" },
        { icon: "material-symbols:rss-feed-rounded", label: "RSS" },
        { icon: "material-symbols:trophy-outline-rounded", label: "Top 100" },
        { icon: "material-symbols:download-rounded", label: "Hors ligne" },
        { icon: "material-symbols:upload-file-outline-rounded", label: "OPML" },
      ],
      action: () => goto(podcastsEnabled() ? "/podcasts" : "/settings/online"),
    },
    {
      id: "podcast_listening",
      icon: "material-symbols:download-for-offline-rounded",
      colors: ["#6366f1", "#0ea5e9"],
      satellites: ["material-symbols:bolt-rounded", "material-symbols:replay-rounded", "material-symbols:queue-music-rounded"],
      action: () => goto(podcastsEnabled() ? "/podcasts/downloads" : "/settings/online"),
    },
    {
      id: "podcast_preview",
      icon: "material-symbols:star-rounded",
      colors: ["#f59e0b", "#e11d48"],
      satellites: ["material-symbols:visibility-outline-rounded", "material-symbols:more-horiz", "material-symbols:keep-rounded"],
      action: () => goto(podcastsEnabled() ? "/podcasts" : "/settings/online"),
    },
    {
      id: "sidebar_organize",
      icon: "material-symbols:swap-vert-rounded",
      colors: ["#10b981", "#0d9488"],
      satellites: ["material-symbols:drag-indicator", "material-symbols:queue-music-rounded", "material-symbols:favorite-rounded"],
      action: () => sidebarOrganize.set(true),
    },
  ],
  "0.4.2": [
    {
      id: "folder_covers",
      icon: "material-symbols:folder-rounded",
      colors: ["#f59e0b", "#d97706"],
      satellites: ["material-symbols:album", "material-symbols:photo-library-rounded", "material-symbols:high-quality-outline-rounded"],
      action: () => {
        const id = get(libraryStore).librarySelected?.id;
        if (id) goto(`/library/${id}/folders`);
      },
    },
    {
      id: "lastfm",
      icon: "simple-icons:lastdotfm",
      colors: ["#e11d48", "#f97316"],
      satellites: ["material-symbols:graphic-eq-rounded", "material-symbols:history-rounded", "material-symbols:public"],
      logos: [
        { icon: "simple-icons:lastdotfm", label: "Last.fm" },
        { icon: "material-symbols:graphic-eq-rounded", label: "ListenBrainz" },
      ],
      action: () => goto("/settings/online"),
    },
    {
      id: "m4a_tags",
      icon: "material-symbols:sell-outline-rounded",
      colors: ["#10b981", "#0ea5e9"],
      satellites: ["material-symbols:music-note-rounded", "material-symbols:image-outline-rounded", "material-symbols:verified-outline-rounded"],
    },
    {
      id: "playback_start",
      icon: "material-symbols:play-circle-outline-rounded",
      colors: ["#8b5cf6", "#0ea5e9"],
      satellites: ["material-symbols:terminal-rounded", "material-symbols:percent-rounded", "material-symbols:counter-1-outline-rounded"],
      action: () => goto("/settings/general"),
    },
  ],
};

export const slidesFor = (version: string): WhatsNewSlide[] => RELEASES[version] ?? [];
