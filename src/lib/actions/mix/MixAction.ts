import { get } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import { profilSelector } from "#lib/stores/profil/profil.store";
import { handleTracksPlay } from "#lib/actions/queue/QueueAction";
import type { TrackListView } from "#lib/types/ui/library/track/TrackListView";
import { t, currentLocale } from "#lib/i18n";
import { popinStore } from "#lib/stores/ui/popin.store";
import { playlistStore } from "#lib/stores/playlist/playlist.store";
import { toasts } from "#lib/stores/ui/toast.store";
import { renouveler } from "#lib/stores/mix/mix.store";
import { PLAYLIST_COLORS } from "#lib/components/playlist/playlistConfig";
import SmartPlaylistPopin from "#lib/components/playlist/smart/SmartPlaylistPopin.svelte";
import type { ActionMix } from "#lib/components/mix/MixCard.svelte";
import type { SpecMix } from "#lib/config/mixes";

/** Le menu d'un mix : nouveau tirage, le garder ; et pour un mix créé, le modifier ou le supprimer. */
export function actionsMix(spec: SpecMix, libraryId: number): ActionMix[] {
  const tr = get(t);
  const actions: ActionMix[] = [
    { icone: "material-symbols:refresh-rounded", libelle: tr("mix_view.redraw"), agir: () => renouveler(libraryId, spec.cle) },
    { icone: "material-symbols:playlist-add-rounded", libelle: tr("mix_view.keep"), agir: (liste) => garder(spec, liste) },
  ];
  const p = spec.playlist;
  if (p) {
    actions.push(
      { icone: "material-symbols:tune-rounded", libelle: tr("mix_view.edit"), agir: () => modifierMix(p.id) },
      { icone: "material-symbols:delete-outline-rounded", libelle: tr("mix_view.delete"), danger: true, agir: () => supprimerMix(p.id, p.name) },
    );
  }
  return actions;
}

export const lancerMix = (liste: TrackListView[]) => handleTracksPlay(liste);

/** Fige le tirage du jour dans une vraie playlist. */
async function garder(spec: SpecMix, liste: TrackListView[]) {
  if (!liste.length) return;
  const tr = get(t);
  try {
    const profilId = get(profilSelector).profilSelected?.id;
    if (!profilId) throw new Error(tr("smart.no_profile"));
    const date = new Date().toLocaleDateString(get(currentLocale), { day: "numeric", month: "short" });
    const nom = tr("mix_view.keep_name").replace("{name}", spec.nom).replace("{date}", date);
    const playlist = await playlistStore.addPlaylist(profilId, nom, null, spec.playlist?.color ?? PLAYLIST_COLORS[0]);
    let n = 0;
    for (const piste of liste) {
      if (!piste.path) continue;
      try {
        await invoke("add_track_to_playlist", { playlistId: playlist.id, path: piste.path, libraryTrackId: piste.id });
        n++;
      } catch {
        // Doublon ou fichier disparu.
      }
    }
    await playlistStore.refresh();
    toasts.push({ type: "success", title: tr("mix_view.keep"), message: tr("mix_view.kept").replace("{n}", String(n)).replace("{name}", nom) });
  } catch (e) {
    toasts.push({ type: "error", title: tr("mix_view.keep"), message: String(e) });
  }
}

export function nouveauMix() {
  popinStore.open(get(t)("mix_view.new"), SmartPlaylistPopin, { mix: true }, { size: "xl", icon: "material-symbols:shuffle-rounded", flush: true });
}

function modifierMix(playlistId: number) {
  popinStore.open(get(t)("mix_view.edit"), SmartPlaylistPopin, { playlistId, mix: true }, { size: "xl", icon: "material-symbols:shuffle-rounded", flush: true });
}

async function supprimerMix(id: number, nom: string) {
  try {
    await playlistStore.removePlaylist(id);
    toasts.push({ type: "success", title: nom, message: get(t)("mix_view.deleted") });
  } catch (e) {
    toasts.push({ type: "error", title: nom, message: String(e) });
  }
}
