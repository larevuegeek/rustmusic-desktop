/** Suit `lire` avec `ms` de retard, pour ne pas filtrer à chaque frappe ; vidée, elle suit aussitôt. */
export function differe(lire: () => string, ms = 150) {
  let valeur = $state(lire());
  $effect(() => {
    const v = lire();
    if (!v.trim()) { valeur = v; return; }
    const id = setTimeout(() => (valeur = v), ms);
    return () => clearTimeout(id);
  });
  return { get valeur() { return valeur; } };
}
