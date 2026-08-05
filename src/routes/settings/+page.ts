import { redirect } from '@sveltejs/kit';

// /settings n'a pas de contenu propre : chaque section est une sous-route.
export function load() {
  redirect(307, '/settings/general');
}
