import type { UnlistenFn } from "@tauri-apps/api/event";
import { listen } from "@tauri-apps/api/event";
import { player } from "$lib/stores/player/player.store";
import { invoke } from "@tauri-apps/api/core";
import { get } from "svelte/store";
import type { QueueTrack } from "$lib/types/db/queue/QueueTrack";
import type { AudioFile } from "$lib/types/db/audioFile/AudioFile";
import { recent } from "$lib/stores/recent/recent.store";
import { queueState } from "$lib/stores/queue/queueState.store";
import { settingsStore } from "$lib/stores/settings/settings.store";
import { sleepTimer } from "$lib/stores/player/sleepTimer.store";
import { sendNotification, isPermissionGranted, requestPermission } from "@tauri-apps/plugin-notification";

let raf: number = 0;
let basePos = 0;
let baseTime = 0;
let playRequestId = 0;
let timer: ReturnType<typeof setTimeout> | null = null;
let seekPending = false;
let trackPlaybackEnded: UnlistenFn | null = null;
let preparingUnlisten: UnlistenFn | null = null;
let queueUnsubscribe: (() => void) | null = null;
let trackAdvancedUnlisten: UnlistenFn | null = null;
let sleepUnsubscribe: (() => void) | null = null;
let playStartedAt = 0; // timestamp (Date.now()) au moment où CPAL démarre vraiment

// Piste annoncée au backend pour le préchargement (lecture sans blanc).
// On mémorise l'index en plus du morceau : à la promotion, c'est lui qui
// permet d'avancer la file sans avoir à rechercher par chemin (un même
// fichier peut figurer plusieurs fois dans la file).
let announcedNext: { track: QueueTrack; index: number } | null = null;
let lastAnnouncedPath: string | null = null;

let initialized = false;

class PlayerService {

    // ==========================================
    // 🚀 INIT GLOBAL (à appeler UNE seule fois)
    // ==========================================
    async init() {
        if (initialized) return;
        initialized = true;

        await this.initPlaybackListener();
        await this.initPreparingListener();
        await this.initTrackAdvancedListener();
        this.initQueueSync();

        // La minuterie « fin du morceau » doit empêcher tout préchargement :
        // on réévalue donc l'annonce à chaque changement de son état.
        sleepUnsubscribe = sleepTimer.subscribe(() => this.syncNextTrack());

        // Sleep timer : à échéance (mode durée), on met en pause la lecture.
        sleepTimer.setOnFire(() => {
            const status = get(player).status;
            if (status === "playing") this.pauseFile();
        });
    }

    destroy() {
        if (trackPlaybackEnded) trackPlaybackEnded();
        if (preparingUnlisten) preparingUnlisten();
        if (trackAdvancedUnlisten) trackAdvancedUnlisten();
        if (queueUnsubscribe) queueUnsubscribe();
        if (sleepUnsubscribe) sleepUnsubscribe();
        this.stopAnimation();
        if (timer) clearTimeout(timer);
        initialized = false;
    }

    // ==========================================
    // 🔄 LISTENER PRE-DECODE (profil Minimal)
    // ==========================================
    // Le backend émet `playback-preparing: true` au début du pré-décodage et
    // `false` quand CPAL démarre vraiment. Pendant le pré-décodage, le frontend
    // doit FIGER la progress bar (l'animation JS avance dans le vide sinon).
    private async initPreparingListener() {
        preparingUnlisten = await listen<boolean>("playback-preparing", (e) => {
            const preparing = e.payload === true;
            player.update({
                isPreparing: preparing,
                // À la fin du pre-decode, reset les positions à 0 pour que la
                // lecture redémarre proprement (le backend démarre à 0 aussi).
                ...(preparing ? {} : { jsPosition: 0, rustPosition: 0 }),
            });
            if (preparing) {
                // Resync de la base de l'animation au cas où elle tournait déjà.
                basePos = 0;
                baseTime = Date.now();
            } else {
                // CPAL démarre vraiment maintenant : mémoriser l'instant pour
                // détecter les "playback-ended" trop rapides (= bug driver).
                playStartedAt = Date.now();
            }
        });
    }

    // ==========================================
    // 🎧 LISTENER FIN DE LECTURE
    // ==========================================
    private async initPlaybackListener() {
        trackPlaybackEnded = await listen("playback-ended", () => {
            this.stopAnimation();
            if (timer) clearTimeout(timer);

            // Garde-fou anti boucle : si le morceau "se termine" en moins de
            // 3 secondes, c'est probablement un problème CPAL (driver audio
            // qui crash, buffer mal configuré...) plutôt qu'une vraie fin de
            // lecture. On stoppe au lieu de relancer en boucle infinie.
            const elapsed = (Date.now() - playStartedAt) / 1000;
            if (playStartedAt > 0 && elapsed < 3) {
                console.warn(
                    `⚠️ playback-ended trop rapide (${elapsed.toFixed(1)}s) — ` +
                    "stream CPAL probablement KO. Stop pour éviter une boucle."
                );
                player.update({ status: "ended", isPreparing: false });
                playStartedAt = 0;
                return;
            }
            playStartedAt = 0;

            // Sleep timer « fin de piste » : on stoppe au lieu d'enchaîner.
            if (sleepTimer.consumeEndOfTrack()) {
                this.stopPlay();
                return;
            }

            const state = get(queueState);
            if (state.repeatMode === "one") {
                // Repeat One : relancer le même morceau directement
                const track = state.tracks[state.currentIndex];
                if (track) this.playFile(track);
            } else {
                queueState.next();
            }
        });
    }

    // ==========================================
    // 🔗 REMPLACE TON $effect()
    // ==========================================
    private initQueueSync() {
        queueUnsubscribe = queueState.subscribe(state => {

            const track = state.tracks[state.currentIndex] ?? null;
            const currentPlayer = get(player);
            const currentPlayerTrackId = currentPlayer.trackId;

            // Toute mutation de la file (réordonnancement, ajout, retrait,
            // shuffle, repeat…) passe ici : c'est le point unique où l'on
            // réévalue « quelle est la suite ? » pour le préchargement.
            this.syncNextTrack();

            if (!track) {
                this.stopPlay();
                return;
            }

            // Premier chargement → preload seulement
            if (!currentPlayerTrackId) {
                this.preloadTrack(track);
                return;
            }

            // Le track a changé (next/previous/shuffle) → lancer la lecture
            if (track.queueId !== currentPlayerTrackId) {
                this.playFile(track);
            }
        });
    }

    // ==========================================
    // 🔗 LECTURE SANS BLANC — annonce de la suite
    // ==========================================
    // Calcule la piste qui suivra, avec exactement la même règle que
    // `queueState.next()`, et l'annonce au backend pour qu'il la précharge.
    // Toute annonce différente invalide le préchargement en cours côté Rust.
    private syncNextTrack() {
        const qs = get(queueState);
        let candidate: { track: QueueTrack; index: number } | null = null;

        // Minuterie « fin du morceau » : il ne doit rien y avoir après.
        const sleepMode = get(sleepTimer).mode;

        if (sleepMode !== "end-of-track" && qs.tracks.length > 0 && qs.currentIndex >= 0) {
            let index: number;

            if (qs.repeatMode === "one") {
                index = qs.currentIndex;
            } else {
                index = qs.currentIndex + 1;
                if (index >= qs.tracks.length) {
                    index = qs.repeatMode === "all" ? 0 : -1;
                }
            }

            const next = index >= 0 ? qs.tracks[index] : null;
            if (next) candidate = { track: next, index };
        }

        // Toujours rafraîchir la référence locale : l'index peut changer même
        // quand le chemin ne bouge pas (réordonnancement de la file).
        announcedNext = candidate;

        const path = candidate?.track.path ?? null;
        if (path === lastAnnouncedPath) return;
        lastAnnouncedPath = path;

        invoke("set_next_track", { path }).catch((e) =>
            console.warn("[gapless] set_next_track:", e)
        );
    }

    // ==========================================
    // 🔗 LECTURE SANS BLANC — la piste a été promue
    // ==========================================
    // Le backend a enchaîné sans interrompre le flux audio. Le frontend doit
    // donc se resynchroniser SANS relancer la lecture.
    private async initTrackAdvancedListener() {
        trackAdvancedUnlisten = await listen<string>("track-advanced", async (e) => {
            const advanced = announcedNext;
            announcedNext = null;
            lastAnnouncedPath = null;

            // Garde-fou : si ce qui a été promu ne correspond pas à ce qu'on
            // avait annoncé, on ne touche à rien — le flux normal
            // (`playback-ended`) reprendra la main en fin de morceau.
            if (!advanced || advanced.track.path !== e.payload) {
                console.warn("[gapless] track-advanced inattendu :", e.payload);
                return;
            }

            const track = advanced.track;

            try {
                const audioFile = await invoke("open_file", { path: track.path }) as AudioFile;

                // ⚠️ ORDRE CRITIQUE : le player d'abord, la file ensuite.
                // `initQueueSync` relance `playFile` quand le queueId de la
                // file diffère de celui du player ; en mettant le player à
                // jour en premier, les deux coïncident déjà quand la
                // souscription se déclenche, et la lecture n'est pas coupée.
                player.update({
                    status: "playing",
                    pathFile: track.path,
                    audioFile,
                    trackId: track.queueId,
                    jsPosition: 0,
                    rustPosition: 0,
                    isPreparing: false,
                });

                // Rebase l'animation de la barre de progression : le backend a
                // remis sa position à zéro pour la nouvelle piste.
                basePos = 0;
                baseTime = Date.now();
                playStartedAt = Date.now();

                await queueState.setCurrentIndex(advanced.index);

                // Historique et notification, comme pour une lecture normale.
                const libraryCacheId = await invoke<number | null>(
                    "get_library_cache_id_by_path",
                    { path: track.path }
                );
                await invoke<void>("insert_recent_file", {
                    path: track.path,
                    libraryId: libraryCacheId,
                });
                recent.refreshRecent();
                this.sendTrackNotification(track, audioFile);
            } catch (err) {
                console.error("[gapless] resynchronisation après enchaînement :", err);
            }

            // Annonce de la piste d'après, pour rester en avance.
            this.syncNextTrack();
        });
    }

    // ==========================================
    // 📦 PRELOAD (conservé)
    // ==========================================
    async preloadTrack(track: QueueTrack) {

        const currentRequestId = ++playRequestId;

        try {
            await this.stopPlay();

            const audioFile = await invoke("open_file", { path: track.path }) as AudioFile;

            if (currentRequestId !== playRequestId) return;

            player.update({
                status: "idle",
                pathFile: track.path,
                audioFile,
                trackId: track.queueId
            });

        } catch (err) {
            console.error("Erreur preload:", err);
        }
    }

    // ==========================================
    // ▶ PLAY
    // ==========================================
    async playFile(track: QueueTrack) {

        const currentRequestId = ++playRequestId;

        try {
            await this.stopPlay();

            const audioFile = await invoke("open_file", { path: track.path }) as AudioFile;

            if (currentRequestId !== playRequestId) return;

            player.update({
                status: "playing",
                pathFile: track.path,
                audioFile,
                trackId: track.queueId
            });

            await invoke("play_file", { path: track.path });

            // Si pas de mode Minimal/pre-decode, on considère que la lecture
            // commence immédiatement (sera écrasé par le preparing listener
            // pour les profils Low/Minimal).
            playStartedAt = Date.now();

            this.runPosition();
            this.startAnimation();

            // Gestion récents
            const libraryCacheId = await invoke<number | null>(
                "get_library_cache_id_by_path",
                { path: track.path }
            );

            await invoke<void>("insert_recent_file", {
                path: track.path,
                libraryId: libraryCacheId
            });

            recent.refreshRecent();

            // Notification système si activée dans les paramètres
            this.sendTrackNotification(track, audioFile);

        } catch (err) {
            console.error("Erreur lecture:", err);
        }
    }

    // ==========================================
    // 🔔 NOTIFICATION
    // ==========================================
    // Envoie une notification système quand un nouveau morceau commence
    // Vérifie d'abord que les notifications sont activées dans les paramètres
    // et que l'OS a accordé la permission
    private async sendTrackNotification(track: QueueTrack, audioFile: AudioFile) {
        try {
            const notifEnabled = settingsStore.get('show_notifications');
            if (notifEnabled !== 'true') return;

            const title = audioFile.tags?.title ?? track.title ?? 'Titre inconnu';
            const artist = audioFile.tags?.artist ?? track.artist ?? 'Artiste inconnu';
            const album = audioFile.tags?.album ?? '';
            const body = album ? `${artist} — ${album}` : artist;

            // Essayer d'abord le plugin Tauri (fonctionne en prod)
            try {
                let permitted = await isPermissionGranted();
                if (!permitted) {
                    const permission = await requestPermission();
                    permitted = permission === 'granted';
                }
                if (permitted) {
                    sendNotification({ title, body });
                    return;
                }
            } catch {
                // Plugin Tauri indisponible (dev mode) — fallback Web API
            }

            // Fallback : Web Notification API (fonctionne en dev + prod)
            if ('Notification' in window) {
                if (Notification.permission === 'default') {
                    await Notification.requestPermission();
                }
                if (Notification.permission === 'granted') {
                    new Notification(title, { body, silent: true });
                }
            }
        } catch (e) {
            // Silencieux — les notifications ne sont pas critiques
        }
    }

    // ==========================================
    // ⏹ STOP
    // ==========================================
    async stopPlay() {
        try {
            await invoke("stop_play");

            player.update({
                status: "idle",
                rustPosition: 0,
                jsPosition: 0
            });

            if (timer) clearTimeout(timer);
            timer = null;

            this.stopAnimation();

        } catch (err) {
            console.error(err);
        }
    }

    // ==========================================
    // ⏩ SEEK
    // ==========================================
    async seekTo(positionSeconds: number) {
        await invoke("seek_to", { position: positionSeconds });
        player.update({ jsPosition: positionSeconds, rustPosition: positionSeconds });
        basePos = positionSeconds;
        baseTime = Date.now();
        seekPending = true; // Ignorer les prochaines rustPosition stale
    }

    // ==========================================
    // ⏯ TOGGLE
    // ==========================================
    async handleTogglePlay() {

        const queue = get(queueState);
        const currentTrack = queue.tracks[queue.currentIndex] ?? null;
        if (!currentTrack) return;

        const status = get(player).status;

        if (status === "playing") {
            await this.pauseFile();
        } else if (status === "paused") {
            await this.resumePlay();
        } else {
            await this.playFile(currentTrack);
        }
    }

    async resumePlay() {
        try {
            await invoke("pause_play");
            player.update({ status: "playing" });
            this.runPosition();
            this.startAnimation();
        } catch (err) {
            console.error("Resume failed", err);
        }
    }

    // ==========================================
    // ⏸ PAUSE
    // ==========================================
    async pauseFile() {
        try {
            await invoke("pause_play");

            player.update({status: "paused" });

            if (timer) clearTimeout(timer);
            timer = null;

            this.stopAnimation();

        } catch (err) {
            console.error(err);
        }
    }

    // ==========================================
    // ⏮ PREV
    // ==========================================
    async prevTrack() {
        await this.stopPlay();
        queueState.previous();
    }

    // ==========================================
    // ⏭ NEXT
    // ==========================================
    async nextTrack() {
        await this.stopPlay();
        queueState.next();
    }

    // ==========================================
    // 📡 POLLING RUST
    // ==========================================
    private async runPosition() {

        if (get(player).status !== "playing") return;

        try {
            const [current, total] = await invoke<[number, number]>("get_progress");

            if (seekPending) {
                // Après un seek, ignorer les positions Rust stale
                // Accepter seulement quand Rust a rattrapé la position seekée
                const expectedPos = get(player).jsPosition;
                if (Math.abs(current - expectedPos) < 1.5) {
                    seekPending = false;
                } else {
                    // Position stale, on skip cette mise à jour
                    timer = setTimeout(() => this.runPosition(), 300);
                    return;
                }
            }

            player.update({
                rustPosition: current,
                duration: total
            });

        } catch (err) {
            console.error("Erreur récupération état:", err);
        }

        timer = setTimeout(() => this.runPosition(), 300);
    }

    // ==========================================
    // 🎞 RAF JS
    // ==========================================
    private startAnimation() {

        this.stopAnimation();

        basePos = get(player).rustPosition;
        baseTime = Date.now();

        const loop = () => {

            const p = get(player);
            if (p.status !== "playing") return;

            // Pendant le pré-décodage (Minimal mode), CPAL n'a pas encore
            // démarré : on FIGE la progress bar à 0 pour ne pas afficher
            // une avance fantôme. La boucle continue (raf) mais ne met pas
            // jsPosition à jour, et on re-base le timer pour que la reprise
            // soit propre.
            if (p.isPreparing) {
                basePos = 0;
                baseTime = Date.now();
                raf = requestAnimationFrame(loop);
                return;
            }

            // Se recaler sur la position Rust si drift trop important
            // (seulement quand pas en seek — sinon le guard dans runPosition gère)
            if (p.rustPosition > 0 && !seekPending) {
                const drift = Math.abs((basePos + (Date.now() - baseTime) / 1000) - p.rustPosition);
                if (drift > 2) {
                    basePos = p.rustPosition;
                    baseTime = Date.now();
                }
            }

            const elapsed = (Date.now() - baseTime) / 1000;
            const pos = basePos + elapsed;

            player.update({
                jsPosition: Math.min(pos, p.duration)
            });

            raf = requestAnimationFrame(loop);
        };

        raf = requestAnimationFrame(loop);
    }

    private stopAnimation() {
        if (raf) cancelAnimationFrame(raf);
        raf = 0;
    }
}

export const playerService = new PlayerService();