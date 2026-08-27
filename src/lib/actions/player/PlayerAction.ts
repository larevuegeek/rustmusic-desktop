import { open } from '@tauri-apps/plugin-dialog';
import { player } from "$lib/stores/player/player.store";
import { invoke } from "@tauri-apps/api/core";
import { get } from "svelte/store";
import { queueState } from "$lib/stores/queue/queueState.store";
import type { AudioFile } from "$lib/types/db/audioFile/AudioFile";
import { thumbnail_getter } from "$lib/helper/tools/imgTools";
import { toLibraryCacheCreate } from "$lib/mapper/library/mapLibraryCache";
import type { QueueTrack } from "$lib/types/db/queue/QueueTrack";
import { profilSelector } from "$lib/stores/profil/profil.store";
import { toasts } from "$lib/stores/ui/toast.store";
import { playerService } from '$lib/services/player/player.service';


async function handleTrack(path: string): Promise<QueueTrack> {
        const playerStore = get(player);

        if(playerStore?.status == "playing" || playerStore?.status == "paused") {
            await playerService.stopPlay();
        }

        // La mutation qui suit fait réagir le synchroniseur de file. On lui
        // signale que l'appelant prendra la décision, sinon il lance une
        // lecture que `handleSelectTrack` s'empresse d'arrêter.
        playerService.expectExplicitAction();

        //On raz la queueState et ajouter ce fichier
        const track = await queueState.loadTrack(path);

        return track;
}


export async function handleSelectTrack(path: string) {
    try {

        const track = await handleTrack(path);

        playerService.preloadTrack(track);

    } catch (err) {
      console.error("Erreur play_file", err);
    }
}

export async function handlePlayTrack(path: string) {
    try {

        const track = await handleTrack(path);

        playerService.playFile(track);

    } catch (err) {
      console.error("Erreur play_file", err);
    }
}

export async function handleClickOpenFile() {
    await openAudioFile();
};

export async function handleClickOpenDirectory() {
    await openAudioDirectory();
};

export default async function openAudioFile(): Promise<void> {

    try {
        const selectedPath = await open({
            multiple: false,
            title: "Ouvrir un fichier",
            filters: [
            { name: 'Fichiers audio', extensions: ['mp3', 'flac', 'ogg', 'm4a', 'dsf', 'dff'] }
            ]
        });

        if(!selectedPath) return;

        const audioFile = await invoke('open_file', { path: selectedPath }) as AudioFile;

        if(!audioFile) return;
 
        ///////////////////// Gestion de la queue
        queueState.loadTrack(audioFile.path);
        //////////////////////////////////////////

        let thumbnailPath: string| undefined | null = await thumbnail_getter(selectedPath, audioFile);

        //On insert dans la librairy
        await invoke('create_library_cache', { payload: toLibraryCacheCreate(selectedPath, audioFile, thumbnailPath) });
        
    } catch(err) {
        console.error(err);
    }
}

export async function openAudioDirectory(): Promise<void> {

    try {
        const selectedPath = await open({
            directory: true,  // ← La clé importante !
            multiple: false,
            title: "Sélectionner un dossier"
        });

        if (!selectedPath) return;

        const audioFiles = await invoke('open_files', { directory: selectedPath }) as AudioFile[];
        if (audioFiles.length === 0) return;

        const [firstAudioFile, ...restAudioFile] = audioFiles;

        let currentFilePath = firstAudioFile.path;
        const firstThumbnail = await thumbnail_getter(currentFilePath, firstAudioFile);

        await invoke('create_library_cache', { payload: toLibraryCacheCreate(selectedPath, firstAudioFile, firstThumbnail) });

        ///////////////////// Gestion de la queue
        queueState.loadTrack(firstAudioFile.path);
        //////////////////////////////////////////

        await Promise.all(restAudioFile.map(async (audioFile) => {
            let currentFilePath = audioFile.path;

            const thumbnailPath = await thumbnail_getter(currentFilePath, audioFile);
            await invoke('create_library_cache', { payload: toLibraryCacheCreate(selectedPath, audioFile, thumbnailPath) });

            let track = audioFileToQueueTrack(audioFile);

            //rajouter dans la queue
            queueState.addTrack(track);
        }));

        toasts.push({
            type: "success",
            title: "Playlist mise à jour",
            message: `${audioFiles.length} fichier${audioFiles.length > 1 ? 's' : ''} ajouté${audioFiles.length > 1 ? 's' : ''} à la playlist`
        });

    } catch(err) {
        console.error(err);
    }
}

export function audioFileToQueueTrack(audioFile: AudioFile): QueueTrack {

    const profil = get(profilSelector);

    return {
        queueId: crypto.randomUUID(),
        profilId: profil.profilSelected?.id ?? 1,
        path: audioFile.path,
        title: audioFile.tags?.title ?? "Inconnu",
        artist: audioFile.tags?.artist,
        duration: audioFile?.duration,
        cover: audioFile.tags?.attached_images?.[0]?.image_src,
        position: 0
    };
}


