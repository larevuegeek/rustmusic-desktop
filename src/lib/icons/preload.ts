/**
 * Préchargement des icônes Iconify pour le mode offline/prod.
 *
 * @iconify/svelte charge les icônes depuis l'API réseau par défaut.
 * En prod (installeur .exe), pas de réseau = icônes manquantes.
 *
 * La solution : charger les packs JSON installés (@iconify-json/*)
 * et les enregistrer dans le cache Iconify via addCollection().
 * Ainsi les icônes sont disponibles instantanément, sans requête réseau.
 */
import { addCollection } from '@iconify/svelte';
import type { IconifyJSON } from '@iconify/types';

// Importer les données JSON de chaque pack utilisé
import lucide from '@iconify-json/lucide/icons.json';
import heroicons from '@iconify-json/heroicons/icons.json';
import ph from '@iconify-json/ph/icons.json';
import tabler from '@iconify-json/tabler/icons.json';
import mynaui from '@iconify-json/mynaui/icons.json';
import radixIcons from '@iconify-json/radix-icons/icons.json';
import uit from '@iconify-json/uit/icons.json';

// Les .icons.json sont typés `{}` par défaut par TS — on les caste en
// IconifyJSON puisqu'on connait leur forme.
addCollection(lucide as IconifyJSON);
addCollection(heroicons as IconifyJSON);
addCollection(ph as IconifyJSON);
addCollection(tabler as IconifyJSON);
addCollection(mynaui as IconifyJSON);
addCollection(radixIcons as IconifyJSON);
addCollection(uit as IconifyJSON);
