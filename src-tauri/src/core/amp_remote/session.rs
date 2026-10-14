//! Session de télécommande : connexion gardée ouverte, état tenu à jour, commandes envoyées.
//!
//! L'ampli annonce lui-même chaque changement (sa télécommande, sa façade) : l'interface
//! reçoit l'état à jour par l'événement `amp-remote-state`, sans interroger.

use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use super::model::{Amp, Brand, Codec, Command, Status};
use super::onkyo;

const RETRY: Duration = Duration::from_secs(5);
const KEEPALIVE: Duration = Duration::from_secs(30);
/// Au-delà, un flux sans message reconnaissable est jeté.
const MAX_BUFFER: usize = 1 << 20;

/// Ce que l'interface affiche : l'ampli, ses capacités et son état.
#[derive(Debug, Clone, Serialize)]
pub struct View {
    pub amp: Amp,
    #[serde(flatten)]
    pub status: Status,
}

struct Remote {
    amp: Amp,
    tx: UnboundedSender<Command>,
    status: Arc<Mutex<Status>>,
    task: tauri::async_runtime::JoinHandle<()>,
}

static REMOTE: Mutex<Option<Remote>> = Mutex::new(None);
static APP: OnceLock<AppHandle> = OnceLock::new();

fn remote() -> MutexGuard<'static, Option<Remote>> {
    REMOTE.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn set_app(app: AppHandle) {
    let _ = APP.set(app);
}

/// Pilote `amp` (`None` : plus aucun) ; la connexion précédente est fermée.
pub fn select(amp: Option<Amp>) {
    let mut guard = remote();
    if let Some(old) = guard.take() {
        old.task.abort();
    }
    let Some(amp) = amp else {
        emit(None);
        return;
    };
    let (codec, caps): (Box<dyn Codec>, _) = match amp.brand {
        Brand::Onkyo => (Box::new(onkyo::OnkyoCodec::default()), onkyo::default_caps()),
        // Les autres marques arrivent avec leur pilote.
        Brand::Denon | Brand::Yamaha => {
            log::warn!("🎛️  Télécommande : {:?} pas encore pris en charge", amp.brand);
            return;
        }
    };
    let status = Arc::new(Mutex::new(Status { caps, state: Default::default(), cover: None }));
    let (tx, rx) = mpsc::unbounded_channel();
    let task = tauri::async_runtime::spawn(run(amp.clone(), codec, status.clone(), rx));
    log::info!("🎛️  Télécommande : {} ({})", amp.name, amp.ip);
    *guard = Some(Remote { amp, tx, status, task });
}

pub fn view() -> Option<View> {
    remote().as_ref().map(|r| View { amp: r.amp.clone(), status: lock(&r.status).clone() })
}

pub fn send(cmd: Command) -> Result<(), String> {
    let guard = remote();
    let r = guard.as_ref().ok_or("aucun ampli piloté")?;
    if !lock(&r.status).state.connected {
        return Err(format!("{} injoignable", r.amp.name));
    }
    r.tx.send(cmd).map_err(|_| "télécommande arrêtée".to_string())
}

fn lock(status: &Mutex<Status>) -> MutexGuard<'_, Status> {
    status.lock().unwrap_or_else(|e| e.into_inner())
}

fn emit(view: Option<View>) {
    if let Some(app) = APP.get() {
        let _ = app.emit("amp-remote-state", view);
    }
}

/// Pochette de l'écran réseau, en adresse `data:` prête à afficher.
pub fn cover() -> Option<String> {
    let guard = remote();
    let r = guard.as_ref()?;
    let st = lock(&r.status);
    st.cover.as_ref().map(data_url)
}

fn data_url((mime, bytes): &(String, Vec<u8>)) -> String {
    use base64::Engine;
    format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes))
}

fn emit_cover(status: &Mutex<Status>) {
    let url = lock(status).cover.as_ref().map(data_url);
    if let Some(app) = APP.get() {
        let _ = app.emit("amp-remote-cover", url);
    }
}

fn publish(amp: &Amp, status: &Mutex<Status>) {
    emit(Some(View { amp: amp.clone(), status: lock(status).clone() }));
}

/// Connexion, lecture des annonces, envoi des commandes ; se reconnecte tant qu'elle est choisie.
async fn run(amp: Amp, mut codec: Box<dyn Codec>, status: Arc<Mutex<Status>>, mut rx: UnboundedReceiver<Command>) {
    loop {
        let connect = tokio::time::timeout(Duration::from_secs(4), TcpStream::connect((amp.ip.as_str(), amp.port))).await;
        if let Ok(Ok(stream)) = connect {
            lock(&status).state.connected = true;
            publish(&amp, &status);
            if let Err(e) = serve(stream, codec.as_mut(), &amp, &status, &mut rx).await {
                log::info!("🎛️  {} : connexion perdue ({e})", amp.name);
            }
            lock(&status).state.connected = false;
            publish(&amp, &status);
        }
        // Commandes données pendant la coupure : obsolètes à la reconnexion.
        tokio::time::sleep(RETRY).await;
        while rx.try_recv().is_ok() {}
    }
}

async fn serve(
    stream: TcpStream,
    codec: &mut dyn Codec,
    amp: &Amp,
    status: &Mutex<Status>,
    rx: &mut UnboundedReceiver<Command>,
) -> Result<(), String> {
    let (mut rd, mut wr) = stream.into_split();
    for frame in codec.hello() {
        wr.write_all(&frame).await.map_err(|e| e.to_string())?;
    }
    let mut buf = Vec::new();
    let mut chunk = vec![0u8; 16 * 1024];
    let mut keepalive = tokio::time::interval(KEEPALIVE);
    keepalive.tick().await;
    loop {
        tokio::select! {
            read = rd.read(&mut chunk) => {
                let n = read.map_err(|e| e.to_string())?;
                if n == 0 {
                    return Err("fermée par l'ampli".into());
                }
                buf.extend_from_slice(&chunk[..n]);
                let (changed, cover) = {
                    let mut st = lock(status);
                    let before = st.state.clone();
                    let used = codec.feed(&buf, &mut st);
                    buf.drain(..used);
                    (st.state != before, st.state.net.cover_rev != before.net.cover_rev)
                };
                if cover {
                    emit_cover(status);
                }
                if buf.len() > MAX_BUFFER {
                    buf.clear();
                }
                if changed {
                    publish(amp, status);
                }
            }
            cmd = rx.recv() => {
                let Some(cmd) = cmd else { return Ok(()) };
                let frames = codec.encode(&cmd, &lock(status));
                for frame in frames {
                    wr.write_all(&frame).await.map_err(|e| e.to_string())?;
                }
            }
            _ = keepalive.tick() => {
                wr.write_all(&codec.keepalive()).await.map_err(|e| e.to_string())?;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::model::Zone;

    #[tokio::test]
    async fn talks_to_a_fake_onkyo() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let device = tokio::spawn(async move {
            let (mut s, _) = listener.accept().await.unwrap();
            let mut seen = Vec::new();
            let mut buf = vec![0u8; 4096];
            // Questions d'ouverture, puis l'état annoncé par l'ampli.
            while !String::from_utf8_lossy(&seen).contains("SLZQSTN") {
                let n = s.read(&mut buf).await.unwrap();
                seen.extend_from_slice(&buf[..n]);
            }
            s.write_all(&[onkyo::frame("PWR01"), onkyo::frame("MVL14")].concat()).await.unwrap();
            // La commande de l'interface arrive, puis l'ampli coupe.
            while !String::from_utf8_lossy(&seen).contains("MVLUP") {
                let n = s.read(&mut buf).await.unwrap();
                seen.extend_from_slice(&buf[..n]);
            }
        });

        let stream = TcpStream::connect(addr).await.unwrap();
        let status = Arc::new(Mutex::new(Status { caps: onkyo::default_caps(), state: Default::default(), cover: None }));
        let (tx, mut rx) = mpsc::unbounded_channel();
        let amp = Amp { id: "onkyo:test".into(), name: "Banc".into(), brand: Brand::Onkyo, model: "Banc".into(), ip: addr.ip().to_string(), port: addr.port() };
        let st = status.clone();
        let session = tokio::spawn(async move { serve(stream, &mut onkyo::OnkyoCodec::default(), &amp, &st, &mut rx).await });

        let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
        while lock(&status).state.main.volume != Some(20.0) {
            assert!(tokio::time::Instant::now() < deadline, "état jamais reçu");
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert!(lock(&status).state.main.power);
        tx.send(Command::VolumeStep { zone: Zone::Main, up: true }).unwrap();
        device.await.unwrap();
        let ended = tokio::time::timeout(Duration::from_secs(3), session).await.unwrap().unwrap();
        assert_eq!(ended, Err("fermée par l'ampli".to_string()));
    }
}
