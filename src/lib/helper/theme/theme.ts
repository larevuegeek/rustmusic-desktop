export type ThemeMode = 'auto' | 'light' | 'dark';
export type ContrastMode = 'normal' | 'high';

/**
 * Applique le niveau de contraste en posant `data-contrast` sur <html>.
 *
 * Le rendu de l'interface repose beaucoup sur des couches très peu opaques
 * (`white/5`, `text-neutral-400`…) et des flous d'arrière-plan : élégant, mais
 * difficile à lire pour qui a une vue fatiguée, un écran très lumineux ou un
 * dalle peu contrastée. Le mode élevé neutralise ces effets — voir les règles
 * `[data-contrast="high"]` dans `app.css`.
 */
export function applyContrastMode(mode: ContrastMode) {
    const root = document.documentElement;
    if (mode === 'high') {
        root.setAttribute('data-contrast', 'high');
    } else {
        root.removeAttribute('data-contrast');
    }
}

let mediaQueryListener: ((e: MediaQueryListEvent) => void) | null = null;

/**
 * Applique le thème :
 * - 'light' / 'dark' : force la classe correspondante
 * - 'auto' : suit la préférence système et écoute les changements en live
 */
export function applyThemeMode(mode: ThemeMode) {
    cleanupAutoListener();

    if (mode === 'auto') {
        const mq = window.matchMedia('(prefers-color-scheme: dark)');
        applyDarkClass(mq.matches);
        mediaQueryListener = (e) => applyDarkClass(e.matches);
        mq.addEventListener('change', mediaQueryListener);
        return;
    }

    applyDarkClass(mode === 'dark');
}

function applyDarkClass(isDark: boolean) {
    document.documentElement.classList.toggle('dark', isDark);
}

function cleanupAutoListener() {
    if (mediaQueryListener) {
        const mq = window.matchMedia('(prefers-color-scheme: dark)');
        mq.removeEventListener('change', mediaQueryListener);
        mediaQueryListener = null;
    }
}

// Compat avec l'ancien code (SwitchTheme.svelte) — sera supprimé plus tard
export function toogleTheme(isDarkTheme: boolean): boolean {
    const next = !isDarkTheme;
    applyDarkClass(next);
    return next;
}

export function applyTheme(isDark: boolean) {
    applyDarkClass(isDark);
}
