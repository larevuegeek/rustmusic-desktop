/**
 * Mode « mini-player » : réduit la fenêtre principale en un lecteur compact
 * toujours au premier plan (always-on-top). Bascule aller/retour en gardant
 * la taille et la position précédentes pour les restaurer.
 *
 * Le panneau déroulant (file d'attente / paroles) agrandit la fenêtre VERS LE
 * BAS avec une animation de hauteur (easeOutCubic) pour un effet fluide.
 */

import { writable, get } from "svelte/store";
import { currentMonitor, getCurrentWindow, LogicalSize, PhysicalPosition, PhysicalSize } from "@tauri-apps/api/window";

export const miniPlayerActive = writable(false);
/** Micro-lecteur : sous-mode du mini, la pochette seule dans un carré. */
export const microPlayerActive = writable(false);
/** Épinglé au-dessus des autres fenêtres (par défaut en entrant en mode mini). */
export const miniPinned = writable(true);

/** Largeur fixe (points logiques) + hauteur du panneau déroulé qd ouvert. */
const MINI_W = 380;
/** Hauteur de repli par défaut, remplacée par la mesure réelle du contenu. */
const MINI_H = 150;
/** Côté du micro-lecteur (points logiques) : de 220 à trois fois plus, retenu d'une fois sur l'autre. */
export const MICRO_MIN = 220;
export const MICRO_MAX = 660;
const MICRO_SIZE_KEY = "rustmusic:micro-size";
let microSize = (() => {
  try {
    const n = Number(localStorage.getItem(MICRO_SIZE_KEY));
    return n >= MICRO_MIN && n <= MICRO_MAX ? n : MICRO_MIN;
  } catch {
    return MICRO_MIN;
  }
})();
/** Hauteur ajoutée par le panneau (file d'attente / paroles) quand déroulé. */
const PANEL_H = 300;
/** Plancher du mode normal : en dessous, l'interface n'est plus utilisable. */
const NORMAL_MIN_W = 800;
const NORMAL_MIN_H = 560;
/** Taille de départ (tauri.conf.json), quand la taille gardée n'est pas fiable. */
const NORMAL_W = 1280;
const NORMAL_H = 900;

/** Taille, position et état agrandi d'avant le mode mini (pour restaurer). */
let savedSize: PhysicalSize | null = null;
let savedPos: PhysicalPosition | null = null;
let savedMaximized = false;
let savedFullscreen = false;
/** Hauteur logique courante (suivie pour animer depuis la bonne valeur). */
let currentH = MINI_H;
/** Hauteur réelle du contenu replié, mesurée par le composant (ResizeObserver). */
let collapsedH = MINI_H;
/** Panneau déroulé ou non (pour réajuster si le contenu change de taille). */
let expanded = false;
let resizeRaf = 0;
// Vrai une fois la fenêtre réduite, faux dès le début de la sortie : sans ça, le mini
// encore affiché remesurait sa hauteur pendant l'agrandissement et rétrécissait la fenêtre.
let tailleMini = false;
// Une bascule à la fois : un double clic capturait la taille réduite comme « taille normale ».
let bascule = false;

async function setH(h: number): Promise<void> {
  if (!tailleMini) return;
  currentH = h;
  try {
    await getCurrentWindow().setSize(new LogicalSize(MINI_W, h));
  } catch (e) {
    console.error("[mini-player] setSize failed:", e);
  }
}

/** Garde la fenêtre entière dans la zone de travail de l'écran : collée au bord, elle ne déborde plus en grandissant. */
async function keepOnScreen(width: number, height: number): Promise<void> {
  try {
    const win = getCurrentWindow();
    const monitor = await currentMonitor();
    if (!monitor) return;
    const scale = await win.scaleFactor();
    const pos = await win.outerPosition();
    const area = monitor.workArea;
    const maxX = area.position.x + area.size.width - Math.round(width * scale);
    const maxY = area.position.y + area.size.height - Math.round(height * scale);
    const x = Math.max(area.position.x, Math.min(pos.x, maxX));
    const y = Math.max(area.position.y, Math.min(pos.y, maxY));
    if (x !== pos.x || y !== pos.y) await win.setPosition(new PhysicalPosition(x, y));
  } catch (e) {
    console.error("[mini-player] keepOnScreen failed:", e);
  }
}

/** Quitte le plein écran et attend la fin de l'animation : macOS ignore les redimensionnements pendant. */
async function quitterPleinEcran(win: ReturnType<typeof getCurrentWindow>): Promise<void> {
  let fin: () => void = () => {};
  const termine = new Promise<void>((r) => { fin = r; });
  let calme: ReturnType<typeof setTimeout> | undefined;
  const plafond = setTimeout(fin, 2000);
  const arreter = await win.onResized(() => {
    clearTimeout(calme);
    calme = setTimeout(fin, 200);
  });
  try {
    await win.setFullscreen(false);
    await termine;
  } finally {
    clearTimeout(calme);
    clearTimeout(plafond);
    arreter();
  }
}

export async function enterMiniPlayer(): Promise<void> {
  if (get(miniPlayerActive) || bascule) return;
  bascule = true;
  try {
    const win = getCurrentWindow();
    savedFullscreen = await win.isFullscreen();
    if (savedFullscreen) await quitterPleinEcran(win);
    savedMaximized = await win.isMaximized();
    if (savedMaximized) await win.unmaximize();
    savedSize = await win.innerSize();
    savedPos = await win.outerPosition();
    await win.setMinSize(null);
    await win.setAlwaysOnTop(get(miniPinned));
    await win.setResizable(false);
    expanded = false;
    currentH = collapsedH;
    await win.setSize(new LogicalSize(MINI_W, collapsedH));
    await keepOnScreen(MINI_W, collapsedH);
    tailleMini = true;
    miniPlayerActive.set(true);
  } catch (e) {
    console.error("[mini-player] enter failed:", e);
  } finally {
    bascule = false;
  }
}

export async function exitMiniPlayer(): Promise<void> {
  if (!get(miniPlayerActive) || bascule) return;
  bascule = true;
  tailleMini = false;
  cancelAnimationFrame(resizeRaf);
  try {
    const win = getCurrentWindow();
    await win.setAlwaysOnTop(false);
    await win.setResizable(true);
    await win.setMinSize(new LogicalSize(NORMAL_MIN_W, NORMAL_MIN_H));
    // Une taille gardée trop petite (bascule ratée) ne se restaure pas : on repart de la taille d'origine.
    const avant = savedSize?.toLogical(await win.scaleFactor());
    const fiable = !!avant && avant.width >= NORMAL_MIN_W && avant.height >= NORMAL_MIN_H;
    await win.setSize(fiable ? savedSize! : new LogicalSize(NORMAL_W, NORMAL_H));
    if (fiable && savedPos) await win.setPosition(savedPos);
    else await win.center();
    if (savedMaximized) await win.maximize();
    if (savedFullscreen) await win.setFullscreen(true);
    microPlayerActive.set(false);
    miniPlayerActive.set(false);
  } catch (e) {
    console.error("[mini-player] exit failed:", e);
  } finally {
    bascule = false;
  }
}

/**
 * Au démarrage : plancher du mode normal, et taille d'origine si l'appli a été
 * fermée en mode mini (la taille réduite a alors été gardée pour la prochaine fois).
 */
export async function assurerTailleNormale(): Promise<void> {
  try {
    const win = getCurrentWindow();
    await win.setMinSize(new LogicalSize(NORMAL_MIN_W, NORMAL_MIN_H));
    const taille = (await win.innerSize()).toLogical(await win.scaleFactor());
    if (taille.width < NORMAL_MIN_W || taille.height < NORMAL_MIN_H) {
      await win.setSize(new LogicalSize(NORMAL_W, NORMAL_H));
      await win.center();
    }
  } catch (e) {
    console.error("[mini-player] normal size check failed:", e);
  }
}

/** Garde ou non le mini-lecteur au-dessus des autres fenêtres. */
export async function toggleMiniPin(): Promise<void> {
  const epingle = !get(miniPinned);
  try {
    await getCurrentWindow().setAlwaysOnTop(epingle);
    miniPinned.set(epingle);
  } catch (e) {
    console.error("[mini-player] setAlwaysOnTop failed:", e);
  }
}

/** Du mini au micro : la fenêtre devient un carré, le mini ne se remesure plus. */
export async function enterMicroPlayer(): Promise<void> {
  if (!get(miniPlayerActive) || get(microPlayerActive) || bascule) return;
  bascule = true;
  tailleMini = false;
  cancelAnimationFrame(resizeRaf);
  try {
    await getCurrentWindow().setSize(new LogicalSize(microSize, microSize));
    await keepOnScreen(microSize, microSize);
    microPlayerActive.set(true);
  } catch (e) {
    console.error("[micro-player] enter failed:", e);
  } finally {
    bascule = false;
  }
}

/** Redimensionne le micro en gardant le carré (poignée du coin), sans recaler pendant le geste. */
export async function resizeMicroPlayer(size: number): Promise<void> {
  if (!get(microPlayerActive)) return;
  microSize = Math.round(Math.max(MICRO_MIN, Math.min(MICRO_MAX, size)));
  try {
    await getCurrentWindow().setSize(new LogicalSize(microSize, microSize));
  } catch (e) {
    console.error("[micro-player] resize failed:", e);
  }
}

/** Fin du geste : la fenêtre revient dans l'écran et la taille est retenue. */
export async function endMicroResize(): Promise<void> {
  await keepOnScreen(microSize, microSize);
  try {
    localStorage.setItem(MICRO_SIZE_KEY, String(microSize));
  } catch {}
}

export function currentMicroSize(): number {
  return microSize;
}

/** Retour au mini, panneau replié. */
export async function exitMicroPlayer(): Promise<void> {
  if (!get(microPlayerActive) || bascule) return;
  bascule = true;
  try {
    expanded = false;
    currentH = collapsedH;
    await getCurrentWindow().setSize(new LogicalSize(MINI_W, collapsedH));
    await keepOnScreen(MINI_W, collapsedH);
    tailleMini = true;
    microPlayerActive.set(false);
  } catch (e) {
    console.error("[micro-player] exit failed:", e);
  } finally {
    bascule = false;
  }
}

export async function toggleMiniPlayer(): Promise<void> {
  if (get(miniPlayerActive)) await exitMiniPlayer();
  else await enterMiniPlayer();
}

/**
 * Anime la hauteur de la fenêtre mini de la valeur courante vers `to`
 * (easeOutCubic, ~220 ms). Donne l'effet de déroulement fluide du panneau.
 */
function animateMiniHeight(to: number): void {
  cancelAnimationFrame(resizeRaf);
  const from = currentH;
  if (from === to) return;
  const start = Date.now();
  const dur = 220;
  const step = () => {
    if (!tailleMini) return;
    const tt = Math.min(1, (Date.now() - start) / dur);
    const eased = 1 - Math.pow(1 - tt, 3); // easeOutCubic
    const h = Math.round(from + (to - from) * eased);
    void setH(h);
    if (tt < 1) resizeRaf = requestAnimationFrame(step);
  };
  step();
}

/** Ouvre (déroulé) ou ferme le panneau, avec animation. */
export function setMiniExpanded(exp: boolean): void {
  expanded = exp;
  const to = exp ? collapsedH + PANEL_H : collapsedH;
  // On remonte d'abord si le panneau dépasserait en bas de l'écran.
  if (exp) void keepOnScreen(MINI_W, to).then(() => animateMiniHeight(to));
  else animateMiniHeight(to);
}

/**
 * Signale la hauteur réelle du contenu replié (top bar + lecteur + progression
 * + onglets), mesurée par le composant. La fenêtre s'ajuste au pixel près :
 * ni bande vide en bas, ni onglets coupés. Sans effet si le panneau est ouvert.
 */
export function reportCollapsedHeight(h: number): void {
  const nh = Math.round(h);
  if (nh <= 0 || Math.abs(nh - collapsedH) < 1) return;
  collapsedH = nh;
  if (tailleMini && !expanded) void setH(collapsedH);
}
