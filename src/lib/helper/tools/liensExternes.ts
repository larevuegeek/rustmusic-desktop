import { openUrl } from "@tauri-apps/plugin-opener";

/** Un lien web ou e-mail s'ouvre dans le programme du système : suivi par la WebView, il remplaçait l'appli par une page d'erreur. */
export function ouvrirLiensExternes(): () => void {
  const clic = (e: MouseEvent) => {
    const lien = (e.target as Element | null)?.closest?.("a[href]") as HTMLAnchorElement | null;
    if (!lien || !/^(https?|mailto|tel):/i.test(lien.href)) return;
    // Les liens internes ont l'origine de l'appli (localhost en dev) : SvelteKit s'en charge.
    if (new URL(lien.href).origin === location.origin) return;
    e.preventDefault();
    openUrl(lien.href).catch((err) => console.error("[liens] ouverture impossible :", err));
  };
  document.addEventListener("click", clic, true);
  return () => document.removeEventListener("click", clic, true);
}
