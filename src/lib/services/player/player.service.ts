import type { UnlistenFn } from "@tauri-apps/api/event";
import { listen } from "@tauri-apps/api/event";
import { player } from "#lib/stores/player/player.store";
import { invoke } from "@tauri-apps/api/core";
import { get } from "svelte/store";
import type { QueueTrack } from "#lib/types/db/queue/QueueTrack";
import type { AudioFile } from "#lib/types/db/audioFile/AudioFile";
import type { RecentFileListView } from "#lib/types/ui/recent/RecentFileListView";
import { recent } from "#lib/stores/recent/recent.store";
import { profilSelector } from "#lib/stores/profil/profil.store";
import { queueState, followingIndex } from "#lib/stores/queue/queueState.store";
import { serverStore } from "#lib/stores/mediaServer/mediaServer.store";
import { settingsStore } from "#lib/stores/settings/settings.store";
import { sleepTimer } from "#lib/stores/player/sleepTimer.store";
import { sendNotification, isPermissionGranted, requestPermission } from "@tauri-apps/plugin-notification";
import { t } from "#lib/i18n";
import { toasts } from "#lib/stores/ui/toast.store";
import { radioPlayer } from "#lib/stores/radio/radioPlayer.store";
import { podcastQueue } from "#lib/stores/podcast/podcastQueue.store";
import { playbackSpeed } from "#lib/stores/player/playbackSpeed.store";
import { chapters } from "#lib/stores/podcast/chapters.store";

/** Morceau d'un serveur injoignable ou désactivé : sauté à l'enchaînement. */
const skipUnavailable = (track: QueueTrack) => serverStore.unavailable(track.path);

let raf: number = 0;
let basePos = 0;
let baseTime = 0;
let playRequestId = 0;
let timer: ReturnType<typeof setTimeout> | null = null;
let seekPending = false;
// Cible et heure du dernier saut : la position Rust est jugée à leur aune, pas à celle de l'animation.
let seekTarget = 0;
let seekAt = 0;

// Dernière position écrite en base, pour n'y revenir que toutes les cinq
// secondes. Remise à zéro à chaque nouveau morceau.
let positionEnregistree = 0;
let trackPlaybackEnded: UnlistenFn | null = null;
let preparingUnlisten: UnlistenFn | null = null;
let queueUnsubscribe: (() => void) | null = null;
let trackAdvancedUnlisten: UnlistenFn | null = null;
let outputLostUnlisten: UnlistenFn | null = null;
/** Sortie changée pendant la pause : la reprise relance le morceau dessus. */
let outputStale = false;
let sleepUnsubscribe: (() => void) | null = null;
let playStartedAt = 0; // timestamp (Date.now()) au moment où CPAL démarre vraiment
let speed = 1; // vitesse de lecture : la position avance d'autant par seconde

// ─── Compteur d'écoutes ───
//
// Chemin du morceau déjà compté, pour ne l'être qu'une fois. Remis à zéro à
// chaque nouvelle lecture.
let playCountedPath: string | null = null;
// Suivi d'écoute (ListenBrainz) : un morceau compte à la moitié ou à 4 min, s'il dure plus de 30 s.
let scrobbledPath: string | null = null;
let listenStartedAt = 0; // secondes Unix, début de l'écoute
const scrobbleThreshold = (duree: number) => Math.min(240, duree / 2);

/**
 * Durée d'écoute au-delà de laquelle un morceau compte pour une écoute.
 *
 * La moitié du morceau, plafonnée à une minute. Compter dès le premier
 * échantillon ferait d'un survol de bibliothèque une série de fausses écoutes,
 * et « les plus écoutés » remonterait ce qu'on a le plus vite passé. Le plafond
 * évite qu'un morceau de vingt minutes ne compte jamais.
 */
function listenThreshold(duree: number): number {
    if (!duree || duree <= 0) return 60;
    return Math.min(60, duree / 2);
}

// Piste annoncée au backend pour le préchargement (lecture sans blanc).
// On mémorise l'index en plus du morceau : à la promotion, c'est lui qui
// permet d'avancer la file sans avoir à rechercher par chemin (un même
// fichier peut figurer plusieurs fois dans la file).
let announcedNext: { track: QueueTrack; index: number } | null = null;
let lastAnnouncedPath: string | null = null;

let initialized = false;

/**
 * Vrai quand une action de l'utilisateur va muter la file et décidera
 * elle-même de la suite.
 *
 * Sans lui, un clic sur une piste produit deux intentions concurrentes. La file
 * est mutée, le synchroniseur y voit un changement de piste et lance la
 * lecture ; puis l'action, qui voulait seulement sélectionner, appelle
 * `preloadTrack` — dont le `stopPlay` tue ce qui vient de démarrer.
 *
 * D'où le symptôme : « Préparation du morceau… » s'affiche, puis rien, ou une
 * lecture qui part quelques secondes plus tard selon qui gagne la course.
 *
 * En mode « simple clic = lecture », le même enchaînement appelait `playFile`
 * deux fois — le défaut passait inaperçu, la seconde demande annulant la
 * première.
 */
let actionExplicite = false;

/** Le profil actif : l'historique et les positions sont les siens. */
const currentProfilId = () => get(profilSelector).profilSelected?.id ?? null;

// « Reprendre là où je m'étais arrêté » ne vaut que pour la première lecture de la session.
let premiereLecture = true;

/** Position enregistrée du morceau, s'il mérite une reprise : ni tout début, ni toute fin. */
async function resumePosition(path: string): Promise<number> {
    try {
        const profilId = currentProfilId();
        const recents = profilId == null ? [] : await invoke<RecentFileListView[]>("get_recent_files", { profilId });
        const r = recents.find((f) => f.path === path);
        // Épisode de podcast : sa position vit avec lui, pas dans les récents.
        if (!r) {
            if (!path.includes("podcasts")) return 0;
            const pos = (await invoke<number | null>("podcast_resume_position", { path })) ?? 0;
            return pos > 5 ? pos : 0;
        }
        const pos = r.last_position ?? 0;
        return pos > 5 && (!r.duration || pos < r.duration - 5) ? pos : 0;
    } catch {
        return 0;
    }
}

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
        await this.initOutputLostListener();
        this.initQueueSync();

        // La minuterie « fin du morceau » doit empêcher tout préchargement :
        // on réévalue donc l'annonce à chaque changement de son état.
        sleepUnsubscribe = sleepTimer.subscribe(() => this.syncNextTrack());

        // Sleep timer : à échéance (mode durée), on met en pause la lecture.
        sleepTimer.setOnFire(() => {
            if (this.isPlaying()) this.pauseFile();
        });
    }

    destroy() {
        if (trackPlaybackEnded) trackPlaybackEnded();
        if (preparingUnlisten) preparingUnlisten();
        if (trackAdvancedUnlisten) trackAdvancedUnlisten();
        if (outputLostUnlisten) outputLostUnlisten();
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
                // Le son part vraiment maintenant : c'est ici, et nulle part
                // avant, que l'animation a le droit de commencer.
                playStartedAt = Date.now();
                this.startAnimation();
            }
        });
    }

    // ==========================================
    // 🔌 SORTIE AUDIO PERDUE (DAC débranché…)
    // ==========================================
    // Le Rust a basculé sur la sortie du système, ou n'a rien trouvé : on arrête.
    private async initOutputLostListener() {
        outputLostUnlisten = await listen<{ device: string; fallback: string | null }>("audio-output-lost", (e) => {
            const { device, fallback } = e.payload;
            const tr = get(t);
            if (fallback) {
                toasts.push({
                    type: "warning",
                    title: tr("player.output_lost"),
                    message: tr("player.output_fallback").replace("{lost}", device).replace("{device}", fallback),
                    duration: 8000,
                });
                return;
            }
            this.stopPlay();
            toasts.push({
                type: "error",
                title: tr("player.output_lost"),
                message: tr("player.output_stopped").replace("{lost}", device),
                duration: 8000,
            });
        });
    }

    // ==========================================
    // 🎧 LISTENER FIN DE LECTURE
    // ==========================================
    private async initPlaybackListener() {
        trackPlaybackEnded = await listen("playback-ended", () => {
            this.stopAnimation();
            // Radio : le flux ne finit pas, il s'est perdu ; la file musicale n'enchaîne pas.
            if (radioPlayer.isActive()) {
                radioPlayer.onEnded();
                return;
            }
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

            // Épisode de podcast : le suivant de sa file, la musique reste en attente.
            if (podcastQueue.isActive()) {
                const next = podcastQueue.advance();
                if (next) this.playFile(next, true);
                else this.stopPlay();
                return;
            }

            const state = get(queueState);
            // Même morceau à rejouer : l'index ne change pas, rien d'autre ne le relancerait.
            if (state.repeatMode === "one" || (state.repeatMode === "all" && state.tracks.length === 1)) {
                const track = state.tracks[state.currentIndex];
                if (track) this.playFile(track);
            } else {
                // Fin de file : arrêt, barre remise à zéro (la file ne change pas, rien ne le ferait).
                queueState.next(skipUnavailable).then((suite) => {
                    if (!suite) this.stopPlay();
                });
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

            // Un épisode joue : la file musique change sans le couper.
            if (podcastQueue.isActive()) return;

            if (!track) {
                this.stopPlay();
                return;
            }

            // Une action de l'utilisateur est en cours : elle a muté la file
            // et sait ce qu'elle veut en faire — lire, ou seulement
            // sélectionner. Décider ici reviendrait à choisir à sa place.
            //
            // Avant la branche du premier chargement, et non après : sur une
            // file vide, celle-ci préchargerait avant que l'action n'ait la
            // main, avec l'arrêt et l'ouverture de fichier que ça suppose.
            if (actionExplicite) {
                actionExplicite = false;
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

        if (sleepMode !== "end-of-track" && !podcastQueue.isActive() && qs.tracks.length > 0 && qs.currentIndex >= 0) {
            const index = followingIndex(qs, skipUnavailable);
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
            let advanced = announcedNext;
            announcedNext = null;
            lastAnnouncedPath = null;

            // Promu autre chose que l'annonce : le moteur joue déjà ce morceau,
            // on le retrouve dans la file plutôt que de garder l'ancien affiché
            // (compteur qui recule, puis le même morceau rejoué).
            if (!advanced || advanced.track.path !== e.payload) {
                const qs = get(queueState);
                const apres = qs.tracks.findIndex((t, i) => i > qs.currentIndex && t.path === e.payload);
                const index = apres >= 0 ? apres : qs.tracks.findIndex((t) => t.path === e.payload);
                invoke("log_player_event", {
                    message: `enchaînement inattendu : promu ${e.payload}, annoncé ${advanced?.track.path ?? "rien"}, retrouvé à l'index ${index}`,
                }).catch(() => {});
                if (index < 0) {
                    this.syncNextTrack();
                    return;
                }
                advanced = { track: qs.tracks[index], index };
            }

            const track = advanced.track;

            // L'enchaînement sans blanc change de morceau sans repasser par
            // `playFile` : sans cette remise à zéro, le morceau promu ne serait
            // jamais compté, et l'enchaînement — le mode d'écoute le plus
            // courant — échapperait entièrement au compteur.
            playCountedPath = null;
            scrobbledPath = null;
            listenStartedAt = Math.floor(Date.now() / 1000);
            chapters.clear();

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
                const profilId = currentProfilId();
                if (profilId != null) {
                    await invoke<void>("insert_recent_file", { profilId, path: track.path });
                    invoke("scrobble_now_playing", { profilId, path: track.path }).catch(() => {});
                }
                recent.refreshRecent();
                this.sendTrackNotification(track, audioFile);
            } catch (err) {
                console.error("[gapless] resynchronisation après enchaînement :", err);
                invoke("log_player_event", { message: `resynchronisation après enchaînement : ${err}` }).catch(() => {});
            }

            // Annonce de la piste d'après, pour rester en avance.
            this.syncNextTrack();
        });
    }

    // ==========================================
    // 📦 PRELOAD (conservé)
    // ==========================================
    /**
     * Annonce qu'une action va muter la file et décidera elle-même de la suite.
     *
     * À appeler juste avant la mutation. Le drapeau est consommé par le
     * synchroniseur, ou à défaut par la lecture ou le préchargement qui suit :
     * il ne peut pas rester armé et avaler une lecture ultérieure.
     */
    expectExplicitAction() {
        actionExplicite = true;
    }

    async preloadTrack(track: QueueTrack) {
        actionExplicite = false;


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
    /** `podcast` : épisode de la file des podcasts ; sinon la musique reprend la main. */
    async playFile(track: QueueTrack, podcast = false) {
        actionExplicite = false;
        // Une seule source : la radio cède la place (le backend la coupe).
        radioPlayer.detach();
        if (!podcast) podcastQueue.detach();
        // Ce qui suit dépend de la source : rien après un épisode, la file musique sinon.
        this.syncNextTrack();
        premiereLecture = false;
        outputStale = false;


        const currentRequestId = ++playRequestId;

        try {
            await this.stopPlay();

            const audioFile = await invoke("open_file", { path: track.path }) as AudioFile;

            if (currentRequestId !== playRequestId) return;

            player.update({
                status: "playing",
                pathFile: track.path,
                audioFile,
                trackId: track.queueId,
                isPreparing: true,
                jsPosition: 0,
                rustPosition: 0,
            });

            playCountedPath = null;
            scrobbledPath = null;
            listenStartedAt = Math.floor(Date.now() / 1000);
            positionEnregistree = 0;

            await invoke("play_file", { path: track.path });

            playStartedAt = Date.now();
            // Vitesse choisie par le moteur (celle de l'émission pour un épisode).
            this.applySpeed(await invoke<number>("playback_speed").catch(() => 1));
            // Chapitres : seulement pour un épisode.
            if (audioFile.tags?.media_type === "podcast") void chapters.load(track.path);
            else chapters.clear();

            this.runPosition();

            // Gestion récents (l'entrée du cache se retrouve côté moteur)
            const profilId = currentProfilId();
            if (profilId != null) {
                await invoke<void>("insert_recent_file", { profilId, path: track.path });
                invoke("scrobble_now_playing", { profilId, path: track.path }).catch(() => {});
            }

            recent.refreshRecent();

            // Notification système si activée dans les paramètres
            this.sendTrackNotification(track, audioFile);

            // Épisode : repris là où on l'avait laissé.
            if (podcast) {
                const from = await resumePosition(track.path);
                if (from > 0 && currentRequestId === playRequestId && (await this.waitUntilReady(15000))) await this.seekTo(from);
            }

        } catch (err) {
            console.error("Erreur lecture:", err);
        }
    }

    /** Vitesse de l'épisode en cours, retenue pour son émission. */
    async setSpeed(value: number) {
        const path = get(player).pathFile;
        if (!path) return;
        this.applySpeed(value);
        await invoke("podcast_set_speed", { path, speed: value }).catch(() => {});
    }

    /** L'animation repart du point atteint, au nouveau rythme. */
    private applySpeed(value: number) {
        if (baseTime > 0) basePos += (Date.now() - baseTime) / 1000 * speed;
        baseTime = Date.now();
        speed = value;
        playbackSpeed.set(value);
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

            const title = audioFile.tags?.title ?? track.title ?? get(t)('common.unknown_title');
            const artist = audioFile.tags?.artist ?? track.artist ?? get(t)('common.unknown_artist');
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
        // La radio a son propre arrêt : une file vidée ou rechargée ne la coupe pas.
        if (radioPlayer.isActive()) return;
        // L'arrêt côté Rust peut durer : le journal montre des « Timeout stop()
        // — le stream ne s'est pas arrêté en 3s ». Pendant ces secondes, une
        // lecture plus récente a le temps de partir. Sans ce garde, l'arrêt
        // retardataire écrasait son état : bouton sur « lecture », position à
        // 00:00 et animation figée, alors que le son tournait.
        const idAuDepart = playRequestId;

        try {
            await invoke("stop_play");

            if (idAuDepart !== playRequestId) return;

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
        seekTarget = positionSeconds;
        seekAt = Date.now();
    }

    // ==========================================
    // ⏯ TOGGLE
    // ==========================================
    async handleTogglePlay() {
        if (radioPlayer.isActive()) return radioPlayer.toggle();

        const queue = get(queueState);
        const currentTrack = queue.tracks[queue.currentIndex] ?? null;
        if (!currentTrack) return;

        const status = get(player).status;

        if (status === "playing") {
            await this.pauseFile();
        } else if (status === "paused") {
            await this.resumePlay();
        } else {
            const reprise = premiereLecture && settingsStore.get('resume_playback') !== 'false'
                ? await resumePosition(currentTrack.path)
                : 0;
            await this.playFile(currentTrack);
            if (reprise && await this.waitUntilReady()) await this.seekTo(reprise);
        }
    }

    /**
     * Change la sortie (périphérique, exclusif, DSD, profil) et relance le morceau dessus,
     * au même endroit : le Rust ne lit ces réglages qu'au lancement d'un morceau.
     * `localOnly` : rien à relancer quand un lecteur réseau joue.
     */
    async applyOutputChange(change: () => Promise<void>, { replay = true, localOnly = false } = {}) {
        const before = get(player);
        const position = before.jsPosition ?? 0;
        await change();
        if (!replay || !before.trackId) return;
        if (localOnly) {
            const { networkPlayerStore } = await import("#lib/stores/audio/networkPlayer.store");
            if (get(networkPlayerStore).selected) return;
        }
        if (before.status === "paused") {
            outputStale = true;
            return;
        }
        if (before.status !== "playing") return;
        await this.replayAt(position);
    }

    /** Relance le morceau courant de la file à `position`. */
    private async replayAt(position: number) {
        const qs = get(queueState);
        const track = qs.tracks[qs.currentIndex];
        if (!track) return;
        await this.playFile(track);
        if (position > 2 && (await this.waitUntilReady(10000))) await this.seekTo(position);
    }

    /** Attend que le son parte vraiment : un saut pendant le pré-décodage se perdrait. */
    async waitUntilReady(delaiMs = 6000): Promise<boolean> {
        const debut = Date.now();
        while (Date.now() - debut < delaiMs) {
            const p = get(player);
            if (p.status === "playing" && !p.isPreparing) return true;
            await new Promise((r) => setTimeout(r, 100));
        }
        return false;
    }

    async resumePlay() {
        if (outputStale) {
            outputStale = false;
            await this.replayAt(get(player).jsPosition ?? 0);
            return;
        }
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
    /** Du son sort-il ? Radio comprise. */
    isPlaying(): boolean {
        if (radioPlayer.isActive()) return get(radioPlayer).status === "playing";
        return get(player).status === "playing";
    }

    async pauseFile() {
        if (radioPlayer.isActive()) {
            if (get(radioPlayer).status === "playing") await radioPlayer.toggle();
            return;
        }
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
        if (radioPlayer.isActive()) return radioPlayer.prev();
        if (podcastQueue.isActive()) {
            const episode = podcastQueue.back();
            if (episode) await this.playFile(episode, true);
            return;
        }
        await this.stopPlay();
        queueState.previous();
    }

    // ==========================================
    // ⏭ NEXT
    // ==========================================
    async nextTrack() {
        if (radioPlayer.isActive()) return radioPlayer.next();
        if (podcastQueue.isActive()) {
            const episode = podcastQueue.advance();
            if (episode) await this.playFile(episode, true);
            return;
        }
        await this.stopPlay();
        queueState.next(skipUnavailable);
    }

    // ==========================================
    // 📡 POLLING RUST
    // ==========================================
    private async runPosition() {

        if (get(player).status !== "playing") return;

        // ─── Une seule chaîne de sondage à la fois ───
        //
        // Chaque tour programme le suivant. Si deux chaînes coexistent — un
        // sondage encore en vol pendant qu'un changement de piste en relance un
        // — elles ne se remplacent pas, elles s'additionnent, et le compte
        // double à chaque piste. On annule donc l'échéance en attente avant
        // d'en programmer une nouvelle.
        if (timer) clearTimeout(timer);
        timer = null;

        try {
            const [current, total] = await invoke<[number, number]>("get_progress");

            // L'aller-retour ci-dessus prend le temps qu'il prend, et la
            // lecture a pu se terminer pendant. Poursuivre reviendrait à
            // ressusciter l'animation d'un morceau fini et à reprogrammer un
            // sondage que plus personne n'arrêtera.
            if (get(player).status !== "playing") return;

            if (seekPending) {
                // Après un seek, ignorer les positions Rust stale.
                // Rust a rejoint la cible quand il est entre elle et ce qui a pu
                // être joué depuis ; au bout de 5 s, on le croit quoi qu'il dise.
                const ecoule = (Date.now() - seekAt) / 1000;
                if ((current >= seekTarget - 0.5 && current <= seekTarget + ecoule * speed + 1.5) || ecoule > 5) {
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

            // ─── Où l'on en est, pour pouvoir y revenir ───
            //
            // Toutes les cinq secondes, pas à chaque tour : c'est une écriture
            // disque, et cinq secondes d'imprécision ne se sentent pas à la
            // reprise. Sans `await` — la position ne doit pas retarder le
            // sondage, et son échec ne regarde pas la lecture.
            if (get(player).pathFile && current - positionEnregistree >= 5) {
                positionEnregistree = current;
                invoke('save_playback_position', {
                    profilId: currentProfilId() ?? 0,
                    path: get(player).pathFile,
                    position: current,
                    duration: total,
                }).catch(() => {});
            }

            // ─── L'écoute se compte ici ───
            //
            // Ce sondage tourne une fois par seconde pendant toute la lecture :
            // c'est le seul endroit qui sait combien on a réellement écouté.
            // Compter au démarrage aurait été plus simple, et faux — voir
            // `seuilEcoute`.
            const p = get(player);
            if (
                p.pathFile &&
                p.pathFile !== playCountedPath &&
                current >= listenThreshold(total)
            ) {
                playCountedPath = p.pathFile;
                // Sans `await` : un compteur ne doit pas retarder le sondage de
                // position, et son échec ne regarde pas la lecture.
                invoke('mark_track_played', { path: p.pathFile }).catch(e =>
                    console.error('Compteur d\'écoutes :', e)
                );
            }
            // Suivi d'écoute : un seuil à part, celui de ListenBrainz et Last.fm.
            if (p.pathFile && p.pathFile !== scrobbledPath && total > 30 && current >= scrobbleThreshold(total)) {
                scrobbledPath = p.pathFile;
                const profilId = currentProfilId();
                if (profilId != null) {
                    invoke('scrobble_listen', { profilId, path: p.pathFile, listenedAt: listenStartedAt || Math.floor(Date.now() / 1000 - current) })
                        .catch(e => console.error('Suivi d\'écoute :', e));
                }
            }

            // Filet : si la position avance alors que l'animation dort, c'est
            // que le signal de départ s'est perdu — un chemin de lecture qui
            // ne l'émet pas, un événement manqué au démarrage. On ne laisse pas
            // la barre figée pour autant.
            if (current > 0 && raf === 0) {
                player.update({ isPreparing: false });
                this.startAnimation();
            }

        } catch (err) {
            console.error("Erreur récupération état:", err);
        }

        // Une seconde, et non trois cents millisecondes : cette position ne
        // sert qu'à recaler l'animation lorsqu'elle dérive de plus de deux
        // secondes. Interroger trois fois par seconde pour détecter un écart de
        // deux secondes, ce sont trois allers-retours qui réveillent le backend
        // pour rien. La reprise après un seek, elle, garde sa cadence rapide.
        timer = setTimeout(() => this.runPosition(), 1000);
    }

    // ==========================================
    // 🎞 RAF JS
    // ==========================================
    private startAnimation() {

        this.stopAnimation();

        basePos = get(player).rustPosition;
        baseTime = Date.now();

        // Dernière valeur poussée dans le store, et seconde entière déjà
        // affichée. Voir la condition de poussée dans la boucle.
        let lastPush = 0;
        let lastWhole = -1;

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
                const drift = Math.abs((basePos + (Date.now() - baseTime) / 1000 * speed) - p.rustPosition);
                if (drift > 2) {
                    basePos = p.rustPosition;
                    baseTime = Date.now();
                }
            }

            const elapsed = (Date.now() - baseTime) / 1000 * speed;
            const pos = basePos + elapsed;

            // ─── Ne réveiller l'interface que quand ça se voit ───
            //
            // Cette boucle tourne à soixante images par seconde, et chaque
            // écriture dans le store réveille les six composants qui s'y
            // abonnent — lecteur, mini-lecteur, file d'attente, paroles — qui
            // recalculent tous leurs dérivés et se redessinent.
            //
            // Or la position ne se voit qu'à deux endroits : une étiquette au
            // format `m:ss`, qui change une fois par seconde, et une barre dont
            // la largeur bouge d'un pixel tous les millièmes de morceau.
            // Soixante hertz pour ça, ce sont cinquante rafraîchissements par
            // seconde qui ne changent rien à l'écran.
            //
            // On pousse donc sur deux motifs : la seconde affichée change, ou
            // un dixième de seconde s'est écoulé — un déplacement de barre
            // largement sous le seuil de perception, et six fois moins de
            // travail pour l'interface.
            const now = Date.now();
            const whole = Math.floor(pos);

            if (whole !== lastWhole || now - lastPush >= 100) {
                lastWhole = whole;
                lastPush = now;
                player.update({
                    jsPosition: Math.min(pos, p.duration)
                });
            }

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