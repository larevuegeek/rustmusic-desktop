import type { AppSettings } from "$lib/stores/settings/settings.store";

export type SettingsSection = {
  id: string;
  icon: string;
  labelKey: string;
  /** Sous-titre court, dans la navigation. */
  hintKey: string;
  /** Phrase d'en-tête de la page. */
  descKey: string;
  /** Réglages comptés par « N modifié(s) » et remis par « Rétablir ». */
  cles?: (keyof AppSettings)[];
};

export const settingsSections: SettingsSection[] = [
  {
    id: "general", icon: "material-symbols:tune-rounded",
    labelKey: "settings.general", hintKey: "settings.general_hint", descKey: "settings.general_desc",
    cles: [
      "auto_start", "minimize_to_tray", "scan_on_startup",
      "single_click_play", "resume_playback", "gapless",
      "system_media_controls", "show_notifications", "show_sleep_timer",
    ],
  },
  {
    id: "appearance", icon: "material-symbols:palette-outline-rounded",
    labelKey: "settings.appearance", hintKey: "settings.appearance_hint", descKey: "settings.appearance_desc",
    cles: [
      "theme", "contrast",
      "show_open_buttons", "show_pinned_albums", "show_pinned_artists",
      "show_liked_in_playlists", "show_recent_in_playlists", "show_recent_in_home", "show_favorites",
      "library_tabs_position", "library_tabs",
      "window_controls_style", "window_controls_position",
    ],
  },
  {
    id: "audio", icon: "material-symbols:speaker-outline-rounded",
    labelKey: "settings.audio", hintKey: "settings.audio_hint", descKey: "settings.audio_desc",
    cles: ["wasapi_exclusive", "dsd_dop"],
  },
  { id: "network", icon: "material-symbols:cast-rounded", labelKey: "settings.network", hintKey: "settings.network_hint", descKey: "settings.network_desc" },
  { id: "storage", icon: "material-symbols:database-outline-rounded", labelKey: "settings.storage", hintKey: "settings.storage_hint", descKey: "settings.storage_desc" },
  { id: "shortcuts", icon: "material-symbols:keyboard-outline-rounded", labelKey: "settings.shortcuts", hintKey: "settings.shortcuts_hint", descKey: "settings.shortcuts_desc" },
  { id: "about", icon: "material-symbols:info-outline-rounded", labelKey: "settings.about_short", hintKey: "settings.about_hint", descKey: "settings.about_desc" },
];
