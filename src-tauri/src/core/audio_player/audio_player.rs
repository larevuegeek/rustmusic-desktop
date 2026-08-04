use std::sync::{Arc, Mutex, RwLock};
use std::thread::JoinHandle;
use std::{fs::File, path::PathBuf};

use ringbuf::traits::Observer;
use symphonia::core::audio::GenericAudioBufferRef;
use symphonia::core::codecs::audio::{AudioCodecParameters, AudioDecoder, AudioDecoderOptions};
use symphonia::core::errors::Error;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::FormatOptions;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::units::Timestamp;
use symphonia::{
    core::{formats::FormatReader, io::MediaSourceStream},
    default::get_probe,
};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ringbuf::{traits::{Producer, Consumer, Split}};
use tauri::{AppHandle, Emitter};
use std::sync::atomic::{AtomicBool, Ordering, AtomicU8, AtomicU64, AtomicUsize};
use crate::core::audio_decoder::dsd::dff_decoder::DffDecoder;
use crate::core::audio_decoder::dsd::dsd_container::DsdContainerReader;
use crate::core::audio_decoder::dsd::dsd_player::run_dsd_playback;
use crate::core::audio_decoder::dsd::dsf_decoder::DsfDecoder;
use crate::core::audio_player::audio_utils::{adapt_channels, convert_audio_buffer_to_interleaved};
use crate::core::audio_resampler::resampler::Resampler;

// ============================================================================
// AUDIO PLAYER - La structure principale
// ============================================================================

#[derive(Debug)]
pub struct AudioPlayer {
    is_paused: Arc<AtomicBool>,
    is_playing: Arc<AtomicBool>,
    is_stopped: Arc<AtomicBool>,
    is_stream_alive: Arc<AtomicBool>,
    pub(crate) current_position: Arc<AtomicU64>, // position en secondes (f64.to_bits())
    total_duration: Arc<AtomicU64>,              // durée totale en secondes (f64.to_bits())
    volume: Arc<AtomicU8>, // 0..100
    selected_device: Arc<Mutex<Option<String>>>,
    seek_position: Arc<AtomicU64>,
}

impl AudioPlayer {
    pub fn new() -> Self {
        Self {
            is_paused: Arc::new(AtomicBool::new(false)),
            is_playing: Arc::new(AtomicBool::new(false)),
            is_stopped: Arc::new(AtomicBool::new(false)),
            is_stream_alive: Arc::new(AtomicBool::new(false)),
            current_position: Arc::new(AtomicU64::new(0.0_f64.to_bits())),
            total_duration: Arc::new(AtomicU64::new(0.0_f64.to_bits())),
            volume: Arc::new(AtomicU8::new(80)),
            selected_device: Arc::new(Mutex::new(None)),
            seek_position: Arc::new(AtomicU64::new(u64::MAX)),
        }
    }

    pub fn set_device(&self, device_name: Option<String>) {
        if let Ok(mut selected) = self.selected_device.lock() {
            *selected = device_name;
        }
    }

    pub fn get_selected_device_name(&self) -> Option<String> {
        self.selected_device.lock().ok().and_then(|d| d.clone())
    }

    pub fn seek_to(&self, position_seconds: f64) {
        // N'écrit que seek_position (signal pour le decoder thread).
        // Ne PAS écrire current_position ici — la Phase 6 le dérivera
        // de current_position_frames une fois le decoder thread mis à jour.
        self.seek_position.store(position_seconds.to_bits(), Ordering::Relaxed);
    }

    pub fn pause(&self) {
        self.is_paused.store(true, Ordering::SeqCst);
        log::debug!("⏸️ Lecture mise en pause");
    }

    pub fn resume(&self) {
        self.is_paused.store(false, Ordering::SeqCst);
        log::debug!("▶️ Lecture reprise");
    }

    pub fn stop(&self) {
        log::debug!("🛑 Arrêt du lecteur demandé");

        // Active le flag d'arrêt
        self.is_stopped.store(true, Ordering::SeqCst);

        // Attendre que le stream se ferme, avec timeout de 3 secondes
        // Si le thread audio a crash, on ne reste pas bloqué indéfiniment
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        while self.is_stream_alive.load(Ordering::SeqCst) {
            std::thread::sleep(std::time::Duration::from_millis(2));
            if std::time::Instant::now() >= deadline {
                log::warn!("⚠️ Timeout stop() — le stream ne s'est pas arrêté en 3s, on force");
                self.is_stream_alive.store(false, Ordering::SeqCst);
                break;
            }
        }

        self.is_paused.store(false, Ordering::SeqCst);
        self.is_playing.store(false, Ordering::SeqCst);

        self.clear_position();

        log::debug!("✅ Lecture arrêtée et réinitialisée");
    }

    pub fn is_paused(&self) -> bool {
        self.is_paused.load(Ordering::SeqCst)
    }

    pub fn is_playing(&self) -> bool {
        self.is_playing.load(Ordering::SeqCst)
    }

    pub fn get_current_position(&self) -> f64 {

        let bits_lus: u64 =  self.current_position.load(Ordering::Relaxed);
        let position_actuelle: f64 = f64::from_bits(bits_lus);

        position_actuelle
    }

    pub fn get_total_duration(&self) -> f64 {
        f64::from_bits(self.total_duration.load(Ordering::Relaxed))
    }

    pub fn clear_position(&self) {
        self.current_position.store(0_f64.to_bits(), Ordering::Relaxed);
    }

    pub fn set_volume(&self, volume: u8) {
        self.volume.store(volume.min(100), Ordering::Relaxed);
    }

    pub fn get_volume(&self) -> u8 {
        self.volume.load(Ordering::Relaxed)
    }

    pub fn play_file(&self, app_handle: AppHandle, file_path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {

        // compare_exchange atomique : si is_playing est false, on le met à true
        // Si un autre thread fait la même chose au même moment, un seul réussit
        // Ça élimine la race condition entre le check et le set
        if self.is_playing.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
            log::debug!("⚠️ Lecture déjà en cours, on ignore la demande de lecture");
            return Ok(());
        }

        log::debug!("🎵 Fichier sélectionné : {:?}", file_path);

        self.is_stopped.store(false, Ordering::SeqCst);
        self.is_stream_alive.store(true, Ordering::SeqCst);

        // Clone pour le thread (on garde la pause synchronisée)
        let is_playing_clone: Arc<AtomicBool> = self.is_playing.clone();
        let is_paused_clone: Arc<AtomicBool> = self.is_paused.clone();
        let is_stopped_clone: Arc<AtomicBool> = self.is_stopped.clone();
        let is_stream_alive_clone: Arc<AtomicBool> = self.is_stream_alive.clone();
        let current_position_clone: Arc<AtomicU64> = self.current_position.clone();
        let total_duration_clone: Arc<AtomicU64> = self.total_duration.clone();
        let volume_clone: Arc<AtomicU8> = self.volume.clone();
        let seek_position_clone: Arc<AtomicU64> = self.seek_position.clone();
        let selected_device_name = self.get_selected_device_name();

        // Reset seek position pour le nouveau fichier
        self.seek_position.store(u64::MAX, Ordering::Relaxed);

        std::thread::spawn(move || {
            if let Err(e) = Self::play_file_thread(
                app_handle,
                file_path,
                is_paused_clone,
                is_playing_clone,
                is_stopped_clone,
                is_stream_alive_clone,
                current_position_clone,
                total_duration_clone,
                volume_clone,
                selected_device_name,
                seek_position_clone,
            ) {
                log::error!("❌ Erreur lecture fichier: {}", e);
            }
        });

        Ok(())
    }

    pub fn play_file_thread(
        app_handle: AppHandle,
        file_path: PathBuf,
        is_paused: Arc<AtomicBool>,
        is_playing: Arc<AtomicBool>,
        is_stopped: Arc<AtomicBool>,
        is_stream_alive: Arc<AtomicBool>,
        current_position: Arc<AtomicU64>,
        total_duration: Arc<AtomicU64>,
        volume: Arc<AtomicU8>,
        selected_device_name: Option<String>,
        seek_position: Arc<AtomicU64>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Court-circuit DSF/DFF : Symphonia ne sait pas lire DSD,
        // on dispatche vers notre pipeline maison (audio_decoder::dsd).
        let ext_lower = file_path
            .extension()
            .and_then(|s| s.to_str())
            .map(|e| e.to_ascii_lowercase());

        if let Some(ref ext) = ext_lower {
            if ext == "dsf" || ext == "dff" {
                return Self::play_dsd_file_thread(
                    app_handle,
                    file_path,
                    is_paused,
                    is_playing,
                    is_stopped,
                    is_stream_alive,
                    current_position,
                    total_duration,
                    volume,
                    selected_device_name,
                    seek_position,
                );
            }
        }

        // Chemin PCM (Symphonia) : on n'est PLUS en DSD → fermer le moteur DoP
        // s'il tournait encore (libère le DAC, qui repasse en PCM).
        #[cfg(target_os = "windows")]
        crate::core::audio_player::output::dop_engine::teardown_engine();

        log::debug!("Fichier sélectionné : {:?}", file_path);

        // ========== PHASE 1 : PROBE ET INFOS ==========
        let mut hint: Hint = Hint::new();
        if let Some(ext) = file_path.extension().and_then(|s| s.to_str()) {
            hint.with_extension(ext);
        }

        // Décision pre-load vs streaming direct :
        //   - Pre-load (tout en RAM) si le fichier est sur un mount réseau
        //     (SMB, NFS, GVFS…) OU si le profil audio est Low/Minimal
        //     (utilisateurs en environnement contraint qui ont besoin de
        //     robustesse plutôt que de play instantané).
        //   - Sinon streaming direct depuis le disque (play instantané,
        //     pas de latence à l'appui sur ▶).
        let pipeline_profile = crate::core::audio_quality::current_profile();
        let needs_preload = crate::core::system_detect::is_network_path(&file_path)
            || matches!(
                pipeline_profile,
                crate::core::audio_quality::AudioQualityProfile::Low
                    | crate::core::audio_quality::AudioQualityProfile::Minimal
            );

        let mss: MediaSourceStream = if needs_preload {
            let bytes: Vec<u8> = std::fs::read(&file_path)?;
            log::info!(
                "📥 Fichier pré-chargé en RAM : {} MB ({})",
                bytes.len() / (1024 * 1024),
                if crate::core::system_detect::is_network_path(&file_path) {
                    "réseau"
                } else {
                    "profil contraint"
                }
            );
            MediaSourceStream::new(Box::new(std::io::Cursor::new(bytes)), Default::default())
        } else {
            let source: Box<File> = Box::new(File::open(&file_path)?);
            MediaSourceStream::new(source, Default::default())
        };

        let format_opts: FormatOptions = Default::default();
        let metadata_opts: MetadataOptions = Default::default();

        let format: Box<dyn FormatReader> =
            get_probe().probe(&hint, mss, format_opts, metadata_opts)?;

        let track: &symphonia::core::formats::Track = format
            .tracks()
            .iter()
            .find(|t| {
                t.codec_params
                    .as_ref()
                    .and_then(|c| c.audio())
                    .map_or(false, |a| a.sample_rate.is_some())
            })
            .ok_or("Aucune piste audio trouvée")?;

        let track_id: u32 = track.id;
        let codec_params: AudioCodecParameters = track
            .codec_params
            .as_ref()
            .and_then(|c| c.audio())
            .cloned()
            .ok_or("Paramètres codec audio manquants")?;
        let source_sample_rate: u32 = codec_params.sample_rate
            .ok_or("Pas de sample rate détecté dans le fichier")?;
        let channels: usize = codec_params.channels.as_ref()
            .ok_or("Pas d'info channels dans le fichier")?.count();

        // Durée : `duration` (en unités de timebase) prioritaire, sinon
        // fallback num_frames / sample_rate (timing déplacé sur Track en 0.6).
        let mut total_secs: Option<f64> = None;
        if let (Some(dur), Some(tb)) = (track.duration, track.time_base) {
            total_secs = tb
                .calc_time(Timestamp::from(dur.get() as i64))
                .map(|t| t.as_secs_f64());
        }
        if total_secs.is_none() {
            if let Some(n_frames) = track.num_frames {
                total_secs = Some(n_frames as f64 / source_sample_rate as f64);
            }
        }
        if let Some(secs) = total_secs {
            total_duration.store(secs.to_bits(), Ordering::Relaxed);
            log::debug!("🕒 Durée totale détectée : {:.2} secondes", secs);
        } else {
            log::debug!("⚠️ Impossible de déterminer la durée totale");
        }

        // ========== PHASE 2 : CONFIGURATION SORTIE ==========
        let host: cpal::Host = cpal::default_host();
        let device: cpal::Device = if let Some(ref name) = selected_device_name {
            let found = host.output_devices()
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?
                .find(|d| {
                    if let Ok(desc) = d.description() {
                        let dn = desc.name().to_string();
                        let display = match desc.manufacturer() {
                            Some(mfr) => format!("{} ({})", dn, mfr),
                            None => dn.clone(),
                        };
                        dn == *name || display == *name
                    } else { false }
                });
            match found {
                Some(d) => d,
                None => {
                    log::error!("Device '{}' introuvable, fallback default", name);
                    host.default_output_device().ok_or("Pas de périphérique audio")?
                }
            }
        } else {
            host.default_output_device().ok_or("Pas de périphérique audio")?
        };

        log::debug!("Output device: {:?}", device.description());

        let output_config: cpal::SupportedStreamConfig = device.default_output_config()?;
        let mut output_sample_rate: u32 = output_config.sample_rate();
        let mut output_channels: u16 = output_config.channels();
        log::info!(
            "🔊 CPAL device negotiated : {} Hz × {} ch · sample format {:?} · buffer config {:?}",
            output_sample_rate,
            output_channels,
            output_config.sample_format(),
            output_config.buffer_size()
        );

        // ─── Nom complet du device CPAL "Nom (Fabricant/Pilote)" ───
        // Nécessaire tôt : sert au matching WASAPI (le nom brut seul est
        // ambigu) ET à la pipeline info. Identique au display_name de
        // device_command pour que la sélection UI corresponde.
        let device_name = device
            .description()
            .ok()
            .map(|d| {
                let name = d.name().to_string();
                match (d.manufacturer(), d.driver()) {
                    (Some(mfr), _) => format!("{} ({})", name, mfr),
                    (_, Some(drv)) => format!("{} ({})", name, drv),
                    _ => name,
                }
            })
            .unwrap_or_else(|| "Périphérique audio".to_string());

        // ─── Pré-négociation WASAPI (Windows) : pilote le décodeur au rate
        // natif du DAC pour une vraie sortie bit-perfect ───
        // Le décodeur resample source → output_sample_rate. Pour du
        // bit-perfect, on veut que ce rate soit celui que WASAPI jouera
        // (idéalement le rate source, sinon le meilleur rate accepté par le
        // DAC en exclusive). On négocie DONC ici, avant de configurer le
        // décodeur, et on override output_sample_rate/channels en conséquence.
        #[cfg(target_os = "windows")]
        {
            use crate::core::audio_player::output;
            if matches!(output::current_preference(), output::AudioBackend::WasapiExclusive) {
                match crate::core::audio_player::audio_output_wasapi::try_negotiate_exclusive_format(
                    source_sample_rate,
                    channels as u16,
                    Some(device_name.clone()),
                ) {
                    Ok(fmt) => {
                        log::info!(
                            "🎚️  WASAPI pré-négocié : décodeur piloté à {} Hz / {} ch (au lieu de {} Hz / {} ch CPAL)",
                            fmt.sample_rate, fmt.channels, output_sample_rate, output_channels
                        );
                        output_sample_rate = fmt.sample_rate;
                        output_channels = fmt.channels;
                    }
                    Err(e) => {
                        log::warn!(
                            "🎚️  WASAPI pré-négociation échouée ({}), décodeur reste au rate CPAL {} Hz",
                            e, output_sample_rate
                        );
                    }
                }
            }
        }

        // CPAL buffer size : sur VM / Minimal profile on demande un buffer
        // BEAUCOUP plus gros (~500 ms) pour absorber les délais de scheduling.
        // ATTENTION : tous les drivers ne supportent pas BufferSize::Fixed
        // (ex : certaines configs PulseAudio refusent, ou la valeur dépasse
        // la plage supportée par le device). On RETRY avec Default si Fixed échoue.
        let profile_for_cpal = crate::core::audio_quality::current_profile();
        let desired_buffer_size = match profile_for_cpal {
            crate::core::audio_quality::AudioQualityProfile::Minimal => {
                Some((output_sample_rate as f32 * 0.5) as u32) // 500 ms
            }
            crate::core::audio_quality::AudioQualityProfile::Low => {
                Some((output_sample_rate as f32 * 0.2) as u32) // 200 ms
            }
            _ => None,
        };

        // Clamp dans la range supportée par le device. Si le device n'accepte
        // pas Fixed (ex: certains drivers Intel HDA virtuels), on retombe sur
        // Default pour éviter un build_output_stream qui échoue.
        let buffer_size_choice = match (desired_buffer_size, output_config.buffer_size()) {
            (Some(n), cpal::SupportedBufferSize::Range { min, max }) => {
                let clamped = n.clamp(*min, *max);
                if clamped != n {
                    log::info!(
                        "🔊 Buffer demandé {} clampé à {} (range device {}-{})",
                        n, clamped, min, max
                    );
                }
                cpal::BufferSize::Fixed(clamped)
            }
            (Some(n), cpal::SupportedBufferSize::Unknown) => {
                // Range inconnue : on tente Fixed mais on saura pas si ça passe.
                log::info!(
                    "🔊 Buffer demandé {} (range device inconnue, fallback Default si échec)",
                    n
                );
                cpal::BufferSize::Fixed(n)
            }
            _ => cpal::BufferSize::Default,
        };

        let config: cpal::StreamConfig = cpal::StreamConfig {
            channels: output_channels,
            sample_rate: output_sample_rate,
            buffer_size: buffer_size_choice,
        };

        log::debug!(
            "🎚️  Config sortie: {} Hz, {} canaux / Source : {} Hz, {} canaux",
            output_sample_rate, output_channels, source_sample_rate, channels
        );

        // La pipeline info sera émise APRÈS la création du backend audio
        // (WASAPI peut négocier ses propres rate/channels/device_name).

        // ========== PHASE 3 : BUFFERS ==========
        // 1. La source actuelle (0 = LiveDecode, 1 = FullBuffer)
        let current_source: Arc<AtomicU8> = Arc::new(AtomicU8::new(0));

        // 2. Le Ring Buffer (remplace le TripleBuffer)
        // Taille pilotée par le profil de qualité (1s / 1.5s / 2s) pour
        // absorber les stalls CPU. Auto-ajustement VM ↔ natif.
        let profile_sym = crate::core::audio_quality::current_profile();
        let ring_secs_sym = profile_sym.ring_buffer_seconds();
        let ring_buffer_capacity: usize = ((output_sample_rate as f32 * ring_secs_sym) as usize)
            * output_channels as usize;
        let ring_buffer: ringbuf::SharedRb<ringbuf::storage::Heap<f32>> = ringbuf::HeapRb::<f32>::new(ring_buffer_capacity);
        let (producer, mut consumer) = ring_buffer.split();

        log::debug!(
            "🔁 RingBuffer créé avec une capacité de {} samples (~{:.1}s @ {}Hz {}ch, profile {:?})",
            ring_buffer_capacity, ring_secs_sym, output_sample_rate, output_channels, profile_sym,
        );

        // 3. Le Full Buffer (remplace le Mutex<AudioFullBuffer>)
        // On sépare les données (RwLock) du curseur de lecture (Atomic)
        // Le RwLock est "safe" ici car une fois le fichier entièrement décodé,
        // le thread audio ne fera que des `.read().unwrap()`, ce qui n'est pas bloquant entre lecteurs.
        let full_buffer_data: Arc<std::sync::RwLock<Vec<_>>> = Arc::new(std::sync::RwLock::new(Vec::new()));
        let full_buffer_cursor: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
        let is_full_buffer_ready: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));

        // 4. Position de lecture (remplace le Mutex<f64>)
        // En lock-free, on préfère compter les samples (entiers) avec un AtomicUsize.
        // Tu pourras convertir ça en secondes côté interface : frames / sample_rate
        let current_position_frames: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));

        // 5. Seek pending — atomique dédiée pour communiquer la target d'un seek
        // entre Phase 6 (loop de fin de lecture) et le callback CPAL. Utilisée
        // uniquement en mode FullBuffer (post pre-decode) : Phase 6 ne peut
        // pas écrire current_position_frames directement car le callback CPAL
        // l'écrit aussi (auto-update depuis cursor) → race condition. Avec
        // cet atomique dédié single-writer (Phase 6) single-reader (CPAL),
        // pas de race possible. Valeur MAX = aucun seek en attente.
        let pending_seek_frames: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(usize::MAX));

        // ========== PHASE 4 : LANCER LE DECODER THREAD ==========
        let need_resampling: bool = source_sample_rate != output_sample_rate;
        log::debug!(
            "🔄 Resampling nécessaire: {}",
            if need_resampling { "OUI" } else { "NON" }
        );

        let is_stopped_clone_decoder: Arc<AtomicBool> = is_stopped.clone();
        let current_source_clone_decoder: Arc<AtomicU8> = current_source.clone();
        let current_position_frames_decoder: Arc<AtomicUsize> = current_position_frames.clone();
        let full_buffer_data_writer: Arc<std::sync::RwLock<Vec<f32>>> = full_buffer_data.clone();
        let is_full_buffer_ready_writer: Arc<AtomicBool> = is_full_buffer_ready.clone();
        let seek_position_decoder: Arc<AtomicU64> = seek_position.clone();
        let seek_flush: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
        let seek_flush_decoder: Arc<AtomicBool> = seek_flush.clone();
        let total_duration_for_decoder: Arc<AtomicU64> = total_duration.clone();

        // Extraits pour la pipeline info émise après audio_output.start() —
        // codec_params est déplacé dans le thread décodeur juste après.
        let pipeline_codec = codec_params.codec;
        let pipeline_source_bits = codec_params.bits_per_sample.unwrap_or(16);

        let decoder_handle: JoinHandle<Result<(), String>> = std::thread::spawn(move || {
            decode_thread(
                format,
                codec_params,
                track_id,
                source_sample_rate,
                output_sample_rate,
                channels,
                output_channels,
                producer, // On passe directement le RingBuffer
                full_buffer_data_writer,
                is_full_buffer_ready_writer,
                current_source_clone_decoder,
                current_position_frames_decoder,
                is_stopped_clone_decoder,
                seek_position_decoder,
                source_sample_rate,
                seek_flush_decoder,
                total_duration_for_decoder,
            )
        });

        // ========== PHASE 5 : DEMARRAGE DU STREAM ==========

        // Stratégie d'attente selon le profil :
        //   - Minimal : on attend que le décodeur ait FINI ENTIÈREMENT (pre-decode
        //     complet en RAM). Une fois fini, CPAL ne fera plus aucun décodage,
        //     juste du memcpy depuis le full_buffer. Garantit la lecture sur les
        //     VMs / CPU saturé où le décodage temps-réel ne tient pas.
        //   - Autres profils : on attend juste 10% de pré-remplissage du ring
        //     buffer, démarrage rapide (~50-300ms).
        let force_full_decode = matches!(
            profile_sym,
            crate::core::audio_quality::AudioQualityProfile::Minimal
        );

        if force_full_decode {
            log::info!("⏳ Profil Minimal : pré-décodage complet du fichier en RAM avant lecture...");
            let _ = app_handle.emit("playback-preparing", true);
            let preparing_start = std::time::Instant::now();

            // Estimation du total decodé (f32 stéréo) = durée × rate × ch × 4 octets.
            let total_duration_secs = f64::from_bits(total_duration.load(Ordering::Relaxed));
            let total_bytes = (total_duration_secs.max(0.0)
                * output_sample_rate as f64
                * output_channels as f64
                * 4.0) as u64;

            // Poll jusqu'à fin du décodage. Le decoder s'arrête naturellement
            // quand il a traité tous les packets du fichier.
            let mut last_emit = std::time::Instant::now();
            while !decoder_handle.is_finished() {
                if is_stopped.load(Ordering::Relaxed) {
                    log::debug!("⏹ Stop demandé pendant la préparation");
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(100));

                // Emit progress toutes les 200 ms (cohérent avec le polling
                // d'autres tâches dans la StatusBar).
                if last_emit.elapsed().as_millis() >= 200 {
                    last_emit = std::time::Instant::now();
                    let decoded_bytes = full_buffer_data
                        .read()
                        .map(|fb| fb.len() as u64 * 4)
                        .unwrap_or(0);
                    let _ = app_handle.emit(
                        "playback-preparing-progress",
                        serde_json::json!({
                            "decoded_bytes": decoded_bytes,
                            "total_bytes": total_bytes,
                        }),
                    );
                }
            }

            log::info!(
                "✅ Pré-décodage terminé en {:.2}s",
                preparing_start.elapsed().as_secs_f64()
            );

            // Force le mode FullBuffer : CPAL lira directement depuis full_buffer_data.
            is_full_buffer_ready.store(true, Ordering::Relaxed);
            current_source.store(1, Ordering::Relaxed);

            let _ = app_handle.emit("playback-preparing", false);
        } else {
            log::debug!("⏳ Attente du premier bloc audio...");
            let start_wait: std::time::Instant = std::time::Instant::now();

            loop {
                let source_mode: u8 = current_source.load(Ordering::Relaxed);
                if source_mode == 1 || consumer.occupied_len() > ring_buffer_capacity / 10 {
                    log::debug!("✅ Premier bloc prêt après {:?}", start_wait.elapsed());
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
                if start_wait.elapsed().as_secs() > 2 {
                    log::debug!("⚠️ Timeout d'attente du premier bloc, démarrage quand même...");
                    break;
                }
            }
        }

        is_playing.store(true, Ordering::SeqCst);

        // ─── Backend audio (CPAL shared OU WASAPI exclusive) ───
        // Déléguée au module `output` qui résout la préférence utilisateur
        // (toggle dans Réglages), tente WASAPI avec fallback automatique
        // sur CPAL si la négociation échoue, et expose le trait AudioOutput.
        use crate::core::audio_player::output;
        let playback_atomics = output::PlaybackAtomics {
            is_paused: is_paused.clone(),
            is_stopped: is_stopped.clone(),
            volume: volume.clone(),
            current_position_frames: current_position_frames.clone(),
        };
        let symphonia_shared = output::SymphoniaSharedState {
            full_buffer_data: full_buffer_data.clone(),
            full_buffer_cursor: full_buffer_cursor.clone(),
            is_full_buffer_ready: is_full_buffer_ready.clone(),
            current_source: current_source.clone(),
            seek_flush: seek_flush.clone(),
            pending_seek_frames: pending_seek_frames.clone(),
            output_channels,
        };

        let mut audio_output = output::create_symphonia_output(
            output::current_preference(),
            source_sample_rate,
            channels as u16,
            device,
            config,
            device_name.clone(),
            playback_atomics,
            symphonia_shared,
            consumer,
        )
        .map_err(|e| -> Box<dyn std::error::Error> { format!("AudioOutput: {e}").into() })?;

        audio_output
            .start()
            .map_err(|e| -> Box<dyn std::error::Error> { format!("AudioOutput start: {e}").into() })?;

        log::debug!("▶️ Lecture en cours via {}", audio_output.backend().display_name());

        // ─── Pipeline info to the frontend (for the player status bar) ───
        // Émise après la création du backend pour refléter les vraies valeurs
        // (WASAPI exclusive matche typiquement le source rate, CPAL shared
        // utilise le sample rate configuré du device Windows).
        let profile_for_pipeline = crate::core::audio_quality::current_profile();
        let effective_output_rate = audio_output.output_sample_rate();
        let effective_output_channels = audio_output.output_channels();
        let effective_device_name = audio_output.device_name().to_string();
        let backend_label = audio_output.backend().display_name().to_string();
        let bit_perfect = audio_output.backend().is_bit_perfect_capable()
            && effective_output_rate == source_sample_rate;
        crate::core::audio_player::pipeline_info::PlaybackPipelineInfo {
            source_format: crate::core::audio_player::pipeline_info::symphonia_format_label(
                pipeline_codec,
            )
            .to_string(),
            source_sample_rate,
            source_bits: pipeline_source_bits,
            source_channels: channels as u8,
            intermediate_pcm_rate: None,
            dsd_filter_taps: None,
            dsd_decimation: None,
            output_sample_rate: effective_output_rate,
            output_channels: effective_output_channels as u8,
            device_name: effective_device_name,
            resampler_active: source_sample_rate != effective_output_rate,
            quality_profile: format!("{:?}", profile_for_pipeline).to_lowercase(),
            backend: backend_label,
            bit_perfect,
        }
        .emit(&app_handle);

        // Safety : re-émettre preparing: false après le démarrage effectif
        // de CPAL au cas où le précédent emit (Minimal pre-decode) aurait été
        // perdu ou doublé par le frontend.
        let _ = app_handle.emit("playback-preparing", false);

        // ========== PHASE 6 : ATTENTE FIN ==========

        match decoder_handle.join() {
            Ok(Ok(_)) => log::debug!("🎵 Décodage terminé"),
            Ok(Err(e)) => log::error!("❌ Erreur thread décodage: {e}"),
            Err(_) => log::error!("❌ Panic dans le thread de décodage"),
        }

        // ⭐ CRITIQUE: Attendre que le FullBuffer soit complètement lu
        // Le decoder est terminé mais il reste peut-être des données dans le buffer
        log::debug!("⏳ Attente de la fin de lecture du buffer...");

        loop {
            std::thread::sleep(std::time::Duration::from_millis(100));

            // ─── SEEK pendant Phase 6 (decoder mort, FullBuffer en lecture) ───
            // Le decoder thread est terminé : la lecture audio est intégralement
            // pilotée par le callback CPAL qui lit `full_buffer_data` à
            // l'offset `full_buffer_cursor`. Pour seek, on écrit directement
            // le nouveau curseur, et on signale `seek_flush` pour que CPAL
            // vide ce qu'il a en cours (fade-in + silence le prochain
            // callback). On passe AUSSI par `pending_seek_frames` pour que
            // CPAL re-confirme la position (au cas où sa boucle d'auto-update
            // de `current_position_frames` lui ferait écraser notre écriture).
            let seek_bits = seek_position.load(Ordering::Relaxed);
            if seek_bits != u64::MAX {
                seek_position.store(u64::MAX, Ordering::Relaxed);
                let total_dur = f64::from_bits(total_duration.load(Ordering::Relaxed));
                let seek_seconds = f64::from_bits(seek_bits).max(0.0).min(total_dur.max(0.0));
                let new_frames = (seek_seconds * output_sample_rate as f64) as usize;
                let new_cursor = new_frames * output_channels as usize;

                // 1. Curseur FullBuffer : déplacement direct. Phase 6 est le
                //    seul writer (decoder mort, CPAL fait fetch_add depuis
                //    cette nouvelle base).
                full_buffer_cursor.store(new_cursor, Ordering::Release);

                // 2. Pending seek : indique à CPAL la cible exacte pour qu'il
                //    re-sync current_position_frames même si son auto-update
                //    a déjà tourné.
                pending_seek_frames.store(new_frames, Ordering::Release);

                // 3. Position frontend : maj immédiate pour que la UI ne
                //    revienne pas en arrière le temps que CPAL re-store.
                current_position_frames.store(new_frames, Ordering::Release);
                let new_pos_secs = new_frames as f64 / output_sample_rate as f64;
                current_position.store(new_pos_secs.to_bits(), Ordering::Relaxed);

                // 4. Seek flush : signal à CPAL → drain ring + fade-in.
                seek_flush.store(true, Ordering::Release);

                log::debug!("⏩ Seek Phase6 → {:.2}s ({} frames, cursor={})", seek_seconds, new_frames, new_cursor);
                continue; // Ne pas vérifier is_end ce tour-ci
            }

            let source: u8 = current_source.load(Ordering::Relaxed);
            let mut is_end: bool = false;

            if source == 1 {
                // En mode FullBuffer, on vérifie si le curseur a atteint la fin
                if let Ok(fb) = full_buffer_data.read() {
                    let cursor: usize = full_buffer_cursor.load(Ordering::Relaxed);
                    is_end = cursor >= fb.len();
                }
            } else {
                // En mode LiveDecode, le décodeur a fini.
                // Court délai pour que CPAL vide les derniers samples du ring buffer
                std::thread::sleep(std::time::Duration::from_millis(100));
                is_end = true;
            }

            // Mise à jour de la position pour le frontend (10x par seconde)
            let frames: usize = current_position_frames.load(Ordering::Relaxed);
            let new_pos: f64 = frames as f64 / output_sample_rate as f64;
            current_position.store(new_pos.to_bits(), Ordering::Relaxed);

            if is_end || is_stopped.load(Ordering::Relaxed) {
                // On vide le buffer final
                if let Ok(mut fb) = full_buffer_data.write() {
                    fb.clear();
                }
                break;
            }
        }

        log::debug!("✅ Lecture terminée");

        drop(audio_output);
        log::debug!("🧹 Fin de lecture - backend audio libéré");

        is_stream_alive.store(false, Ordering::SeqCst);
        
        //Envoyer un signal au frontend
        if !is_stopped.load(Ordering::Relaxed) {
            if let Err(e) = app_handle.emit("playback-ended", file_path.to_string_lossy().to_string()) {
                log::error!("❌ Erreur d'envoi de l'event Tauri : {}", e);
            }
        }

        is_stopped.store(false, Ordering::SeqCst);
        is_playing.store(false, Ordering::SeqCst);
        current_position.store(0.0_f64.to_bits(), Ordering::Relaxed);
        current_position_frames.store(0, Ordering::Relaxed);
        full_buffer_cursor.store(0, Ordering::Relaxed);

        Ok(())
    }

    // ============================================================================
    // DSD FILE THREAD — version simplifiée pour lire les .dsf
    // Pipeline : DsdDecoder → DsdToPcmConverter → Resampler → RingBuffer → CPAL
    // Pas de FullBuffer (overkill pour DSD : 80 Mo en RAM pour 4 min stéréo).
    // ============================================================================
    fn play_dsd_file_thread(
        app_handle: AppHandle,
        file_path: PathBuf,
        is_paused: Arc<AtomicBool>,
        is_playing: Arc<AtomicBool>,
        is_stopped: Arc<AtomicBool>,
        is_stream_alive: Arc<AtomicBool>,
        current_position: Arc<AtomicU64>,
        total_duration: Arc<AtomicU64>,
        volume: Arc<AtomicU8>,
        selected_device_name: Option<String>,
        seek_position: Arc<AtomicU64>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        log::debug!("🎵 [DSD] Fichier sélectionné : {:?}", file_path);

        // ─── 1. Open the right container reader (DSF or DFF) → boxed trait object ───
        let ext = file_path
            .extension()
            .and_then(|s| s.to_str())
            .map(|e| e.to_ascii_lowercase())
            .unwrap_or_default();

        let decoder: Box<dyn DsdContainerReader + Send> = match ext.as_str() {
            "dsf" => Box::new(DsfDecoder::open(&file_path)?),
            "dff" => Box::new(DffDecoder::open(&file_path)?),
            other => {
                return Err(format!(
                    "Extension {:?} non supportée par le pipeline DSD",
                    other
                )
                .into())
            }
        };

        let dsd_rate = decoder.sample_rate();
        let channel_count = decoder.channel_count();
        let duration = decoder.duration_seconds();
        total_duration.store(duration.to_bits(), Ordering::Relaxed);

        log::debug!(
            "🎵 [DSD/{}] Source : {} ch, DSD {} Hz, {:.2}s",
            ext.to_uppercase(),
            channel_count,
            dsd_rate,
            duration
        );

        // ─── 2. CPAL output device + config (mêmes choix que Symphonia path) ───
        let host = cpal::default_host();
        let device: cpal::Device = if let Some(ref name) = selected_device_name {
            let found = host
                .output_devices()?
                .find(|d| {
                    if let Ok(desc) = d.description() {
                        let dn = desc.name().to_string();
                        let display = match desc.manufacturer() {
                            Some(mfr) => format!("{} ({})", dn, mfr),
                            None => dn.clone(),
                        };
                        dn == *name || display == *name
                    } else {
                        false
                    }
                });
            match found {
                Some(d) => d,
                None => {
                    log::error!("Device '{}' introuvable, fallback default", name);
                    host.default_output_device()
                        .ok_or("Pas de périphérique audio")?
                }
            }
        } else {
            host.default_output_device()
                .ok_or("Pas de périphérique audio")?
        };

        // ─── Décision DSD natif (DoP) — Windows uniquement ───
        // DoP si : préférence activée + WASAPI exclusive actif + profil non
        // Minimal + le DAC accepte le format porteur (24-bit au rate DSD/16).
        // Sinon on continue sur le chemin DSD2PCM classique.
        #[cfg(target_os = "windows")]
        {
            use crate::core::audio_player::output;
            let profile_dop = crate::core::audio_quality::current_profile();
            let is_minimal = matches!(
                profile_dop,
                crate::core::audio_quality::AudioQualityProfile::Minimal
            );
            let dop_wanted = output::dop_enabled()
                && matches!(
                    output::current_preference(),
                    output::AudioBackend::WasapiExclusive
                )
                && !is_minimal;

            if dop_wanted {
                let carrier = crate::core::audio_decoder::dsd::dop_encoder::dop_carrier_rate(dsd_rate);
                // Nom complet "Nom (Fabricant)" pour cibler le bon DAC en WASAPI.
                let device_full_name = device
                    .description()
                    .ok()
                    .map(|d| {
                        let name = d.name().to_string();
                        match (d.manufacturer(), d.driver()) {
                            (Some(mfr), _) => format!("{} ({})", name, mfr),
                            (_, Some(drv)) => format!("{} ({})", name, drv),
                            _ => name,
                        }
                    })
                    .unwrap_or_else(|| "Périphérique audio".to_string());

                let supported = crate::core::audio_player::audio_output_wasapi::dop_format_supported(
                    carrier,
                    channel_count as u16,
                    Some(device_full_name.clone()),
                );

                if supported {
                    log::info!(
                        "🎚️  DSD natif (DoP) activé : {} → porteur {} Hz sur '{}'",
                        crate::core::audio_player::pipeline_info::dsd_label(dsd_rate),
                        carrier,
                        device_full_name
                    );
                    let lsb_first = ext == "dsf";
                    return Self::run_dsd_dop_thread(
                        app_handle,
                        decoder,
                        file_path,
                        lsb_first,
                        dsd_rate,
                        channel_count,
                        carrier,
                        device_full_name,
                        duration,
                        is_paused,
                        is_playing,
                        is_stopped,
                        is_stream_alive,
                        current_position,
                        total_duration,
                        seek_position,
                    );
                } else {
                    log::warn!(
                        "🎚️  DoP non supporté par le DAC au porteur {} Hz, fallback DSD2PCM",
                        carrier
                    );
                }
            }
        }

        // ─── Décision DSD natif (DoP) — Linux via ALSA hw exclusif ───
        // DoP si : préférence activée + profil non Minimal + le device
        // sélectionné mappe vers un `hw:` ALSA qui accepte S32_LE @ carrier en
        // accès exclusif. Sinon on continue sur le chemin DSD2PCM classique.
        #[cfg(target_os = "linux")]
        {
            use crate::core::audio_player::output;
            let profile_dop = crate::core::audio_quality::current_profile();
            let is_minimal = matches!(
                profile_dop,
                crate::core::audio_quality::AudioQualityProfile::Minimal
            );
            if output::dop_enabled() && !is_minimal {
                let carrier = crate::core::audio_decoder::dsd::dop_encoder::dop_carrier_rate(dsd_rate);
                let device_full_name = device
                    .description()
                    .ok()
                    .map(|d| {
                        let name = d.name().to_string();
                        match (d.manufacturer(), d.driver()) {
                            (Some(mfr), _) => format!("{} ({})", name, mfr),
                            (_, Some(drv)) => format!("{} ({})", name, drv),
                            _ => name,
                        }
                    })
                    .unwrap_or_else(|| "Périphérique audio".to_string());

                // Résoudre le hw: ALSA : le choix EXPLICITE de l'utilisateur
                // (selected_device_name) d'abord — car `device_full_name` peut
                // avoir défauté sur un autre périphérique si le lookup CPAL a
                // échoué (→ résoudrait la mauvaise carte).
                let hw_id = selected_device_name
                    .as_deref()
                    .and_then(output::dop_alsa::resolve_hw_id)
                    .or_else(|| output::dop_alsa::resolve_hw_id(&device_full_name));

                if let Some(hw_id) = hw_id {
                    // Réserver la carte via D-Bus AVANT la probe : PipeWire/Pulse
                    // relâche le `hw:` (sinon ouverture exclusive = EBUSY). La
                    // réservation est gardée vivante pendant toute la lecture et
                    // relâchée à la fin (→ PipeWire reprend la carte).
                    let reservation = output::device_reservation::card_index_from_hw_id(&hw_id)
                        .and_then(|idx| {
                            match output::device_reservation::DeviceReservation::acquire(idx) {
                                Ok(r) => Some(r),
                                Err(e) => {
                                    log::warn!("🔒 Réservation carte {idx} échouée ({e}) — tentative sans");
                                    None
                                }
                            }
                        });

                    if output::dop_alsa::dop_format_supported(&hw_id, carrier, channel_count as u16) {
                        log::info!(
                            "🎚️  DSD natif (DoP) ALSA activé : {} → porteur {} Hz sur '{}' ({})",
                            crate::core::audio_player::pipeline_info::dsd_label(dsd_rate),
                            carrier,
                            device_full_name,
                            hw_id
                        );
                        let lsb_first = ext == "dsf";
                        return Self::run_dsd_dop_alsa_thread(
                            app_handle,
                            decoder,
                            file_path,
                            lsb_first,
                            dsd_rate,
                            channel_count,
                            carrier,
                            hw_id,
                            device_full_name,
                            duration,
                            is_paused,
                            is_playing,
                            is_stopped,
                            is_stream_alive,
                            current_position,
                            total_duration,
                            seek_position,
                            reservation,
                        );
                    } else {
                        log::warn!(
                            "🎚️  DoP ALSA non supporté au porteur {} Hz (device occupé ou format refusé), fallback DSD2PCM",
                            carrier
                        );
                        // `reservation` est droppée ici → PipeWire reprend la carte.
                    }
                } else {
                    log::debug!(
                        "🎚️  Aucun hw: ALSA résolu pour '{}', fallback DSD2PCM",
                        device_full_name
                    );
                }
            }
        }

        // ─── Décision DSD natif (DoP) — macOS via CoreAudio (hog mode) ───
        // DoP si : préférence activée + profil non Minimal + le device
        // sélectionné se résout en AudioDeviceID acceptant le rate porteur.
        // Rappel : CoreAudio n'a AUCUN transport DSD natif, le DoP est donc la
        // seule voie possible sur Mac — et un DAC class-compliant (UAC2) suffit,
        // aucun pilote propriétaire requis. Un DAC UAC1 (≤ 96 kHz) ne déclarera
        // pas le porteur → `dop_format_supported` échoue → fallback DSD2PCM.
        #[cfg(target_os = "macos")]
        {
            use crate::core::audio_player::output;
            let profile_dop = crate::core::audio_quality::current_profile();
            let is_minimal = matches!(
                profile_dop,
                crate::core::audio_quality::AudioQualityProfile::Minimal
            );
            if output::dop_enabled() && !is_minimal {
                let carrier = crate::core::audio_decoder::dsd::dop_encoder::dop_carrier_rate(dsd_rate);
                let device_full_name = device
                    .description()
                    .ok()
                    .map(|d| {
                        let name = d.name().to_string();
                        match (d.manufacturer(), d.driver()) {
                            (Some(mfr), _) => format!("{} ({})", name, mfr),
                            (_, Some(drv)) => format!("{} ({})", name, drv),
                            _ => name,
                        }
                    })
                    .unwrap_or_else(|| "Périphérique audio".to_string());

                // Même ordre de priorité que le DoP ALSA : le choix EXPLICITE de
                // l'utilisateur d'abord, car `device_full_name` peut avoir
                // défauté sur un autre périphérique si le lookup CPAL a échoué.
                let device_id = selected_device_name
                    .as_deref()
                    .and_then(output::dop_coreaudio::resolve_device_id)
                    .or_else(|| output::dop_coreaudio::resolve_device_id(&device_full_name));

                if let Some(device_id) = device_id {
                    if output::dop_coreaudio::dop_format_supported(
                        device_id,
                        carrier,
                        channel_count as u16,
                    ) {
                        log::info!(
                            "🎚️  DSD natif (DoP) CoreAudio activé : {} → porteur {} Hz sur '{}' (device {})",
                            crate::core::audio_player::pipeline_info::dsd_label(dsd_rate),
                            carrier,
                            device_full_name,
                            device_id
                        );
                        let lsb_first = ext == "dsf";
                        return Self::run_dsd_dop_coreaudio_thread(
                            app_handle,
                            decoder,
                            file_path,
                            lsb_first,
                            dsd_rate,
                            channel_count,
                            carrier,
                            device.clone(),
                            device_id,
                            device_full_name,
                            is_paused,
                            is_playing,
                            is_stopped,
                            is_stream_alive,
                            current_position,
                            total_duration,
                            seek_position,
                        );
                    } else {
                        // Le détail du refus (canaux ou rates déclarés) est
                        // logué juste au-dessus par `dop_format_supported`.
                        log::warn!(
                            "🎚️  DoP CoreAudio non supporté au porteur {} Hz, fallback DSD2PCM",
                            carrier
                        );
                    }
                } else {
                    log::debug!(
                        "🎚️  Aucun AudioDeviceID résolu pour '{}', fallback DSD2PCM",
                        device_full_name
                    );
                }
            }
        }

        // On ne joue PAS ce DSD en DoP (toggle off, format non supporté, ou
        // Minimal) → fermer un éventuel moteur DoP vivant (le DAC repasse en PCM).
        #[cfg(target_os = "windows")]
        crate::core::audio_player::output::dop_engine::teardown_engine();

        let output_config = device.default_output_config()?;
        let output_sample_rate = output_config.sample_rate();
        let output_channels = output_config.channels();

        // Buffer hardware CPAL : gros sur profil contraint pour tolérer les
        // stalls OS scheduling. Cf. raisonnement détaillé dans play_file_thread.
        let dsd_profile_for_cpal = crate::core::audio_quality::current_profile();
        let dsd_buffer_size = match dsd_profile_for_cpal {
            crate::core::audio_quality::AudioQualityProfile::Minimal => {
                cpal::BufferSize::Fixed((output_sample_rate as f32 * 0.5) as u32)
            }
            crate::core::audio_quality::AudioQualityProfile::Low => {
                cpal::BufferSize::Fixed((output_sample_rate as f32 * 0.2) as u32)
            }
            _ => cpal::BufferSize::Default,
        };

        let config = cpal::StreamConfig {
            channels: output_channels,
            sample_rate: output_sample_rate,
            buffer_size: dsd_buffer_size,
        };

        log::debug!(
            "🎚️  [DSD] Config sortie: {} Hz × {} ch / Source : DSD {} Hz × {} ch",
            output_sample_rate, output_channels, dsd_rate, channel_count
        );

        // ─── Pipeline info to the frontend (for the player status bar) ───
        // For DSD the intermediate PCM rate is decided by the active profile
        // (44.1k on Low, 88.2k on High/Medium). The resampler is active iff
        // that intermediate rate differs from the device rate.
        let profile_for_pipeline = crate::core::audio_quality::current_profile();
        let intermediate_pcm_rate = profile_for_pipeline.dsd_target_rate(dsd_rate);
        let dsd_device_name = device
            .description()
            .ok()
            .map(|d| d.name().to_string())
            .unwrap_or_else(|| "Périphérique audio".to_string());
        crate::core::audio_player::pipeline_info::PlaybackPipelineInfo {
            source_format: crate::core::audio_player::pipeline_info::dsd_label(dsd_rate),
            source_sample_rate: dsd_rate,
            source_bits: 1,
            source_channels: channel_count as u8,
            intermediate_pcm_rate: Some(intermediate_pcm_rate),
            dsd_filter_taps: Some((profile_for_pipeline.dsd_filter_window_bytes() * 8) as u32),
            dsd_decimation: Some(dsd_rate / intermediate_pcm_rate),
            output_sample_rate,
            output_channels: output_channels as u8,
            device_name: dsd_device_name,
            resampler_active: intermediate_pcm_rate != output_sample_rate,
            quality_profile: format!("{:?}", profile_for_pipeline).to_lowercase(),
            // DSD passe par CPAL (WASAPI exclusive DSD non supporté ici).
            backend: crate::core::audio_player::output::AudioBackend::CpalShared
                .display_name()
                .to_string(),
            bit_perfect: false,
        }
        .emit(&app_handle);

        // ─── 3. Ring buffer ───
        // Taille pilotée par le profil :
        //   - Minimal : on dimensionne le buffer pour CONTENIR TOUT LE MORCEAU
        //     (+ 2 s de marge). Le décodeur DSD remplira tout, et CPAL n'aura
        //     plus qu'à drainer en memcpy. Pre-decode complet, garanti sans
        //     underrun même sur VM saturée.
        //   - Autres : taille par durée (1-5s) → pas assez pour pre-decode
        //     mais permet de lancer la lecture rapidement.
        let profile = crate::core::audio_quality::current_profile();
        let force_full_decode = matches!(
            profile,
            crate::core::audio_quality::AudioQualityProfile::Minimal
        );
        let ring_secs = if force_full_decode {
            (duration as f32) + 2.0
        } else {
            profile.ring_buffer_seconds()
        };
        let ring_capacity = ((output_sample_rate as f32 * ring_secs)
            as usize)
            * output_channels as usize;
        log::debug!(
            "🔁 [DSD] Ring buffer : {:.1}s ({} samples, profile {:?}, full_decode={})",
            ring_secs, ring_capacity, profile, force_full_decode
        );
        let ring_buffer = ringbuf::HeapRb::<f32>::new(ring_capacity);
        let (producer, mut consumer) = ring_buffer.split();

        // ─── 4. Atomics partagées avec la callback CPAL ───
        let current_position_frames = Arc::new(AtomicUsize::new(0));
        let seek_flush = Arc::new(AtomicBool::new(false));

        // ─── 5. Spawn DSD playback thread ───
        let producer_for_thread = producer;
        let is_stopped_dec = is_stopped.clone();
        let seek_position_dec = seek_position.clone();
        let current_position_frames_dec = current_position_frames.clone();
        let total_duration_dec = total_duration.clone();
        let seek_flush_dec = seek_flush.clone();

        let decoder_handle = std::thread::spawn(move || {
            run_dsd_playback(
                decoder,
                output_sample_rate,
                output_channels,
                producer_for_thread,
                is_stopped_dec,
                seek_position_dec,
                current_position_frames_dec,
                total_duration_dec,
                seek_flush_dec,
            )
        });

        // ─── 6. Wait for pre-fill (Minimal = pre-decode complet) ───
        // Stratégie selon le profil :
        //   - Minimal : attendre que le thread décodeur ait FINI (pre-decode
        //     complet du fichier dans le ring buffer). Garanti sans underrun.
        //   - Autres  : attendre `pre_fill_ratio` de remplissage (25-40 %) puis
        //     lancer CPAL — démarrage rapide, lecture en streaming.
        if force_full_decode {
            log::info!("⏳ [DSD] Profil Minimal : pré-décodage complet du fichier en RAM...");
            let _ = app_handle.emit("playback-preparing", true);
            let prep_start = std::time::Instant::now();
            // Total = durée × sample rate × ch × 4 octets (f32).
            let total_bytes = (duration
                * output_sample_rate as f64
                * output_channels as f64
                * 4.0) as u64;
            let mut last_emit = std::time::Instant::now();
            while !decoder_handle.is_finished() {
                if is_stopped.load(Ordering::Relaxed) {
                    log::debug!("⏹ Stop pendant pre-decode DSD");
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(100));

                // Emit progress toutes les 200 ms.
                if last_emit.elapsed().as_millis() >= 200 {
                    last_emit = std::time::Instant::now();
                    let decoded_bytes = consumer.occupied_len() as u64 * 4;
                    let _ = app_handle.emit(
                        "playback-preparing-progress",
                        serde_json::json!({
                            "decoded_bytes": decoded_bytes,
                            "total_bytes": total_bytes,
                        }),
                    );
                }
            }
            log::info!(
                "✅ [DSD] Pré-décodage terminé en {:.2}s",
                prep_start.elapsed().as_secs_f64()
            );

            // ─── 6b. Drain ring buffer → Vec<f32> (FullBuffer mode) ───
            // Le ring buffer contient tout le fichier décodé (consumer.occupied_len()
            // = nombre total de samples f32 stéréo). On le vide dans un Vec<f32>
            // pour que CPAL puisse y lire à n'importe quel index → seek instantané.
            // Sans ça, le ring buffer est FIFO et le seek arrière est impossible.
            let drain_len = consumer.occupied_len();
            let mut drained: Vec<f32> = vec![0.0f32; drain_len];
            let drained_n = consumer.pop_slice(&mut drained);
            drop(consumer); // plus utilisé en mode FullBuffer
            // Si pop_slice a retourné moins que prévu (peu probable, mais
            // possible si stop pendant pre-decode), on tronque pour ne pas
            // jouer des zéros à la fin du buffer.
            drained.truncate(drained_n);
            log::debug!(
                "📦 [DSD-FullBuffer] {} samples drainés ({:.1} Mo), prêt pour seek instantané",
                drained_n,
                drained.len() as f32 * 4.0 / 1024.0 / 1024.0
            );

            let full_buffer: Arc<Vec<f32>> = Arc::new(drained);
            let full_buffer_cursor: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));

            // Le décodeur a déjà terminé son travail (pre-decode complet). On
            // joint maintenant pour libérer le handle ; il ne fera plus rien.
            match decoder_handle.join() {
                Ok(Ok(_)) => log::debug!("🎵 [DSD-FullBuffer] Décodeur joint OK"),
                Ok(Err(e)) => log::error!("❌ [DSD-FullBuffer] Erreur thread: {}", e),
                Err(_) => log::error!("❌ Panic dans le thread DSD"),
            }

            let _ = app_handle.emit("playback-preparing", false);

            // ─── 7. Build CPAL stream FullBuffer ───
            is_playing.store(true, Ordering::SeqCst);

            let is_paused_cpal = is_paused.clone();
            let is_stopped_cpal = is_stopped.clone();
            let current_position_frames_cpal = current_position_frames.clone();
            let volume_cpal = volume.clone();
            let seek_flush_cpal = seek_flush.clone();
            let full_buffer_cpal = full_buffer.clone();
            let full_buffer_cursor_cpal = full_buffer_cursor.clone();
            let mut fade_in_samples: usize = 0;
            let cpal_error_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let output_channels_cpal = output_channels;

            let stream = device.build_output_stream(
                config.clone(),
                move |output: &mut [f32], _| {
                    if is_paused_cpal.load(Ordering::Relaxed)
                        || is_stopped_cpal.load(Ordering::Relaxed)
                    {
                        output.fill(0.0);
                        return;
                    }

                    // Seek flush : reposition du curseur (Phase 8 a déjà écrit
                    // full_buffer_cursor + current_position_frames, ici on
                    // ajoute juste le fade-in et on silence ce callback pour
                    // éviter le click).
                    if seek_flush_cpal.load(Ordering::Acquire) {
                        seek_flush_cpal.store(false, Ordering::Release);
                        fade_in_samples = 2048;
                        output.fill(0.0);
                        return;
                    }

                    let cursor = full_buffer_cursor_cpal.load(Ordering::Relaxed);
                    let available = full_buffer_cpal.len().saturating_sub(cursor);
                    let samples_read = available.min(output.len());
                    if samples_read > 0 {
                        output[..samples_read]
                            .copy_from_slice(&full_buffer_cpal[cursor..cursor + samples_read]);
                        full_buffer_cursor_cpal.fetch_add(samples_read, Ordering::Relaxed);
                    }
                    if samples_read < output.len() {
                        output[samples_read..].fill(0.0);
                    }

                    // Position : dériver depuis le curseur (source de vérité)
                    if samples_read > 0 && !seek_flush_cpal.load(Ordering::Relaxed) {
                        let c = full_buffer_cursor_cpal.load(Ordering::Relaxed);
                        current_position_frames_cpal
                            .store(c / output_channels_cpal as usize, Ordering::Relaxed);
                    }

                    let vol = volume_cpal.load(Ordering::Relaxed) as f32 / 100.0;
                    for s in output.iter_mut() {
                        let fade = if fade_in_samples > 0 {
                            fade_in_samples -= 1;
                            (2048 - fade_in_samples) as f32 / 2048.0
                        } else {
                            1.0
                        };
                        *s = (*s * vol * fade * 0.98).clamp(-1.0, 1.0);
                    }
                },
                {
                    let cpal_error_count = cpal_error_count.clone();
                    move |err| {
                        let n = cpal_error_count.fetch_add(1, Ordering::Relaxed);
                        if n == 0 || (n < 1000 && n % 100 == 0) || n % 1000 == 0 {
                            log::error!("❌ [DSD-FullBuffer] Erreur CPAL (#{}): {:?}", n + 1, err);
                        }
                    }
                },
                None,
            )?;

            stream.play()?;
            log::debug!("▶️ [DSD-FullBuffer] Lecture en cours...");
            let _ = app_handle.emit("playback-preparing", false);

            // ─── 8. Phase 8 — Wait for end + handle seek (Minimal/FullBuffer) ───
            let total_frames_expected = full_buffer.len() / output_channels as usize;
            loop {
                std::thread::sleep(std::time::Duration::from_millis(100));

                // SEEK : maj directe du curseur + flush pour fade-in.
                let seek_bits = seek_position.load(Ordering::Relaxed);
                if seek_bits != u64::MAX {
                    seek_position.store(u64::MAX, Ordering::Relaxed);
                    let total_dur = f64::from_bits(total_duration.load(Ordering::Relaxed));
                    let seek_seconds = f64::from_bits(seek_bits)
                        .max(0.0)
                        .min(total_dur.max(0.0));
                    let new_frames = (seek_seconds * output_sample_rate as f64) as usize;
                    let new_cursor = new_frames * output_channels as usize;

                    full_buffer_cursor.store(new_cursor, Ordering::Release);
                    current_position_frames.store(new_frames, Ordering::Release);
                    let new_pos_secs = new_frames as f64 / output_sample_rate as f64;
                    current_position.store(new_pos_secs.to_bits(), Ordering::Relaxed);
                    seek_flush.store(true, Ordering::Release);

                    log::debug!(
                        "⏩ [DSD-FullBuffer] Seek → {:.2}s ({} frames, cursor={})",
                        seek_seconds, new_frames, new_cursor
                    );
                    continue;
                }

                let frames = current_position_frames.load(Ordering::Relaxed);
                let new_pos = frames as f64 / output_sample_rate as f64;
                current_position.store(new_pos.to_bits(), Ordering::Relaxed);

                if is_stopped.load(Ordering::Relaxed) || frames >= total_frames_expected {
                    break;
                }
            }

            log::debug!("✅ [DSD-FullBuffer] Lecture terminée");
            drop(stream);
            is_stream_alive.store(false, Ordering::SeqCst);

            if !is_stopped.load(Ordering::Relaxed) {
                if let Err(e) = app_handle.emit(
                    "playback-ended",
                    file_path.to_string_lossy().to_string(),
                ) {
                    log::error!("❌ Erreur d'envoi de l'event Tauri : {}", e);
                }
            }

            is_stopped.store(false, Ordering::SeqCst);
            is_playing.store(false, Ordering::SeqCst);
            current_position.store(0.0_f64.to_bits(), Ordering::Relaxed);
            current_position_frames.store(0, Ordering::Relaxed);

            return Ok(());
        } else {
            let pre_fill_threshold = (ring_capacity as f32 * profile.pre_fill_ratio()) as usize;
            log::debug!("⏳ [DSD] Attente du premier bloc audio...");
            let start_wait = std::time::Instant::now();
            loop {
                if consumer.occupied_len() > pre_fill_threshold {
                    log::debug!("✅ [DSD] Premier bloc prêt après {:?}", start_wait.elapsed());
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
                if start_wait.elapsed().as_secs() > 5 {
                    log::debug!("⚠️ [DSD] Timeout attente premier bloc");
                    break;
                }
            }
        }

        is_playing.store(true, Ordering::SeqCst);

        // ─── 7. Build CPAL stream (LiveDecode-only) ───
        let is_paused_cpal = is_paused.clone();
        let is_stopped_cpal = is_stopped.clone();
        let current_position_frames_cpal = current_position_frames.clone();
        let volume_cpal = volume.clone();
        // Compteur d'erreurs CPAL (underrun, etc.) pour throttler le log :
        // sur VM/CPU faible, on peut en recevoir des centaines par seconde,
        // c'est inutile de toutes les écrire. On log 1 sur 100.
        let cpal_error_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let seek_flush_cpal = seek_flush.clone();
        let mut fade_in_samples: usize = 0;

        let stream = device.build_output_stream(
            config.clone(),
            move |output: &mut [f32], _| {
                if is_paused_cpal.load(Ordering::Relaxed)
                    || is_stopped_cpal.load(Ordering::Relaxed)
                {
                    output.fill(0.0);
                    return;
                }

                if seek_flush_cpal.load(Ordering::Relaxed) {
                    let to_skip = consumer.occupied_len();
                    consumer.skip(to_skip);
                    seek_flush_cpal.store(false, Ordering::Relaxed);
                    fade_in_samples = 2048;
                    output.fill(0.0);
                    return;
                }

                let read = consumer.pop_slice(output);
                // Underrun → silence sur le reste
                for s in output[read..].iter_mut() {
                    *s = 0.0;
                }

                // Position update : LiveDecode = incrément des frames jouées
                if read > 0 {
                    let frames = read / output_channels as usize;
                    current_position_frames_cpal.fetch_add(frames, Ordering::Relaxed);
                }

                // Volume + fade-in post-seek + clipping de sécurité
                let vol = volume_cpal.load(Ordering::Relaxed) as f32 / 100.0;
                for s in output.iter_mut() {
                    let fade = if fade_in_samples > 0 {
                        fade_in_samples -= 1;
                        (2048 - fade_in_samples) as f32 / 2048.0
                    } else {
                        1.0
                    };
                    *s = (*s * vol * fade * 0.98).clamp(-1.0, 1.0);
                }
            },
            {
                let cpal_error_count = cpal_error_count.clone();
                move |err| {
                    let n = cpal_error_count.fetch_add(1, Ordering::Relaxed);
                    // Log la 1re, puis 1/100, puis 1/1000 pour signaler le
                    // problème sans inonder la console.
                    if n == 0 || (n < 1000 && n % 100 == 0) || n % 1000 == 0 {
                        log::error!("❌ [DSD] Erreur CPAL (#{}): {:?}", n + 1, err);
                    }
                }
            },
            None,
        )?;

        stream.play()?;
        log::debug!("▶️ [DSD] Lecture en cours...");

        // Safety : re-émettre preparing: false en cas d'event perdu (cf. path Symphonia).
        let _ = app_handle.emit("playback-preparing", false);

        // ─── 8. Wait for end of decode + drain ring buffer ───
        match decoder_handle.join() {
            Ok(Ok(_)) => log::debug!("🎵 [DSD] Décodage terminé"),
            Ok(Err(e)) => log::error!("❌ [DSD] Erreur thread décodage: {}", e),
            Err(_) => log::error!("❌ [DSD] Panic dans le thread"),
        }

        // Wait for the CPAL callback to drain the ring buffer.
        // We can't peek the consumer from here (it was moved into the closure),
        // so we compare the played frames count against the expected total.
        let total_frames_expected = (duration * output_sample_rate as f64) as usize;
        loop {
            std::thread::sleep(std::time::Duration::from_millis(100));

            // Position update pour le frontend
            let frames = current_position_frames.load(Ordering::Relaxed);
            let new_pos = frames as f64 / output_sample_rate as f64;
            current_position.store(new_pos.to_bits(), Ordering::Relaxed);

            if is_stopped.load(Ordering::Relaxed) || frames >= total_frames_expected {
                break;
            }
        }

        log::debug!("✅ [DSD] Lecture terminée");

        drop(stream);
        is_stream_alive.store(false, Ordering::SeqCst);

        if !is_stopped.load(Ordering::Relaxed) {
            if let Err(e) =
                app_handle.emit("playback-ended", file_path.to_string_lossy().to_string())
            {
                log::error!("❌ Erreur d'envoi de l'event Tauri : {}", e);
            }
        }

        is_stopped.store(false, Ordering::SeqCst);
        is_playing.store(false, Ordering::SeqCst);
        current_position.store(0.0_f64.to_bits(), Ordering::Relaxed);
        current_position_frames.store(0, Ordering::Relaxed);

        Ok(())
    }

    /// Chemin **DSD natif (DoP)** — Windows uniquement.
    ///
    /// Décode les octets DSD → encode en trames DoP → ring buffer i32 → backend
    /// WASAPI DoP qui écrit verbatim au DAC (DSD natif, bit-perfect). Aucun
    /// resampling, aucune conversion PCM, volume logiciel inopérant.
    #[cfg(target_os = "windows")]
    #[allow(clippy::too_many_arguments)]
    fn run_dsd_dop_thread(
        app_handle: AppHandle,
        decoder: Box<dyn DsdContainerReader + Send>,
        file_path: PathBuf,
        lsb_first: bool,
        dsd_rate: u32,
        channel_count: u8,
        carrier_rate: u32,
        device_full_name: String,
        duration: f64,
        is_paused: Arc<AtomicBool>,
        is_playing: Arc<AtomicBool>,
        is_stopped: Arc<AtomicBool>,
        is_stream_alive: Arc<AtomicBool>,
        current_position: Arc<AtomicU64>,
        total_duration: Arc<AtomicU64>,
        seek_position: Arc<AtomicU64>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use crate::core::audio_player::output::dop_engine;

        let channels = channel_count as u16;
        let profile = crate::core::audio_quality::current_profile();

        // ─── Démarrer la piste sur le MOTEUR DoP persistant (gapless) ───
        // Réutilise le moteur vivant si le DAC est déjà locké sur un carrier
        // compatible (→ démarrage instantané, pas de warm-up). Sinon en crée un
        // nouveau (warm-up ~2.5 s pour le lock DSD).
        let (handle, warmup_needed) = match dop_engine::begin_track_on_engine(
            carrier_rate,
            channels,
            device_full_name.clone(),
            is_paused.clone(),
            decoder,
            lsb_first,
            seek_position.clone(),
            total_duration.clone(),
        ) {
            Ok(v) => v,
            Err(e) => {
                log::error!("❌ Moteur DoP indisponible: {e}");
                let _ = app_handle.emit("playback-preparing", false);
                is_playing.store(false, Ordering::SeqCst);
                return Err(format!("DoP engine: {e}").into());
            }
        };

        // ─── Pipeline info (DoP) ───
        crate::core::audio_player::pipeline_info::PlaybackPipelineInfo {
            source_format: crate::core::audio_player::pipeline_info::dsd_label(dsd_rate),
            source_sample_rate: dsd_rate,
            source_bits: 1,
            source_channels: channel_count,
            intermediate_pcm_rate: None,
            dsd_filter_taps: None,
            dsd_decimation: None,
            output_sample_rate: carrier_rate,
            output_channels: channel_count,
            device_name: device_full_name.clone(),
            resampler_active: false,
            quality_profile: format!("{:?}", profile).to_lowercase(),
            backend: "WASAPI DoP".to_string(),
            bit_perfect: true,
        }
        .emit(&app_handle);

        // Compteur figé UNIQUEMENT si un warm-up est nécessaire (nouveau moteur).
        // En réutilisation, le DAC est déjà locké → démarrage instantané.
        if warmup_needed {
            let _ = app_handle.emit("playback-preparing", true);
        } else {
            let _ = app_handle.emit("playback-preparing", false);
        }

        is_playing.store(true, Ordering::SeqCst);
        is_stream_alive.store(true, Ordering::SeqCst);

        // ─── Attente fin de piste : position + dé-figeage compteur ───
        let mut preparing_cleared = !warmup_needed;
        let wait_start = std::time::Instant::now();
        let total_frames_expected = (duration * carrier_rate as f64) as usize;
        let end_threshold = total_frames_expected.saturating_sub(carrier_rate as usize / 4);
        let mut user_stopped = false;
        loop {
            std::thread::sleep(std::time::Duration::from_millis(50));

            // Dé-fige le compteur quand la musique démarre (warm-up fini) ou
            // après un garde-fou de 6 s.
            if !preparing_cleared
                && (handle.audio_started.load(Ordering::Relaxed)
                    || wait_start.elapsed().as_secs() >= 6)
            {
                let _ = app_handle.emit("playback-preparing", false);
                preparing_cleared = true;
            }

            let frames = handle.music_frames_played.load(Ordering::Relaxed);
            let new_pos = frames as f64 / carrier_rate as f64;
            current_position.store(new_pos.to_bits(), Ordering::Relaxed);

            if is_stopped.load(Ordering::Relaxed) {
                user_stopped = true;
                break;
            }
            if handle.decoder_done.load(Ordering::Relaxed) && frames >= end_threshold {
                break;
            }
        }

        // Fin de piste : on GARDE le moteur vivant (silence DSD → DAC locké,
        // gapless). Le décodeur courant est arrêté ; un watchdog fermera le
        // moteur si aucune piste ne suit dans 5 s (vrai arrêt utilisateur).
        // Sur STOP utilisateur : on draine la musique bufferisée pour que
        // l'audio se taise IMMÉDIATEMENT (sinon jusqu'à ~4 s de buffer joué).
        dop_engine::end_current_track_on_engine(user_stopped);

        log::debug!("✅ [DoP] Fin de piste (moteur gardé vivant, user_stopped={user_stopped})");
        is_stream_alive.store(false, Ordering::SeqCst);

        if !preparing_cleared {
            let _ = app_handle.emit("playback-preparing", false);
        }

        // playback-ended seulement sur fin naturelle (→ le frontend enchaîne).
        if !user_stopped {
            if let Err(e) =
                app_handle.emit("playback-ended", file_path.to_string_lossy().to_string())
            {
                log::error!("❌ Erreur d'envoi de l'event Tauri : {}", e);
            }
        }

        is_stopped.store(false, Ordering::SeqCst);
        is_playing.store(false, Ordering::SeqCst);
        current_position.store(0.0_f64.to_bits(), Ordering::Relaxed);

        Ok(())
    }

    /// Thread de lecture DSD natif (DoP) via ALSA hw exclusif — Linux, per-track.
    ///
    /// Décode les octets DSD → encode en trames DoP → écrit verbatim au DAC via
    /// ALSA `hw:` (bit-perfect, aucun resampling ni conversion PCM, volume
    /// logiciel inopérant). Bloquant jusqu'à EOF ou stop utilisateur.
    #[cfg(target_os = "linux")]
    #[allow(clippy::too_many_arguments)]
    fn run_dsd_dop_alsa_thread(
        app_handle: AppHandle,
        decoder: Box<dyn DsdContainerReader + Send>,
        file_path: PathBuf,
        lsb_first: bool,
        dsd_rate: u32,
        channel_count: u8,
        carrier_rate: u32,
        hw_id: String,
        device_full_name: String,
        duration: f64,
        is_paused: Arc<AtomicBool>,
        is_playing: Arc<AtomicBool>,
        is_stopped: Arc<AtomicBool>,
        is_stream_alive: Arc<AtomicBool>,
        current_position: Arc<AtomicU64>,
        total_duration: Arc<AtomicU64>,
        seek_position: Arc<AtomicU64>,
        // Réservation D-Bus de la carte (PipeWire l'a relâchée). Gardée vivante
        // pendant toute la lecture ; droppée au retour → PipeWire reprend la carte.
        reservation: Option<crate::core::audio_player::output::device_reservation::DeviceReservation>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use crate::core::audio_player::output::dop_alsa::{
            run_alsa_dop_playback, AlsaDopControl,
        };

        let _reservation = reservation; // maintient la carte réservée jusqu'à la fin
        let _ = duration; // (position dérivée des frames écrites, pas de la durée)
        let profile = crate::core::audio_quality::current_profile();

        // ─── Pipeline info (DoP ALSA) ───
        crate::core::audio_player::pipeline_info::PlaybackPipelineInfo {
            source_format: crate::core::audio_player::pipeline_info::dsd_label(dsd_rate),
            source_sample_rate: dsd_rate,
            source_bits: 1,
            source_channels: channel_count,
            intermediate_pcm_rate: None,
            dsd_filter_taps: None,
            dsd_decimation: None,
            output_sample_rate: carrier_rate,
            output_channels: channel_count,
            device_name: device_full_name.clone(),
            resampler_active: false,
            quality_profile: format!("{:?}", profile).to_lowercase(),
            backend: "ALSA DoP".to_string(),
            bit_perfect: true,
        }
        .emit(&app_handle);

        // Le DAC peut se muter ~1-2 s le temps d'acquérir le lock DSD.
        let _ = app_handle.emit("playback-preparing", false);
        is_playing.store(true, Ordering::SeqCst);
        is_stream_alive.store(true, Ordering::SeqCst);

        let ctl = AlsaDopControl {
            is_paused,
            is_stopped: is_stopped.clone(),
            current_position: current_position.clone(),
            seek_position,
            total_duration,
        };

        let result = run_alsa_dop_playback(
            &hw_id,
            carrier_rate,
            channel_count as u16,
            decoder,
            lsb_first,
            ctl,
        );

        is_stream_alive.store(false, Ordering::SeqCst);

        let natural_end = match result {
            Ok(natural) => natural,
            Err(e) => {
                log::error!("❌ [DoP ALSA] {e}");
                false
            }
        };

        // playback-ended seulement sur fin naturelle (→ le frontend enchaîne).
        if natural_end {
            if let Err(e) =
                app_handle.emit("playback-ended", file_path.to_string_lossy().to_string())
            {
                log::error!("❌ Erreur d'envoi de l'event Tauri : {}", e);
            }
        }

        is_stopped.store(false, Ordering::SeqCst);
        is_playing.store(false, Ordering::SeqCst);
        current_position.store(0.0_f64.to_bits(), Ordering::Relaxed);

        log::debug!("✅ [DoP ALSA] Fin de piste (natural_end={natural_end})");
        Ok(())
    }

    /// Thread de lecture DSD natif (DoP) via CoreAudio — macOS, per-track.
    ///
    /// Décode les octets DSD → encode en trames DoP → sort verbatim au DAC via
    /// un stream CoreAudio en hog mode (bit-perfect : le rate device est figé
    /// sur le porteur, aucun rééchantillonnage, volume logiciel inopérant).
    /// Bloquant jusqu'à EOF ou stop utilisateur.
    #[cfg(target_os = "macos")]
    #[allow(clippy::too_many_arguments)]
    fn run_dsd_dop_coreaudio_thread(
        app_handle: AppHandle,
        decoder: Box<dyn DsdContainerReader + Send>,
        file_path: PathBuf,
        lsb_first: bool,
        dsd_rate: u32,
        channel_count: u8,
        carrier_rate: u32,
        device: cpal::Device,
        device_id: u32,
        device_full_name: String,
        is_paused: Arc<AtomicBool>,
        is_playing: Arc<AtomicBool>,
        is_stopped: Arc<AtomicBool>,
        is_stream_alive: Arc<AtomicBool>,
        current_position: Arc<AtomicU64>,
        total_duration: Arc<AtomicU64>,
        seek_position: Arc<AtomicU64>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use crate::core::audio_player::output::dop_coreaudio::{
            run_coreaudio_dop_playback, CoreAudioDopControl,
        };

        let profile = crate::core::audio_quality::current_profile();

        // ─── Pipeline info (DoP CoreAudio) ───
        crate::core::audio_player::pipeline_info::PlaybackPipelineInfo {
            source_format: crate::core::audio_player::pipeline_info::dsd_label(dsd_rate),
            source_sample_rate: dsd_rate,
            source_bits: 1,
            source_channels: channel_count,
            intermediate_pcm_rate: None,
            dsd_filter_taps: None,
            dsd_decimation: None,
            output_sample_rate: carrier_rate,
            output_channels: channel_count,
            device_name: device_full_name.clone(),
            resampler_active: false,
            quality_profile: format!("{:?}", profile).to_lowercase(),
            backend: "CoreAudio DoP".to_string(),
            bit_perfect: true,
        }
        .emit(&app_handle);

        // Le DAC peut se muter ~1-2 s le temps d'acquérir le lock DSD.
        let _ = app_handle.emit("playback-preparing", false);
        is_playing.store(true, Ordering::SeqCst);
        is_stream_alive.store(true, Ordering::SeqCst);

        let ctl = CoreAudioDopControl {
            is_paused,
            is_stopped: is_stopped.clone(),
            current_position: current_position.clone(),
            seek_position,
            total_duration,
        };

        let result = run_coreaudio_dop_playback(
            &device,
            device_id,
            carrier_rate,
            channel_count as u16,
            decoder,
            lsb_first,
            ctl,
        );

        is_stream_alive.store(false, Ordering::SeqCst);

        let natural_end = match result {
            Ok(natural) => natural,
            Err(e) => {
                log::error!("❌ [DoP CoreAudio] {e}");
                false
            }
        };

        // playback-ended seulement sur fin naturelle (→ le frontend enchaîne).
        if natural_end {
            if let Err(e) =
                app_handle.emit("playback-ended", file_path.to_string_lossy().to_string())
            {
                log::error!("❌ Erreur d'envoi de l'event Tauri : {}", e);
            }
        }

        is_stopped.store(false, Ordering::SeqCst);
        is_playing.store(false, Ordering::SeqCst);
        current_position.store(0.0_f64.to_bits(), Ordering::Relaxed);

        log::debug!("✅ [DoP CoreAudio] Fin de piste (natural_end={natural_end})");
        Ok(())
    }
}

// ===============
// DECODER THREAD
// ===============
pub fn decode_thread<P>(
    mut format: Box<dyn FormatReader>,
    codec_params: AudioCodecParameters,
    track_id: u32,
    source_sample_rate: u32,
    output_sample_rate: u32,
    channels: usize,
    output_channels: u16,
    mut producer: P,
    full_buffer_data: Arc<RwLock<Vec<f32>>>,
    is_full_buffer_ready: Arc<AtomicBool>,
    current_source: Arc<AtomicU8>,
    current_position_frames: Arc<AtomicUsize>,
    is_stopped: Arc<AtomicBool>,
    seek_position: Arc<AtomicU64>,
    sample_rate_for_seek: u32,
    seek_flush: Arc<AtomicBool>,
    total_duration: Arc<AtomicU64>,
) -> Result<(), String>
where
    P: Producer<Item = f32>
{
    log::debug!("🧵 Decoder thread démarré");

    let mut decoder: Box<dyn AudioDecoder> = symphonia::default::get_codecs()
        .make_audio_decoder(&codec_params, &AudioDecoderOptions::default())
        .map_err(|e| format!("Erreur création decoder: {:?}", e))?;

    // resampler si besoin (None = aucun resampling, output rate == source rate)
    // Sa qualité (chunk size + sub-chunks) suit le profil audio global.
    let mut resampler: Option<Resampler> = Resampler::maybe_new_with_profile(
        source_sample_rate,
        output_sample_rate,
        channels,
        crate::core::audio_quality::current_profile(),
    )?;

    let mut total_frames_decoded: u64 = 0;
    let mut packet_count: u64 = 0;

    loop {
        if is_stopped.load(Ordering::Relaxed) {
            log::debug!("Stop détecté — on quitte le décodage");
            break;
        }

        // ─── SEEK : vérifier si un seek a été demandé ───
        let seek_bits = seek_position.load(Ordering::Relaxed);
        if seek_bits != u64::MAX {
            seek_position.store(u64::MAX, Ordering::Relaxed);

            let total_dur = f64::from_bits(total_duration.load(Ordering::Relaxed));
            let seek_seconds = f64::from_bits(seek_bits).max(0.0).min(total_dur.max(0.0));
            let new_frames = (seek_seconds * output_sample_rate as f64) as usize;

            // CAS 1 : FullBuffer prêt — le fichier est entièrement décodé en RAM.
            // Le décodeur n'a pas accès direct à `full_buffer_cursor`, on passe
            // donc par `seek_flush` que CPAL traitera à son prochain callback
            // (~1 ms) en repositionnant le curseur depuis `current_position_frames`.
            if is_full_buffer_ready.load(Ordering::Relaxed) {
                current_position_frames.store(new_frames, Ordering::Relaxed);
                seek_flush.store(true, Ordering::Relaxed);
                log::debug!("Seek FullBuffer à {:.1}s (instantané)", seek_seconds);
                continue;
            }

            // CAS 2 : LiveDecode — seek dans symphonia + flush ring buffer
            let seek_ts = (seek_seconds * sample_rate_for_seek as f64) as i64;
            match format.seek(
                symphonia::core::formats::SeekMode::Coarse,
                symphonia::core::formats::SeekTo::Timestamp { ts: Timestamp::from(seek_ts), track_id },
            ) {
                Ok(seeked) => {
                    current_position_frames.store(new_frames, Ordering::Relaxed);

                    // Reset resampler (clear accumulator + rubato state)
                    if let Some(ref mut rs) = resampler {
                        rs.reset();
                    }
                    total_frames_decoded = (seek_seconds * source_sample_rate as f64) as u64;

                    // Flush le ring buffer — wait que CPAL ACK le flush.
                    // En conditions normales CPAL répond en 5-20 ms (un cycle
                    // de callback). On poll à 1 ms avec timeout 80 ms : safety
                    // net si CPAL est bloqué (driver freeze), mais sans pénalité
                    // perçue par l'utilisateur.
                    seek_flush.store(true, Ordering::Relaxed);
                    let flush_start = std::time::Instant::now();
                    while seek_flush.load(Ordering::Relaxed) {
                        std::thread::sleep(std::time::Duration::from_millis(1));
                        if flush_start.elapsed().as_millis() > 80 {
                            seek_flush.store(false, Ordering::Relaxed);
                            break;
                        }
                    }

                    // Clear le FullBuffer — il contient des données pré-seek corrompues
                    // Le FullBuffer sera reconstruit proprement depuis la nouvelle position
                    if let Ok(mut fb) = full_buffer_data.write() {
                        fb.clear();
                    }
                    is_full_buffer_ready.store(false, Ordering::Relaxed);
                    current_source.store(0, Ordering::Relaxed);

                    log::debug!("Seek LiveDecode à {:.1}s (actual_ts={:?})", seek_seconds, seeked.actual_ts);
                }
                Err(e) => {
                    log::error!("Seek échoué: {:?}", e);
                }
            }
            continue;
        }

        // On récupère les paquets (0.6 : Ok(None) = fin de média propre,
        // un UnexpectedEof est désormais toujours une vraie erreur)
        let packet: symphonia::core::packet::Packet = match format.next_packet() {
            Ok(Some(p)) => p,
            Ok(None) => {
                log::debug!("Fin du fichier - {} packets décodés", packet_count);
                break;
            }
            Err(e) => {
                log::error!("Erreur lecture packet: {:?}", e);
                break;
            }
        };

        packet_count += 1;

        if packet.track_id != track_id {
            continue;
        }

        let audio_buf: GenericAudioBufferRef<'_> = match decoder.decode(&packet) {
            Ok(buf) => buf,
            Err(Error::DecodeError(msg)) => {
                if msg.contains("invalid main_data offset") || msg.contains("frame length") {
                    continue;
                }
                log::error!("❌ Erreur décodage grave: {}", msg);
                continue;
            }
            Err(e) => {
                log::error!("❌ Erreur critique: {:?}", e);
                break;
            }
        };

        let samples: Vec<f32> = convert_audio_buffer_to_interleaved(&audio_buf, channels);

        if samples.is_empty() {
            continue;
        }

        total_frames_decoded += (samples.len() / channels) as u64;

        // RESAMPLING (déléguée au wrapper Resampler ; None = pas de resampling nécessaire)
        let final_samples: Vec<f32> = match resampler.as_mut() {
            Some(rs) => rs.process_interleaved(&samples),
            None => samples,
        };

        // ADAPTATION CANAUX
        let adapted_samples: Vec<f32> = if channels as u16 != output_channels {
            let mut out: Vec<f32> = vec![0.0f32; (final_samples.len() / channels) * output_channels as usize];
            adapt_channels(&final_samples, channels, &mut out, output_channels.into());
            out
        } else {
            final_samples
        };

        // ==== 1. FULL BUFFER WRITER (Nouveau) ====
        {
            // On récupère le lock en écriture (ça va très vite, CPAL ne fait que lire)
            let mut fb = full_buffer_data.write().unwrap_or_else(|e| e.into_inner());
            fb.extend_from_slice(&adapted_samples);

            if !is_full_buffer_ready.load(Ordering::Relaxed) {
                const SAFETY_MARGIN: usize = 2000;
                
                // On calcule le nombre de samples déjà joués grâce à l'AtomicUsize
                let played_frames: usize = current_position_frames.load(Ordering::Relaxed);
                let played_samples: usize = played_frames * output_channels as usize;

                if fb.len() > played_samples + SAFETY_MARGIN {
                    is_full_buffer_ready.store(true, Ordering::Relaxed);
                    
                    // On bascule sur le FullBuffer (1 = FullBuffer)
                    current_source.store(1, Ordering::Relaxed);
                    
                    log::debug!(
                        "🚀 FullBuffer prêt ! ({} samples) - Basculement effectué",
                        fb.len()
                    );
                }
            }
        }

        // ==== 2. RING BUFFER WRITER (Nouveau) ====
        let current_source_mode: u8 = current_source.load(Ordering::Relaxed);

        if current_source_mode == 0 { // 0 = LiveDecode
            let mut offset: usize = 0;

            while offset < adapted_samples.len() {
                if is_stopped.load(Ordering::Relaxed) {
                    break;
                }

                // ⚡ Réactivité seek : si l'utilisateur a demandé un seek pendant
                // qu'on est bloqué à pousser dans un ring buffer plein, on abandonne
                // le push en cours pour traiter le seek au plus vite. Sans ça, le
                // décodeur peut rester bloqué 20-100 ms à pousser un packet, ce qui
                // donne une latence ressentie à chaque seek pendant que le buffer
                // est rempli (fréquent juste après le démarrage).
                if seek_position.load(Ordering::Relaxed) != u64::MAX {
                    break;
                }

                // push_slice écrit le maximum possible et renvoie combien il en a écrit
                let written: usize = producer.push_slice(&adapted_samples[offset..]);
                offset += written;

                // S'il n'a rien pu écrire, c'est que le buffer est plein.
                // On attend 1ms pour laisser CPAL vider un peu le buffer.
                if written == 0 {
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
            }
        }
    }

    log::debug!(
        "🧵 Decoder thread terminé - {} frames décodées ({:.2}s)",
        total_frames_decoded,
        total_frames_decoded as f64 / source_sample_rate as f64
    );

    Ok(())
}
