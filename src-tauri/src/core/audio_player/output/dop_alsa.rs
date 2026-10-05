//! Sortie DoP (DSD over PCM) bit-perfect via ALSA `hw:` — Linux uniquement.
//!
//! # Pourquoi ALSA direct et pas CPAL/PipeWire
//! Le DoP n'a pas de demi-mesure : soit chaque octet arrive **intact** au DAC
//! (qui reconnaît les marqueurs `0x05/0xFA` et rejoue du DSD natif), soit un
//! seul bit altéré détruit les marqueurs → bruit blanc très fort. PipeWire/Pulse
//! rééchantillonnent et appliquent le volume → destructeurs. On ouvre donc le
//! périphérique ALSA `hw:` en direct (accès exclusif, bypass du serveur son),
//! équivalent Linux du WASAPI exclusive de Windows.
//!
//! # Format
//! `S32_LE`, `carrier_rate` (= dsd_rate / 16), 2 canaux, **sans resampling**.
//! L'encodeur (`dop_encoder`) produit des `i32` `[0][hi][lo][0]` ; le marqueur
//! (`0x05/0xFA`, alterné à chaque trame, compteur continu) est posé ici sur
//! CHAQUE trame — musique comme silence — sinon le DAC perd le lock DSD.
//!
//! Stream tenu par le moteur DoP persistant (`dop_engine`) : pistes enchaînées sans relock.

#![cfg(target_os = "linux")]

use std::sync::atomic::Ordering;

use alsa::pcm::{Access, Format, HwParams, PCM};
use alsa::{Direction, ValueOr};
use ringbuf::traits::{Consumer, Observer};
use ringbuf::HeapCons;

use super::dop_engine::{DopRenderCtl, PRECHAUFFAGE_S};
use crate::core::audio_decoder::dsd::dop_encoder::{dop_silence_payload, stamp_marker};

/// Un périphérique de sortie ALSA hardware candidat au DoP.
#[derive(Debug, Clone)]
pub struct AlsaDopDevice {
    /// Identifiant ALSA ouvrable, ex. `hw:2,0`.
    pub hw_id: String,
    /// Nom lisible (ex. "Fosi Audio K7"), pour le matching avec le nom UI.
    pub display: String,
}

/// Énumère les cartes ALSA matérielles (device 0 de chaque carte).
///
/// On cible `hw:{index},0` (non ambigu, contrairement à `hw:CARD=<id>`). La
/// plupart des DAC exposent leur sortie principale sur le device 0.
pub fn list_dop_devices() -> Vec<AlsaDopDevice> {
    let mut out = Vec::new();
    for card in alsa::card::Iter::new().flatten() {
        let idx = card.get_index();
        let display = card.get_name().unwrap_or_else(|_| format!("Carte {idx}"));
        out.push(AlsaDopDevice {
            hw_id: format!("hw:{idx},0"),
            display,
        });
    }
    out
}

/// Mappe le nom d'affichage sélectionné dans l'UI (ex. "Fosi Audio K7 (…)")
/// vers l'identifiant ALSA `hw:` correspondant, par sous-chaîne insensible à
/// la casse dans les deux sens. `None` si aucune carte hardware ne correspond.
pub fn resolve_hw_id(selected_display: &str) -> Option<String> {
    let sel = selected_display.to_lowercase();
    for dev in list_dop_devices() {
        let name = dev.display.to_lowercase();
        if name.is_empty() {
            continue;
        }
        if sel.contains(&name) || name.contains(&sel) {
            return Some(dev.hw_id);
        }
    }
    None
}

/// Ouvre le PCM `hw:` en réessayant sur EBUSY (errno 16). Après une réservation
/// D-Bus, PipeWire ferme le device de façon **asynchrone** : le `hw:` peut rester
/// occupé ~centaines de ms (0,4 s mesuré pendant une lecture), parfois plus de
/// 3 s quand WirePlumber est lent à démonter la carte. On réessaie jusqu'à ~6 s.
fn open_hw_pcm(hw_id: &str) -> Result<PCM, String> {
    let mut last = String::new();
    for _ in 0..60 {
        match PCM::new(hw_id, Direction::Playback, false) {
            Ok(pcm) => return Ok(pcm),
            Err(e) => {
                let busy = e.errno() == 16 || e.errno() == -16; // EBUSY
                last = e.to_string();
                if busy {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    continue;
                }
                return Err(last);
            }
        }
    }
    Err(format!("device toujours occupé après attente : {last}"))
}

/// Configure des `HwParams` DoP standard sur un PCM (S32_LE / carrier / 2ch,
/// resampling désactivé). Factorisé entre le probe et le render.
fn configure_dop_params(pcm: &PCM, carrier_rate: u32, channels: u16) -> Result<(), String> {
    let hwp = HwParams::any(pcm).map_err(|e| format!("hw_params any: {e}"))?;
    hwp.set_access(Access::RWInterleaved)
        .map_err(|e| format!("set_access: {e}"))?;
    hwp.set_format(Format::S32LE)
        .map_err(|e| format!("set_format S32_LE: {e}"))?;
    hwp.set_channels(channels as u32)
        .map_err(|e| format!("set_channels: {e}"))?;
    // CRUCIAL : pas de rééchantillonnage, sinon le DoP est détruit.
    hwp.set_rate_resample(false)
        .map_err(|e| format!("set_rate_resample(false): {e}"))?;
    hwp.set_rate(carrier_rate, ValueOr::Nearest)
        .map_err(|e| format!("set_rate {carrier_rate}: {e}"))?;
    // Latence raisonnable : ~80 ms de buffer, périodes ~20 ms.
    let _ = hwp.set_buffer_time_near(80_000, ValueOr::Nearest);
    let _ = hwp.set_period_time_near(20_000, ValueOr::Nearest);
    pcm.hw_params(&hwp).map_err(|e| format!("apply hw_params: {e}"))?;

    // Vérifier que le rate obtenu est EXACTEMENT le carrier (sinon resampling
    // implicite ou format non supporté → DoP invalide).
    let got = pcm
        .hw_params_current()
        .and_then(|h| h.get_rate())
        .map_err(|e| format!("get_rate: {e}"))?;
    if got != carrier_rate {
        return Err(format!("rate obtenu {got} ≠ carrier {carrier_rate}"));
    }
    Ok(())
}

/// Le DAC accepte-t-il le format DoP au carrier donné, en accès exclusif ?
///
/// Ouvre le `hw:` (échoue si occupé par PipeWire/Pulse — dans ce cas pas de
/// bit-perfect possible → `false`), tente la négociation, referme.
pub fn dop_format_supported(hw_id: &str, carrier_rate: u32, channels: u16) -> bool {
    match open_hw_pcm(hw_id) {
        Ok(pcm) => match configure_dop_params(&pcm, carrier_rate, channels) {
            Ok(()) => true,
            Err(e) => {
                log::warn!("🎚️  DoP ALSA format refusé sur {hw_id} @ {carrier_rate} Hz : {e}");
                false
            }
        },
        Err(e) => {
            log::warn!("🎚️  Ouverture exclusive {hw_id} impossible : {e}");
            false
        }
    }
}

/// Trames écrites par tour : ~11,6 ms au porteur DSD64 (176,4 kHz). `writei`
/// bloque tant que le tampon matériel est plein : c'est lui qui cadence la boucle.
const BLOC: usize = 2048;

/// Render persistant du moteur DoP sur le `hw:` jusqu'au `render_stop` ; silence DoP sinon.
/// Marqueur posé sur chaque trame avec un compteur continu : `0x05/0xFA` ne se rompt jamais.
pub fn run_alsa_dop_render(
    hw_id: &str,
    carrier_rate: u32,
    channels: u16,
    mut consumer: HeapCons<i32>,
    ctl: DopRenderCtl,
) -> Result<(), String> {
    let ch = channels.max(1) as usize;

    let pcm = open_hw_pcm(hw_id).map_err(|e| format!("ouverture {hw_id}: {e}"))?;
    configure_dop_params(&pcm, carrier_rate, channels)?;
    pcm.prepare().map_err(|e| format!("prepare: {e}"))?;

    let silence = dop_silence_payload();
    let mut marker_b = false;
    let mut payload = vec![0i32; BLOC * ch];
    let mut buf: Vec<i32> = Vec::with_capacity(BLOC * ch);

    // Écrit `buf` (interleavé, déjà marqué) en gérant les XRUN.
    let write_all = |pcm: &PCM, buf: &[i32]| -> Result<(), String> {
        let io = pcm.io_i32().map_err(|e| format!("io_i32: {e}"))?;
        let mut off = 0usize;
        while off < buf.len() {
            match io.writei(&buf[off..]) {
                Ok(frames) => off += frames * ch,
                Err(e) => {
                    if pcm.recover(e.errno(), true).is_err() {
                        return Err(format!("writei: {e}"));
                    }
                }
            }
        }
        Ok(())
    };

    // Compose un bloc : `musique` trames du payload, le reste en silence DoP.
    let mut composer = |payload: &[i32], musique: usize, buf: &mut Vec<i32>| {
        buf.clear();
        for f in 0..BLOC {
            let marker = marker_b;
            marker_b = !marker_b;
            if f < musique {
                for c in 0..ch {
                    buf.push(stamp_marker(payload[f * ch + c], marker));
                }
            } else {
                let s = stamp_marker(silence, marker);
                buf.extend(std::iter::repeat_n(s, ch));
            }
        }
    };

    // Pré-remplissage : le DAC verrouille sur du silence DoP valide dès la 1re trame.
    for _ in 0..2 {
        composer(&payload[..], 0, &mut buf);
        write_all(&pcm, &buf)?;
    }
    ctl.started_ok.store(true, Ordering::Relaxed);
    log::info!("🎚️  Stream ALSA DoP ouvert : {hw_id} @ {carrier_rate} Hz / {channels} ch (S32_LE, moteur persistant)");

    let warmup_frames = (carrier_rate as f64 * PRECHAUFFAGE_S) as usize;
    let mut warmed = 0usize;

    loop {
        if ctl.render_stop.load(Ordering::Relaxed) {
            // Un peu de silence avant de couper : le DAC quitte le DSD proprement.
            for _ in 0..2 {
                composer(&payload[..], 0, &mut buf);
                let _ = write_all(&pcm, &buf);
            }
            let _ = pcm.drain();
            break;
        }

        let mut musique = 0usize;
        // Vidage et saut d'abord, warm-up compris (sinon le début de la piste
        // déjà bufferisé serait jeté à la fin du warm-up).
        if ctl.drain_request.load(Ordering::Acquire) {
            consumer.clear();
            ctl.drain_request.store(false, Ordering::Release);
        } else if ctl.seek_flush.load(Ordering::Acquire) {
            consumer.clear();
            ctl.seek_flush.store(false, Ordering::Release);
        } else if warmed < warmup_frames {
            warmed += BLOC;
            if warmed >= warmup_frames {
                ctl.audio_started.store(true, Ordering::Relaxed);
                log::debug!("🎚️  [DoP ALSA] Warm-up terminé — lecture musique");
            }
        } else if !ctl.is_paused.load(Ordering::Relaxed) {
            // Trames entières seulement : l'appariement canal/échantillon tient.
            let dispo = (consumer.occupied_len() / ch).min(BLOC);
            musique = consumer.pop_slice(&mut payload[..dispo * ch]) / ch;
        }

        composer(&payload[..], musique, &mut buf);
        write_all(&pcm, &buf)?;
        if musique > 0 {
            ctl.music_frames_played.fetch_add(musique, Ordering::Relaxed);
        }
    }

    log::info!("⏹  ALSA DoP : stream fermé ({hw_id})");
    Ok(())
}

