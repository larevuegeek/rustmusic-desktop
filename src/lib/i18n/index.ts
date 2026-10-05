import { derived } from "svelte/store";
import { settingsStore } from "#lib/stores/settings/settings.store";

import fr from "./locales/fr.json";
import en from "./locales/en.json";
import es from "./locales/es.json";
import de from "./locales/de.json";
import it from "./locales/it.json";

type Translations = Record<string, any>;

const locales: Record<string, Translations> = { fr, en, es, de, it };

// Store réactif de la langue courante — dérivé du settings store
const currentLocale = derived(settingsStore, ($settings) => {
  return $settings.language || 'fr';
});

// Store des traductions courantes
const translations = derived(currentLocale, ($locale) => {
  return locales[$locale] || locales['fr'];
});

/**
 * Store réactif de traduction
 * Usage dans les templates : {$t('home.greeting_morning')}
 * Se met à jour automatiquement quand la langue change
 */
export const t = derived(translations, ($trans) => {
  return (key: string): string => {
    const keys = key.split('.');

    let result: any = $trans;
    for (const k of keys) {
      if (result && typeof result === 'object' && k in result) {
        result = result[k];
      } else {
        return key;
      }
    }

    return typeof result === 'string' ? result : key;
  };
});

export { currentLocale, translations };
