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
import { t, currentLocale } from "$lib/i18n";


async function handleTrack(path: string, contexte?: QueueTrack[]): Promise<QueueTrack> {
        // Pas de `stopPlay` ici : `playFile` et `preloadTrack` le font déjà.
        playerService.expectExplicitAction();

        // La liste d'où vient le clic devient la file : sinon rien à enchaîner
        // en fin de morceau.
        if (contexte && contexte.length > 1) {
            const track = await queueState.loadContext(contexte, path);
            if (track) return track;
        }

        return await queueState.loadTrack(path);
}


// ─── Une action à la fois, et seule la dernière compte ───
const DELAI_MAX_MS = 8000;

let lastAction = 0;
let prevAction: Promise<void> = Promise.resolve();

function addActionQueue(action: () => Promise<void>): Promise<void> {
    const numero = ++lastAction;

    prevAction = prevAction.then(async () => {
        // Une action plus récente est arrivée : celle-ci est caduque.
        if (numero !== lastAction) return;
        try {
            await Promise.race([
                action(),
                new Promise<void>((liberer) => setTimeout(liberer, DELAI_MAX_MS)),
            ]);
        } catch (err) {
            console.error("Erreur play_file", err);
        }
    });

    return prevAction;
}

export function handleSelectTrack(path: string, contexte?: QueueTrack[]): Promise<void> {
    return addActionQueue(async () => {
        // En mode double-clic, un simple clic désigne — il ne commande pas le
        // lecteur. Il coupait la lecture et vidait la file au passage.
        const etat = get(player).status;
        if (etat === "playing" || etat === "paused") return;

        const track = await handleTrack(path, contexte);
        await playerService.preloadTrack(track);
    });
}

// ─── Un geste, une lecture ───
//
// En mode « simple clic = lecture », un double-clic envoie `click, click,
// dblclick` : trois demandes de lecture pour un seul geste. La première est
// déjà partie quand les suivantes arrivent — la file d'actions n'annule que
// ce qui n'a pas commencé — et deux lectures finissaient par se marcher
// dessus : le son partait, l'état du lecteur restait celui de l'autre.
//
// Seule la lecture est concernée. La sélection doit rester libre, sinon le
// double-clic ne lancerait plus rien : son premier clic sélectionne.
export const DELAI_REPETITION_MS = 700;
let dernierePisteLue = "";
let dernierLancement = 0;

/** Vrai si cette demande n'est que l'écho de la précédente. */
export function estUneRepetition(
    path: string,
    pistePrecedente: string,
    instantPrecedent: number,
    maintenant: number,
): boolean {
    return path === pistePrecedente && maintenant - instantPrecedent < DELAI_REPETITION_MS;
}

export function handlePlayTrack(path: string, contexte?: QueueTrack[]): Promise<void> {
    const maintenant = Date.now();
    if (estUneRepetition(path, dernierePisteLue, dernierLancement, maintenant)) {
        return Promise.resolve();
    }
    dernierePisteLue = path;
    dernierLancement = maintenant;

    return addActionQueue(async () => {
        const track = await handleTrack(path, contexte);
        await playerService.playFile(track);
    });
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
            title: get(t)("home.open_file"),
            filters: [
            { name: get(t)("actions.dialog_audio_files"), extensions: ['mp3', 'flac', 'ogg', 'm4a', 'dsf', 'dff'] }
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
            title: get(t)("actions.dialog_select_folder")
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
            title: get(t)("notify.playlist_updated"),
            message: get(t)(audioFiles.length === 1 ? "notify.files_to_playlist_one" : "notify.files_to_playlist_n")
                .replace("{n}", audioFiles.length.toLocaleString(get(currentLocale)))
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
        title: audioFile.tags?.title ?? get(t)("common.unknown_title"),
        artist: audioFile.tags?.artist,
        duration: audioFile?.duration,
        cover: audioFile.tags?.attached_images?.[0]?.image_src,
        position: 0
    };
}


