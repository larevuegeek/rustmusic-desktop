// Nouveautés présentées après une mise à jour : quelques diapos par version.
// Textes dans `whats_new.slides.<id>` (titre, texte) des fichiers de langue.
import { goto } from "$app/navigation";
import { get } from "svelte/store";
import { settingsStore } from "#lib/stores/settings/settings.store";
import { sidebarOrganize } from "#lib/stores/ui/sidebarOrganize.store";

export type WhatsNewSlide = {
  id: string;
  /** Illustration : icône principale, dégradé, petites icônes autour. */
  icon: string;
  colors: [string, string];
  satellites?: string[];
  /** Capture ou image (dans `static/`) à la place de l'icône. */
  image?: string;
  /** Bouton « Essayer » : mène à la nouveauté. */
  action?: () => void;
};

const podcastsEnabled = () => get(settingsStore).podcasts === "true";

const RELEASES: Record<string, WhatsNewSlide[]> = {
  "0.3.2": [
    {
      id: "podcasts",
      icon: "material-symbols:podcasts-rounded",
      colors: ["#8b5cf6", "#c026d3"],
      satellites: ["material-symbols:search-rounded", "material-symbols:rss-feed-rounded", "material-symbols:trophy-outline-rounded"],
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
};

export const slidesFor = (version: string): WhatsNewSlide[] => RELEASES[version] ?? [];
