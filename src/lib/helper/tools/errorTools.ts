/** Message lisible d'une erreur quelconque (Error, chaîne renvoyée par une commande Tauri…). */
export function messageErreur(e: unknown): string {
  if (e instanceof Error) return e.message;
  if (typeof e === "object" && e && "message" in e) return String((e as { message: unknown }).message);
  return String(e ?? "");
}
