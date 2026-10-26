// Menu clic droit du navigateur (Recharger, Inspecter…) : retiré partout, sauf dans les champs de saisie
// (copier, coller). Les menus de l'appli s'ouvrent comme avant. En développement, Maj + clic droit le garde.
export function preventNativeContextMenu(): () => void {
  const block = (e: MouseEvent) => {
    if (import.meta.env.DEV && e.shiftKey) return;
    if ((e.target as Element | null)?.closest?.("input, textarea, [contenteditable]")) return;
    e.preventDefault();
  };
  document.addEventListener("contextmenu", block);
  return () => document.removeEventListener("contextmenu", block);
}
