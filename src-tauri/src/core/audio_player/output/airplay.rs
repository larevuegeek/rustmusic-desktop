//! Sortie AirPlay : le rendu de la piste part vers l'appareil au lieu de la carte son.

use ringbuf::traits::Consumer;

use super::traits::{AudioOutput, AudioOutputError};
use super::types::{AudioBackend, PlaybackAtomics, SymphoniaSharedState};
use crate::core::network_player::airplay::device::AirplayDevice;
use crate::core::network_player::airplay::sender::{self, Track, CHANNELS, SAMPLE_RATE};

pub struct AirplayOutput {
    device: AirplayDevice,
    /// Branchée au démarrage.
    pending: Option<Track>,
    track: Option<u64>,
}

impl AirplayOutput {
    pub fn new<C>(device: AirplayDevice, consumer: C, atomics: PlaybackAtomics, shared: SymphoniaSharedState) -> Self
    where
        C: Consumer<Item = f32> + Send + 'static,
    {
        let render = super::cpal_symphonia::render_fn(consumer, atomics.clone(), shared.clone(), super::samples::MARGE_PARTAGEE, crate::core::network_player::airplay::sender::SAMPLE_RATE);
        Self { device, pending: Some(Track { render: Box::new(render), atomics, shared }), track: None }
    }
}

impl AudioOutput for AirplayOutput {
    fn start(&mut self) -> Result<(), AudioOutputError> {
        if let Some(track) = self.pending.take() {
            self.track = Some(sender::attach(&self.device, track).map_err(AudioOutputError::StartFailed)?);
        }
        Ok(())
    }

    fn output_sample_rate(&self) -> u32 {
        SAMPLE_RATE
    }

    fn output_channels(&self) -> u16 {
        CHANNELS
    }

    fn device_name(&self) -> &str {
        &self.device.name
    }

    fn backend(&self) -> AudioBackend {
        AudioBackend::AirPlay
    }

    fn bits_exacts(&self) -> u32 {
        16
    }
}

impl Drop for AirplayOutput {
    fn drop(&mut self) {
        if let Some(id) = self.track.take() {
            sender::detach(id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::{Ipv4Addr, TcpListener, UdpSocket};
    use std::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex, RwLock};
    use std::time::{Duration, Instant};

    use crate::core::network_player::airplay::device::Encryption;
    use ringbuf::traits::Split;

    /// Faux appareil : répond au RTSP et note les méthodes reçues.
    fn fake_device(audio_port: u16, control_port: u16) -> (u16, Arc<Mutex<Vec<String>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let methods = Arc::new(Mutex::new(Vec::new()));
        let seen = methods.clone();
        std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut writer = stream.try_clone().unwrap();
            let mut reader = BufReader::new(stream);
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap_or(0) == 0 {
                    break;
                }
                let method = line.split_whitespace().next().unwrap_or_default().to_string();
                let (mut cseq, mut len, mut kind) = (String::new(), 0usize, String::new());
                loop {
                    let mut h = String::new();
                    reader.read_line(&mut h).unwrap();
                    if h.trim().is_empty() {
                        break;
                    }
                    if let Some(v) = h.strip_prefix("CSeq:") {
                        cseq = v.trim().to_string();
                    }
                    if let Some(v) = h.strip_prefix("Content-Type:") {
                        kind = v.trim().to_string();
                    }
                    if let Some(v) = h.strip_prefix("Content-Length:") {
                        len = v.trim().parse().unwrap();
                    }
                }
                let mut body = vec![0; len];
                reader.read_exact(&mut body).unwrap();
                let extra = match method.as_str() {
                    "ANNOUNCE" => {
                        assert!(String::from_utf8_lossy(&body).contains("a=rtpmap:96 AppleLossless"));
                        String::new()
                    }
                    "SETUP" => format!(
                        "Session: DEADBEEF;timeout=60\r\nTransport: RTP/AVP/UDP;unicast;mode=record;server_port={audio_port};control_port={control_port};timing_port=0\r\n"
                    ),
                    "RECORD" => "Audio-Latency: 11025\r\n".into(),
                    _ => String::new(),
                };
                let label = if method == "SET_PARAMETER" { format!("{method} {kind}") } else { method.clone() };
                seen.lock().unwrap().push(label);
                write!(writer, "RTSP/1.0 200 OK\r\nCSeq: {cseq}\r\n{extra}\r\n").unwrap();
                if method == "TEARDOWN" {
                    break;
                }
            }
        });
        (port, methods)
    }

    #[test]
    fn streams_paces_pauses_and_closes() {
        let audio = UdpSocket::bind("127.0.0.1:0").unwrap();
        let control = UdpSocket::bind("127.0.0.1:0").unwrap();
        audio.set_read_timeout(Some(Duration::from_millis(50))).unwrap();
        control.set_read_timeout(Some(Duration::from_millis(50))).unwrap();
        let (port, methods) = fake_device(audio.local_addr().unwrap().port(), control.local_addr().unwrap().port());
        let device = AirplayDevice {
            id: "airplay:banc".into(),
            name: "Banc".into(),
            model: String::new(),
            ip: Ipv4Addr::LOCALHOST,
            port,
            encryption: Encryption::None,
            shows: crate::core::network_player::airplay::device::Shows { text: true, artwork: false, progress: true },
        };

        // Une seconde de son en RAM, lue par le rendu commun.
        let samples: Vec<f32> = (0..SAMPLE_RATE as usize * 2).map(|i| ((i % 200) as f32 / 200.0) - 0.5).collect();
        let atomics = PlaybackAtomics {
            is_paused: Arc::new(AtomicBool::new(false)),
            is_stopped: Arc::new(AtomicBool::new(false)),
            volume: Arc::new(AtomicU8::new(100)),
            current_position_frames: Arc::new(AtomicUsize::new(0)),
        };
        let shared = SymphoniaSharedState {
            full_buffer_data: Arc::new(RwLock::new(samples)),
            full_buffer_cursor: Arc::new(AtomicUsize::new(0)),
            is_full_buffer_ready: Arc::new(AtomicBool::new(true)),
            current_source: Arc::new(AtomicU8::new(1)),
            seek_flush: Arc::new(AtomicBool::new(false)),
            pending_seek_frames: Arc::new(AtomicUsize::new(usize::MAX)),
            output_channels: CHANNELS,
            promotion_armed: Arc::new(AtomicBool::new(false)),
        };
        let (_, consumer) = ringbuf::HeapRb::<f32>::new(16).split();
        let mut output = AirplayOutput::new(device, consumer, atomics.clone(), shared.clone());
        let started = Instant::now();
        output.start().unwrap();
        crate::core::network_player::airplay::sender::test_now_playing("Banc d'essai", 1.0);
        assert_eq!(*methods.lock().unwrap(), ["OPTIONS", "ANNOUNCE", "SETUP", "RECORD"]);

        // Synchro d'ouverture, drapeau « premier » levé.
        let mut buf = [0u8; 2048];
        let (n, _) = control.recv_from(&mut buf).unwrap();
        assert_eq!((n, buf[0], buf[1]), (20, 0x90, 0xD4));

        // Paquets audio pendant ~0,5 s : cadencés sur l'horloge, pas envoyés d'un bloc.
        let mut packets = Vec::new();
        while started.elapsed() < Duration::from_millis(500) {
            if let Ok(n) = audio.recv(&mut buf) {
                packets.push(buf[..n].to_vec());
            }
        }
        let elapsed = started.elapsed().as_secs_f64();
        let ceiling = ((elapsed * SAMPLE_RATE as f64 + 4_410.0) / 352.0) as usize + 2;
        assert!(packets.len() >= 40 && packets.len() <= ceiling, "{} paquets (plafond {ceiling})", packets.len());
        assert_eq!(packets[0][1], 0xE0, "marqueur sur le premier paquet");
        assert_eq!(packets[1][1], 0x60);
        let seq = |p: &Vec<u8>| u16::from_be_bytes([p[2], p[3]]);
        let ts = |p: &Vec<u8>| u32::from_be_bytes([p[4], p[5], p[6], p[7]]);
        assert_eq!(seq(&packets[1]), seq(&packets[0]).wrapping_add(1));
        assert_eq!(ts(&packets[1]), ts(&packets[0]).wrapping_add(352));
        assert_eq!(packets[0].len(), 12 + 1416);
        assert!(packets[3][12 + 7..].iter().any(|b| *b != 0), "du son, pas du silence");

        // Rien n'est encore entendu (latence ~2 s) : la position affichée reste à zéro.
        assert!(crate::core::network_player::airplay::sender::lag_secs() > 0.3);

        // Titre et progression partent quand le début s'entend (~2 s), pas avant.
        assert!(!methods.lock().unwrap().iter().any(|m| m.starts_with("SET_PARAMETER")));
        while started.elapsed() < Duration::from_millis(2_600) {
            let _ = audio.recv(&mut buf);
        }
        let sent = methods.lock().unwrap().clone();
        assert!(sent.contains(&"SET_PARAMETER application/x-dmap-tagged".to_string()), "{sent:?}");
        assert!(sent.contains(&"SET_PARAMETER text/parameters".to_string()), "{sent:?}");

        // Pause : l'appareil vide son tampon, la lecture recule à ce qui a été entendu (~0,6 s).
        atomics.is_paused.store(true, Ordering::SeqCst);
        std::thread::sleep(Duration::from_millis(150));
        assert_eq!(methods.lock().unwrap().last().map(String::as_str), Some("FLUSH"));
        assert!(shared.seek_flush.load(Ordering::SeqCst));
        let heard = shared.pending_seek_frames.load(Ordering::SeqCst) as f64 / SAMPLE_RATE as f64;
        assert!((0.3..0.95).contains(&heard), "entendu {heard:.2} s");

        // Arrêt puis fermeture : la session est rendue.
        atomics.is_stopped.store(true, Ordering::SeqCst);
        drop(output);
        crate::core::network_player::airplay::sender::close();
        assert_eq!(methods.lock().unwrap().last().map(String::as_str), Some("TEARDOWN"));
    }
}
