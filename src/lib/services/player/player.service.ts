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

// ─── Compteur d'écoutes ───
//
// Chemin du morceau déjà compté, pour ne l'être qu'une fois. Remis à zéro à
// chaque nouvelle lecture.
let playCountedPath: string | null = null;

/**
 * Durée d'écoute au-delà de laquelle un morceau compte pour une écoute.
 *
 * La moitié du morceau, plafonnée à une minute. Compter dès le premier
 * échantillon ferait d'un survol de bibliothèque une série de fausses écoutes,
 * et « les plus écoutés » remonterait ce qu'on a le plus vite passé. Le plafond
 * évite qu'un morceau de vingt minutes ne compte jamais.
 */
function seuilEcoute(duree: number): number {
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
                // Le son part vraiment maintenant : c'est ici, et nulle part
                // avant, que l'animation a le droit de commencer.
                playStartedAt = Date.now();
                this.startAnimation();
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

            // L'enchaînement sans blanc change de morceau sans repasser par
            // `playFile` : sans cette remise à zéro, le morceau promu ne serait
            // jamais compté, et l'enchaînement — le mode d'écoute le plus
            // courant — échapperait entièrement au compteur.
            playCountedPath = null;

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
    async playFile(track: QueueTrack) {
        actionExplicite = false;


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
                // Figé d'emblée, sans attendre l'événement du backend : celui-ci
                // part d'un fil qui vient d'être lancé, et les quelques images
                // qui séparent les deux suffiraient à faire sauter la barre.
                // Le backend rendra la main avec `playback-preparing: false`
                // quand le premier échantillon partira vraiment.
                isPreparing: true,
                jsPosition: 0,
                rustPosition: 0,
            });

            // Nouvelle lecture, nouvelle écoute à compter. Rejouer deux fois
            // le même morceau doit compter deux fois : c'est le garde par
            // chemin, remis à zéro ici, qui empêche seulement de le compter
            // plusieurs fois pendant une même lecture.
            playCountedPath = null;

            await invoke("play_file", { path: track.path });

            // Si pas de mode Minimal/pre-decode, on considère que la lecture
            // commence immédiatement (sera écrasé par le preparing listener
            // pour les profils Low/Minimal).
            playStartedAt = Date.now();

            // Le sondage de position démarre, pas l'animation.
            //
            // `play_file` lance un fil et rend la main en quelques
            // millisecondes ; le son, lui, ne part qu'une à deux secondes plus
            // tard — ouverture du périphérique, négociation du format,
            // pré-remplissage. Animer dès maintenant, c'est compter dans le
            // vide, puis revenir à zéro quand la vraie position arrive.
            //
            // C'est donc le backend qui donne le départ, par
            // `playback-preparing: false`. À défaut, `runPosition` s'en charge
            // dès qu'il voit la position avancer — un filet, pas le chemin
            // normal.
            this.runPosition();

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
                current >= seuilEcoute(total)
            ) {
                playCountedPath = p.pathFile;
                // Sans `await` : un compteur ne doit pas retarder le sondage de
                // position, et son échec ne regarde pas la lecture.
                invoke('mark_track_played', { path: p.pathFile }).catch(e =>
                    console.error('Compteur d\'écoutes :', e)
                );
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
                const drift = Math.abs((basePos + (Date.now() - baseTime) / 1000) - p.rustPosition);
                if (drift > 2) {
                    basePos = p.rustPosition;
                    baseTime = Date.now();
                }
            }

            const elapsed = (Date.now() - baseTime) / 1000;
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