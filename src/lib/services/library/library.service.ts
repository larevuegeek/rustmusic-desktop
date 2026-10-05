import { invoke } from "@tauri-apps/api/core";
import { open } from '@tauri-apps/plugin-dialog';
import type { TrackListView } from "#lib/types/ui/library/track/TrackListView";
import type { AlbumListView } from "#lib/types/ui/library/album/AlbumListView";
import type { ArtistListView } from "#lib/types/ui/library/artist/ArtistListView";
import type { ArtistDetailView } from "#lib/types/ui/library/artist/ArtistDetailView";
import type { AlbumDetailView } from "#lib/types/ui/library/album/AlbumDetailView";
import type { TrackDetailView } from "#lib/types/ui/library/track/TrackDetailView";
import type { Library } from "#lib/types/db/library/Library";
import { toasts } from "#lib/stores/ui/toast.store";
import { goto } from "$app/navigation";
import { libraryStore } from "#lib/stores/library/library.store";
import { get } from "svelte/store";
import { t } from "#lib/i18n";

/** La boîte de dialogue seule : les fichiers choisis, ou rien si l'on annule. */
export async function choisirFichiers(): Promise<string[]> {
    const choix = await open({
        multiple: true,
        title: get(t)("actions.dialog_open_files"),
        filters: [
            {
                name: get(t)("actions.dialog_audio_files"),
                extensions: ['mp3', 'flac', 'ogg', 'm4a', 'dsf', 'dff', 'wav', 'aac']
            }
        ]
    });
    if (!choix) return [];
    return Array.isArray(choix) ? choix : [choix];
}

/** La boîte de dialogue seule : le dossier choisi, ou `null` si l'on annule. */
export async function choisirDossier(): Promise<string | null> {
    const choix = await open({
        directory: true,
        multiple: false,
        title: get(t)("actions.dialog_select_folder")
    });
    return typeof choix === "string" ? choix : null;
}

export async function importerFichiers(libraryId: number, files: string[]): Promise<TrackListView[]> {
    if (files.length === 0) return [];
    libraryStore.setImporting(true);
    try {
        return (await invoke<TrackListView[]>('add_files', { libraryId, files })) ?? [];
    } catch (err) {
        console.error(err);
        return [];
    } finally {
        libraryStore.setImporting(false);
    }
}

export async function importerDossier(libraryId: number, directory: string): Promise<TrackListView[]> {
    libraryStore.setImporting(true);
    try {
        return (await invoke<TrackListView[]>('add_directory', { libraryId, directory })) ?? [];
    } catch (err) {
        // Un import tourne déjà : l'appelant le signale.
        if (err === 'deja_en_cours') throw err;
        console.error(err);
        return [];
    } finally {
        libraryStore.setImporting(false);
    }
}

export default async function addLibraryFiles(libraryId: number): Promise<TrackListView[]> {
    return importerFichiers(libraryId, await choisirFichiers());
}

export async function addLibraryDirectory(libraryId: number): Promise<TrackListView[]> {
    const dossier = await choisirDossier();
    return dossier ? importerDossier(libraryId, dossier) : [];
}

export async function loadTrack(
    libraryTrackId: string
): Promise<TrackDetailView | null> {
    
    try {

        const result = await invoke<TrackDetailView>('get_track', { 
            libraryTrackId: libraryTrackId
        });

        return result;
        
    } catch(err) {
        console.error(err);
        return null;
    }
}

export async function loadTracksByAlbum(
    libraryId: number,
    libraryAlbumId: string,
): Promise<TrackListView[]> {
    
    try {

        const result = await invoke<TrackListView[]>('get_tracks_by_album', { 
            libraryId: libraryId,
            libraryAlbumId: libraryAlbumId
        });

        return result;
        
    } catch(err) {
        console.error(err);
        return [];
    }
}

export async function loadAlbum(
    libraryAlbumId: string
): Promise<AlbumDetailView | null> {
    
    try {

        const result = await invoke<AlbumDetailView>('get_album', { 
            libraryAlbumId: libraryAlbumId
        });

        return result;
        
    } catch(err) {
        console.error(err);
        return null;
    }
}

export async function loadAlbums(
    libraryId: number,
    missingCover: boolean = false
): Promise<AlbumListView[]> {
    
    try {

        const result = await invoke<AlbumListView[]>('get_albums', { 
            libraryId: libraryId,
            missingCover: missingCover
        });

        return result;
        
    } catch(err) {
        console.error(err);
        return [];
    }
}

export async function loadArtist(
    libraryArtistId: string
): Promise<ArtistDetailView | null> {
    
    try {
        const result = await invoke<ArtistDetailView>('get_artist', { 
            libraryArtistId: libraryArtistId
        });

        return result;
        
    } catch(err) {
        console.error(err);
        return null;
    }
}

export async function loadArtists(
    libraryId: number
): Promise<ArtistListView[]> {
    
    try {

        const result = await invoke<ArtistListView[]>('get_artists', { 
            libraryId: libraryId
        });

        return result;
        
    } catch(err) {
        console.error(err);
        return [];
    }
}


export async function loadLibrary(id: number, profilId: number, tag: number, currentTag: number): Promise<Library | null> {
    
    try {

       const result: Library | null = await invoke<Library | null>('get_library', { 
           libraryId: id,
       });

      if (tag !== currentTag) return null;

      if (!result || result.profil_id !== profilId) {
        toasts.push({
          type: "error",
          title: get(t)("notify.access_denied"),
          message: get(t)("notify.library_forbidden"),
        });
        goto("/");
        return null;
      }

      return result;

    } catch (e) {
      if (tag !== currentTag) return null;
      
      toasts.push({
          type: "error",
          title: get(t)("notify.access_failed"),
          message: get(t)("common.error_library"),
      });
      
      goto("/");
      return null;
    }
}