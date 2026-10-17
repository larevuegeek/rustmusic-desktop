import { get } from "svelte/store";
import { t } from "#lib/i18n";

export const PLAYLIST_COLORS = [
  '#8b5cf6',
  '#22c55e',
  '#0ea5e9',
  '#f59e0b',
  '#f43f5e',
  '#6366f1',
  '#ec4899',
  '#14b8a6',
  '#f97316',
  '#84cc16',
];

/** `cle` : clé i18n pour `$t` ; `label` la traduit à la lecture, pour les appelants non réactifs. */
function iconFor(id: string, cle: string) {
  return { id, cle, get label() { return get(t)(cle); } };
}

export const PLAYLIST_ICONS = [
  iconFor('mynaui:music', 'playlist_icons.music'),
  iconFor('mynaui:heart', 'playlist_icons.heart'),
  iconFor('mynaui:star', 'playlist_icons.star'),
  iconFor('mynaui:fire', 'playlist_icons.fire'),
  iconFor('mynaui:moon', 'playlist_icons.moon'),
  iconFor('mynaui:sun', 'playlist_icons.sun'),
  iconFor('mynaui:lightning', 'playlist_icons.lightning'),
  iconFor('mynaui:cloud', 'playlist_icons.cloud'),
  iconFor('mynaui:coffee', 'playlist_icons.coffee'),
  iconFor('mynaui:leaf', 'playlist_icons.leaf'),
  iconFor('mynaui:gift', 'playlist_icons.gift'),
  iconFor('mynaui:headphones', 'playlist_icons.headphones'),
];
