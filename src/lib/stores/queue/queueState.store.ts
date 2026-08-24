import { writable, get } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import type { QueueTrack } from "../../types/db/queue/QueueTrack";
import type { QueueState } from "$lib/types/db/queue/QueueState";
import { profilSelector } from "../profil/profil.store";
import { player } from "../player/player.store";
import type { AudioFile } from "$lib/types/db/audioFile/AudioFile";

// L'ID du profil par défaut (tu pourras le rendre dynamique plus tard)
let currentProfilId = 1;

// 2. L'ÉCOUTEUR AUTONOME : Dès que le profil change, on met à jour la variable
// ET on recharge la file d'attente automatiquement !
profilSelector.subscribe((profil) => {
    const newId = profil.profilSelected?.id ?? 1;
    
    // Si l'ID a vraiment changé (pour éviter de recharger pour rien au démarrage)
    if (newId !== currentProfilId) {
        currentProfilId = newId;
        queueState.init(); // Le store se met à jour tout seul comme un grand !
    }
});

// L'état de base quand la file est vide
const defaultState: QueueState = {
    currentIndex: -1,
    tracks: [],
    isShuffled: false,
    repeatMode: "off",
};

const queueStateWritable = writable<QueueState>(defaultState);

// Ordre original des pistes avant shuffle (null = pas de shuffle actif)
let originalTracks: QueueTrack[] | null = null;

// Fisher-Yates shuffle — mélange en place, O(n)
function shuffleArray<T>(arr: T[]): T[] {
    const a = [...arr];
    for (let i = a.length - 1; i > 0; i--) {
        const j = Math.floor(Math.random() * (i + 1));
        [a[i], a[j]] = [a[j], a[i]];
    }
    return a;
}

export const queueState = {
    subscribe: queueStateWritable.subscribe,

    // 1. Initialisation (à appeler dans le onMount de ton layout principal)
    init: async () => {
        try {
            // On demande à Rust de lire SQLite
            const stateFromDb = await invoke<QueueState>('get_queue', { profilId: currentProfilId });

            queueStateWritable.set(stateFromDb);
        } catch (err) {
            console.error("❌ Impossible de charger la file d'attente :", err);
            queueStateWritable.set(defaultState); // Fallback propre
        }
    },

    //2a Ajouter un morceau en début de liste
    addTrack: async (trackData: Omit<QueueTrack, 'queueId' | 'profilId' | 'position'>) => {
        const qs = get(queueStateWritable);
        
        // 1. On calcule l'index d'insertion (juste après la piste actuelle, ou 0 si rien n'est lu)
        const insertIndex = qs.currentIndex > -1 ? qs.currentIndex + 1 : 0;
        
        // 2. On prépare la nouvelle piste
        const newTrack: QueueTrack = {
            ...trackData,
            queueId: crypto.randomUUID(),
            profilId: currentProfilId,
            position: insertIndex,
        };

        // 3. La magie JS : on coupe le tableau en deux, on insère, et on décale les positions
        const newTracks = [
            ...qs.tracks.slice(0, insertIndex),
            newTrack,
            // Pour toutes les pistes qui suivent, on doit faire +1 sur leur position
            ...qs.tracks.slice(insertIndex).map(t => ({ ...t, position: t.position + 1 }))
        ];

        // 4. Mise à jour instantanée de l'interface Svelte
        queueStateWritable.update((state) => ({ ...state, tracks: newTracks }));

        // 5. Envoi à SQLite
        try {
            await invoke('replace_queue_tracks', { 
                profilId: currentProfilId, 
                payload: newTracks 
            });
        } catch (err) {
            console.error("❌ Erreur SQLite lors de l'insertion 'Lire ensuite' :", err);
        }
    },

    //2b Ajouter un morceau en fin de liste
    enqueue: async (trackData: Omit<QueueTrack, 'queueId' | 'profilId' | 'position'>) => {
        const currentState = get(queueStateWritable);
        const position = currentState.tracks.length; // Il se met à la fin
        
        const newTrack: QueueTrack = {
            ...trackData,
            queueId: crypto.randomUUID(), // L'ID unique pour Svelte et SQLite !
            profilId: currentProfilId,
            position,
        };

        // Mise à jour visuelle instantanée
        queueStateWritable.update((qs) => ({ ...qs, tracks: [...qs.tracks, newTrack] }));

        // Envoi silencieux à la base de données
        try {
            await invoke('add_queue_track', { profilId: currentProfilId, payload: newTrack });
        } catch (err) {
            console.error("❌ Erreur SQLite lors de l'ajout :", err);
        }
    },

    // 3. Supprimer un morceau par son ID unique
    removeTrack: async (queueId: string) => {
        queueStateWritable.update((qs) => {
            const newTracks = qs.tracks.filter((t) => t.queueId !== queueId);
            return { ...qs, tracks: newTracks };
        });

        try {
            await invoke('remove_queue_track', { profilId: currentProfilId, queueId });
        } catch (err) {
            console.error("❌ Erreur SQLite lors de la suppression :", err);
        }
    },
    // Réorganiser la file (Drag & Drop)
    reorderTracks: async (fromIndex: number, toIndex: number) => {
        const qs = get(queueStateWritable);

        // Sécurité de base
        if (fromIndex < 0 || fromIndex >= qs.tracks.length || toIndex < 0 || toIndex >= qs.tracks.length) {
            return; 
        }

        // 1. On mémorise l'ID de la piste en cours de lecture pour ne pas la perdre
        const playingTrackId = qs.currentIndex > -1 ? qs.tracks[qs.currentIndex].queueId : null;

        // 2. On clone le tableau et on déplace l'élément (La méthode JS classique)
        const newTracks = [...qs.tracks];
        const [movedTrack] = newTracks.splice(fromIndex, 1); // On retire la piste
        newTracks.splice(toIndex, 0, movedTrack);            // On l'insère à sa nouvelle place

        // 3. On met à jour la propriété 'position' de TOUTES les pistes
        const finalizedTracks = newTracks.map((t, index) => ({ ...t, position: index }));

        // 4. On retrouve le nouvel index de la piste en cours de lecture
        const newCurrentIndex = playingTrackId 
            ? finalizedTracks.findIndex(t => t.queueId === playingTrackId)
            : qs.currentIndex;

        // 5. Mise à jour instantanée de Svelte (Optimistic UI)
        queueStateWritable.update((state) => ({ 
            ...state, 
            tracks: finalizedTracks,
            currentIndex: newCurrentIndex
        }));

        // 6. Sauvegarde en tâche de fond
        try {
            // On écrase toute la liste dans SQLite
            await invoke('replace_queue_tracks', { 
                profilId: currentProfilId, 
                payload: finalizedTracks 
            });

            // Si l'index de lecture a bougé suite au drag & drop, on met à jour la DB !
            if (newCurrentIndex !== qs.currentIndex) {
                await invoke('update_queue_state_index', { 
                    profilId: currentProfilId, 
                    currentIndex: newCurrentIndex 
                });
            }
        } catch (err) {
            console.error("❌ Erreur SQLite lors du drag & drop :", err);
        }
    },
    // 4. Vider toute la file
    clear: async () => {
        queueStateWritable.set(defaultState);
        try {
            await invoke('clear_queue', { profilId: currentProfilId });
        } catch (err) {
            console.error("❌ Erreur SQLite lors du clear :", err);
        }
    },

    // 5. Gérer la lecture (Remplace ton ancien setNowPlaying)
    setCurrentIndex: async (index: number) => {
        queueStateWritable.update((qs) => ({ ...qs, currentIndex: index }));
        
        try {
            await invoke('update_queue_state_index', { profilId: currentProfilId, currentIndex: index });
        } catch (err) {
            console.error("❌ Erreur SQLite lors de la mise à jour de l'index :", err);
        }
    },
    next: async () => {
        const qs = get(queueStateWritable);
        const total = qs.tracks.length;

        if (total === 0) return;

        let nextIndex;

        if (qs.repeatMode === "one") {
            nextIndex = qs.currentIndex;
        } else {
            // Shuffle ou pas, on avance simplement de +1
            // (en shuffle la liste est déjà mélangée)
            nextIndex = qs.currentIndex + 1;

            if (nextIndex >= total) {
                if (qs.repeatMode === "all") {
                    nextIndex = 0;
                } else {
                    return;
                }
            }
        }

        await queueState.setCurrentIndex(nextIndex);
    },
    previous: async () => {
        const qs = get(queueStateWritable);
        const total = qs.tracks.length;

        if (total === 0) return;

        // Shuffle ou pas, on recule simplement de -1
        // (en shuffle la liste est déjà mélangée)
        let prevIndex = qs.currentIndex - 1;

        if (prevIndex < 0) {
            if (qs.repeatMode === "all") {
                prevIndex = total - 1;
            } else {
                prevIndex = 0;
            }
        }

        await queueState.setCurrentIndex(prevIndex);
    },
    // --- Les setters classiques (seulement en JS pour l'instant) ---
    setIsShuffled: async (isShuffled: boolean) => {
        const qs = get(queueStateWritable);
        const currentTrackId = qs.currentIndex > -1 ? qs.tracks[qs.currentIndex]?.queueId : null;

        if (isShuffled) {
            // ON → sauvegarder l'ordre original, mélanger, garder la piste en cours en index 0
            originalTracks = [...qs.tracks];

            let shuffled = shuffleArray(qs.tracks);

            // Placer la piste en cours en première position
            if (currentTrackId) {
                const idx = shuffled.findIndex((t: QueueTrack) => t.queueId === currentTrackId);
                if (idx > 0) {
                    [shuffled[0], shuffled[idx]] = [shuffled[idx], shuffled[0]];
                }
            }

            // Recalculer les positions
            shuffled = shuffled.map((t: QueueTrack, i: number) => ({ ...t, position: i }));

            queueStateWritable.update((s) => ({
                ...s,
                tracks: shuffled,
                currentIndex: 0,
                isShuffled: true,
            }));

            try {
                await invoke('replace_queue_tracks', { profilId: currentProfilId, payload: shuffled });
                await invoke('update_queue_state_index', { profilId: currentProfilId, currentIndex: 0 });
                await invoke('update_queue_state_shuffled', { profilId: currentProfilId, isShuffled: true });
            } catch (err) {
                console.error("❌ Erreur SQLite shuffle ON :", err);
            }
        } else {
            // OFF → restaurer l'ordre original
            if (originalTracks) {
                const restored = originalTracks.map((t: QueueTrack, i: number) => ({ ...t, position: i }));
                const newIndex = currentTrackId
                    ? restored.findIndex((t: QueueTrack) => t.queueId === currentTrackId)
                    : 0;

                queueStateWritable.update((s) => ({
                    ...s,
                    tracks: restored,
                    currentIndex: Math.max(newIndex, 0),
                    isShuffled: false,
                }));

                try {
                    await invoke('replace_queue_tracks', { profilId: currentProfilId, payload: restored });
                    await invoke('update_queue_state_index', { profilId: currentProfilId, currentIndex: Math.max(newIndex, 0) });
                    await invoke('update_queue_state_shuffled', { profilId: currentProfilId, isShuffled: false });
                } catch (err) {
                    console.error("❌ Erreur SQLite shuffle OFF :", err);
                }

                originalTracks = null;
            } else {
                // Pas d'original sauvegardé, on désactive juste le flag
                queueStateWritable.update((s) => ({ ...s, isShuffled: false }));
                try {
                    await invoke('update_queue_state_shuffled', { profilId: currentProfilId, isShuffled: false });
                } catch (err) {
                    console.error("❌ Erreur SQLite shuffle OFF :", err);
                }
            }
        }
    },
    setRepeatMode: async (repeatMode: QueueState["repeatMode"]) => {
        queueStateWritable.update((qs) => ({ ...qs, repeatMode }))

        try {
            await invoke('update_queue_state_repeat_mode', { profilId: currentProfilId, repeatMode });
        } catch (err) {
            console.error("❌ Erreur SQLite lors de la mise à jour du repeat mode :", err);
        }
    },
    getTrackById: (queueId: string): QueueTrack | undefined => {
        return get(queueStateWritable).tracks.find(track => track.queueId === queueId);
    },
    loadTrack: async (path: string) => {

        const audioFile = await invoke('open_file', { path: path }) as AudioFile;

        const newTrack: QueueTrack = {
            path,
            queueId: crypto.randomUUID(),
            profilId: currentProfilId,
            position: 0,
            title: audioFile.tags.title as string,
            artist: audioFile.tags.artist as string,
            duration: audioFile.duration as number,
            // `?.[0]?.` et non `?.[0].` : le premier point d'interrogation ne
            // protège que l'absence du tableau. Sur une liste **vide** — un
            // morceau sans pochette intégrée, cas courant quand l'illustration
            // est un `folder.jpg` à côté — `[0]` rend `undefined`, et lire
            // `.image_src` dessus lève une exception.
            //
            // Elle remontait jusqu'au `catch` de `handlePlayTrack`, qui
            // l'écrivait dans la console et n'en faisait rien : cliquer sur un
            // tel morceau ne lançait donc **rien du tout**, sans le moindre
            // message. Lancer un album entier passait, lui, par un autre
            // chemin — d'où l'impression que seuls les albums étaient jouables.
            cover: audioFile.tags.attached_images?.[0]?.image_src ?? undefined,
        };

        // 1. On met à jour l'état (le UI va s'actualiser direct)
        queueStateWritable.set({
            ...get(queueStateWritable),
            tracks: [newTrack],
            currentIndex: 0, 
        });

        // 2. Sauvegarde SQLite en fond
        try {
            await invoke('replace_queue_tracks', { profilId: currentProfilId, payload: [newTrack] });
            await invoke('update_queue_state_index', { profilId: currentProfilId, currentIndex: 0 });
        } catch (err) {
            console.error("❌ Erreur SQLite lors du chargement :", err);
        }

        player.update({ pathFile: newTrack.path, audioFile: audioFile, trackId: newTrack.queueId });

        return newTrack;
    },
    loadTracks: async (tracks: QueueTrack[]) => {

        if (tracks.length == 0) return;

        // 1. Mettre à jour l'état — le subscribe de initQueueSync va détecter
        //    le changement de track et lancer la lecture automatiquement
        queueStateWritable.set({
            ...get(queueStateWritable),
            tracks: tracks,
            currentIndex: 0,
        });

        // 2. Sauvegarde SQLite en fond
        try {
            await invoke('replace_queue_tracks', { profilId: currentProfilId, payload: tracks });
            await invoke('update_queue_state_index', { profilId: currentProfilId, currentIndex: 0 });
        } catch (err) {
            console.error("❌ Erreur SQLite lors du chargement :", err);
        }

        return tracks[0].queueId;
    },
};