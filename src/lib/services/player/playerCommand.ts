// Commandes de lecture venues de l'extérieur : touches média de l'OS, ligne de commande (`/play`…).
import { playerService } from "#lib/services/player/player.service";

export function runPlayerCommand(cmd: string) {
  switch (cmd) {
    // Un bureau peut envoyer « pause » au verrouillage ou à la veille : une bascule relançait alors la musique.
    case "play":
      if (!playerService.isPlaying()) playerService.handleTogglePlay();
      break;
    case "pause":
      if (playerService.isPlaying()) playerService.pauseFile();
      break;
    case "toggle":
      playerService.handleTogglePlay();
      break;
    case "next":
      playerService.nextTrack();
      break;
    case "previous":
      playerService.prevTrack();
      break;
    case "stop":
      playerService.stopPlay();
      break;
    // Le reste (« quit », « raise »…) n'est pas une commande de lecture.
  }
}
