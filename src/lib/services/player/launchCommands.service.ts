// Démarrage : commandes de la ligne de commande (`RustMusic /play`), sinon l'option « Lancer la lecture au démarrage ».
// L'appli déjà ouverte reçoit celles d'un nouveau lancement par l'événement `cli-command`.
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { get } from "svelte/store";
import { queueState } from "#lib/stores/queue/queueState.store";
import { settingsStore } from "#lib/stores/settings/settings.store";
import { runPlayerCommand } from "#lib/services/player/playerCommand";

export async function initLaunchCommands() {
  await listen<string>("cli-command", (event) => runPlayerCommand(event.payload));

  const commands = await invoke<string[]>("take_launch_commands").catch(() => [] as string[]);
  // Une commande explicite (`/pause`, `/stop`…) passe avant l'option.
  if (commands.length > 0) {
    for (const cmd of commands) runPlayerCommand(cmd);
  } else if (settingsStore.get("play_on_start") === "true" && get(queueState).currentIndex > -1) {
    runPlayerCommand("play");
  }
}
