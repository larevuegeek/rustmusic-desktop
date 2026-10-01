/**
 * Préchargement des icônes Iconify pour le mode offline/prod.
 *
 * @iconify/svelte charge les icônes depuis l'API réseau par défaut ; en prod,
 * la CSP la bloque. Le plugin `scripts/vite-icones.js` extrait à la compilation
 * les seules icônes citées dans src/, enregistrées ici dans le cache Iconify.
 */
import { addCollection } from '@iconify/svelte';
import collections from 'virtual:icones';

for (const c of collections) addCollection(c);
