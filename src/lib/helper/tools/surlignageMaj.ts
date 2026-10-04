import { get } from "svelte/store";
import { selectionStore } from "$lib/stores/ui/selection.store";

/** Maj + clic étend la sélection multiple : sans ça, le navigateur surlignait aussi le texte entre les deux clics. */
export function empecherSurlignageMaj(): () => void {
  const bloquer = (e: MouseEvent) => {
    if (!e.shiftKey || !get(selectionStore).active) return;
    if ((e.target as Element | null)?.closest?.("input, textarea, [contenteditable]")) return;
    e.preventDefault();
  };
  document.addEventListener("mousedown", bloquer, true);
  return () => document.removeEventListener("mousedown", bloquer, true);
}
