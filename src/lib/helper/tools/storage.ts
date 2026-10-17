// localStorage sans risque : indisponible (mode privé, quota), on retombe sur la valeur par défaut.
export function readLocal(cle: string, defaut: string): string;
export function readLocal(cle: string, defaut: string | null): string | null;
export function readLocal(cle: string, defaut: string | null): string | null {
  try {
    return localStorage.getItem(cle) ?? defaut;
  } catch {
    return defaut;
  }
}

export function writeLocal(cle: string, valeur: string) {
  try {
    localStorage.setItem(cle, valeur);
  } catch {
    // Préférence perdue au redémarrage, sans conséquence.
  }
}
