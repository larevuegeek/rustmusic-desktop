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
/**
 * Mémorise un choix pour le script de démarrage de `app.html`.
 *
 * Celui-ci s'exécute avant tout : il ne peut pas interroger la base, qui
 * demande un aller-retour asynchrone. `localStorage` est synchrone et
 * disponible immédiatement — c'est le seul endroit où il peut lire.
 *
 * La base reste la source de vérité ; ceci n'en est qu'un reflet, rafraîchi à
 * chaque application, et sans conséquence s'il diverge.
 */
function remember(key: string, value: string) {
    try {
        localStorage.setItem(key, value);
    } catch {
        // Stockage indisponible : on perd seulement le confort du démarrage
        // sans clignotement, jamais le réglage lui-même.
    }
}

export function applyContrastMode(mode: ContrastMode) {
    remember("rustmusic:contrast", mode);
    const root = document.documentElement;
    if (mode === 'high') {
        root.setAttribute('data-contrast', 'high');
    } else {
        root.removeAttribute('data-contrast');
    }
}

/** Masque les favoris partout : sept boutons cœur et une entrée de menu. */
export function applyFavoritesMode(actif: boolean) {
    const root = document.documentElement;
    if (actif) {
        root.removeAttribute('data-favoris');
    } else {
        root.setAttribute('data-favoris', 'off');
    }
}

let mediaQueryListener: ((e: MediaQueryListEvent) => void) | null = null;

/**
 * Applique le thème :
 * - 'light' / 'dark' : force la classe correspondante
 * - 'auto' : suit la préférence système et écoute les changements en live
 */
export function applyThemeMode(mode: ThemeMode) {
    remember("rustmusic:theme", mode);
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
