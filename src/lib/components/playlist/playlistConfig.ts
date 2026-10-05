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
function icone(id: string, cle: string) {
  return { id, cle, get label() { return get(t)(cle); } };
}

export const PLAYLIST_ICONS = [
  icone('mynaui:music', 'playlist_icons.music'),
  icone('mynaui:heart', 'playlist_icons.heart'),
  icone('mynaui:star', 'playlist_icons.star'),
  icone('mynaui:fire', 'playlist_icons.fire'),
  icone('mynaui:moon', 'playlist_icons.moon'),
  icone('mynaui:sun', 'playlist_icons.sun'),
  icone('mynaui:lightning', 'playlist_icons.lightning'),
  icone('mynaui:cloud', 'playlist_icons.cloud'),
  icone('mynaui:coffee', 'playlist_icons.coffee'),
  icone('mynaui:leaf', 'playlist_icons.leaf'),
  icone('mynaui:gift', 'playlist_icons.gift'),
  icone('mynaui:headphones', 'playlist_icons.headphones'),
];
