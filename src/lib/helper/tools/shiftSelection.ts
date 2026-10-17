import { get } from "svelte/store";
import { selectionStore } from "#lib/stores/ui/selection.store";

/** Maj + clic étend la sélection multiple : sans ça, le navigateur surlignait aussi le texte entre les deux clics. */
export function preventShiftSelection(): () => void {
  const block = (e: MouseEvent) => {
    if (!e.shiftKey || !get(selectionStore).active) return;
    if ((e.target as Element | null)?.closest?.("input, textarea, [contenteditable]")) return;
    e.preventDefault();
  };
  document.addEventListener("mousedown", block, true);
  return () => document.removeEventListener("mousedown", block, true);
}
