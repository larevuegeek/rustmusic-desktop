// Raccourcis clavier : touches par défaut, et celles que l'utilisateur a choisies (réglage `shortcuts`).

export type ShortcutAction =
  | "playPause" | "seekForward" | "seekBackward" | "next" | "previous"
  | "volumeUp" | "volumeDown" | "mute"
  | "search" | "toggleMini";

export type ShortcutGroup = "playback" | "volume" | "navigation";

/** Ordre d'affichage dans les réglages. */
export const SHORTCUT_ACTIONS: { action: ShortcutAction; labelKey: string; group: ShortcutGroup }[] = [
  { action: "playPause", labelKey: "settings.shortcut_play_pause", group: "playback" },
  { action: "seekForward", labelKey: "settings.shortcut_forward", group: "playback" },
  { action: "seekBackward", labelKey: "settings.shortcut_backward", group: "playback" },
  { action: "next", labelKey: "settings.shortcut_next", group: "playback" },
  { action: "previous", labelKey: "settings.shortcut_previous", group: "playback" },
  { action: "volumeUp", labelKey: "settings.shortcut_volume_up", group: "volume" },
  { action: "volumeDown", labelKey: "settings.shortcut_volume_down", group: "volume" },
  { action: "mute", labelKey: "settings.shortcut_mute", group: "volume" },
  { action: "search", labelKey: "settings.shortcut_search", group: "navigation" },
  { action: "toggleMini", labelKey: "settings.shortcut_toggle_mini", group: "navigation" },
];

// « Mod » : Ctrl sous Windows et Linux, ⌘ sous macOS.
export const DEFAULT_SHORTCUTS: Record<ShortcutAction, string[]> = {
  playPause: ["Space"],
  seekForward: ["ArrowRight"],
  seekBackward: ["ArrowLeft"],
  next: ["Mod+ArrowRight"],
  previous: ["Mod+ArrowLeft"],
  volumeUp: ["ArrowUp"],
  volumeDown: ["ArrowDown"],
  mute: ["M"],
  search: ["Mod+K", "Mod+F"],
  toggleMini: ["Mod+Shift+M"],
};

/** Touches qui gardent leur rôle : fermer, et passer d'un champ à l'autre. */
export const RESERVED_KEYS = ["Escape", "Tab"];

const MODIFIER_KEYS = ["Control", "Meta", "Alt", "Shift", "AltGraph", "CapsLock"];

/**
 * Touche lue d'après la disposition du clavier (`key`, pas `code`) : sur un AZERTY,
 * « M » est bien la touche M. `null` tant que seul un modificateur est enfoncé.
 */
export function eventToBinding(e: KeyboardEvent): string | null {
  if (MODIFIER_KEYS.includes(e.key)) return null;
  const key = e.key === " " ? "Space" : e.key.length === 1 ? e.key.toUpperCase() : e.key;
  const parts: string[] = [];
  if (e.ctrlKey || e.metaKey) parts.push("Mod");
  if (e.altKey) parts.push("Alt");
  if (e.shiftKey) parts.push("Shift");
  parts.push(key);
  return parts.join("+");
}

/** Seules les actions modifiées sont enregistrées ; le reste suit les touches par défaut. */
export function parseOverrides(json: string | undefined): Partial<Record<ShortcutAction, string[]>> {
  if (!json) return {};
  try {
    const parsed = JSON.parse(json);
    return parsed && typeof parsed === "object" ? parsed : {};
  } catch {
    return {};
  }
}

export function resolveShortcuts(json: string | undefined): Record<ShortcutAction, string[]> {
  return { ...DEFAULT_SHORTCUTS, ...parseOverrides(json) };
}

export function findAction(shortcuts: Record<ShortcutAction, string[]>, binding: string): ShortcutAction | undefined {
  return (Object.keys(shortcuts) as ShortcutAction[]).find((a) => shortcuts[a].includes(binding));
}
