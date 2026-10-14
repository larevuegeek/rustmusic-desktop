//! Client RTSP d'AirPlay 1 : requêtes texte sur une connexion TCP gardée ouverte.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

/// Réponse RTSP.
#[derive(Debug, Default)]
pub struct Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Response {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)).map(|(_, v)| v.as_str())
    }
}

pub struct Rtsp {
    reader: BufReader<TcpStream>,
    stream: TcpStream,
    /// `rtsp://<notre ip>/<id>` : adresse de la session côté appareil.
    pub url: String,
    cseq: u32,
    session: Option<String>,
    /// Identifiants de l'émetteur, envoyés à chaque requête.
    client: String,
    dacp_id: String,
    active_remote: String,
}

impl Rtsp {
    pub fn connect(addr: SocketAddr) -> Result<Self, String> {
        let stream = TcpStream::connect_timeout(&addr, Duration::from_secs(4)).map_err(|e| format!("connexion à {addr} : {e}"))?;
        stream.set_read_timeout(Some(Duration::from_secs(5))).ok();
        stream.set_write_timeout(Some(Duration::from_secs(5))).ok();
        stream.set_nodelay(true).ok();
        let local = stream.local_addr().map_err(|e| e.to_string())?;
        let random = random_u64();
        let reader = BufReader::new(stream.try_clone().map_err(|e| e.to_string())?);
        Ok(Self {
            reader,
            stream,
            url: format!("rtsp://{}/{}", local.ip(), random as u32),
            cseq: 0,
            session: None,
            client: format!("{random:016X}"),
            dacp_id: format!("{:016X}", random.rotate_left(17)),
            active_remote: format!("{}", (random >> 32) as u32),
        })
    }

    pub fn local_ip(&self) -> Option<std::net::IpAddr> {
        self.stream.local_addr().ok().map(|a| a.ip())
    }

    pub fn set_session(&mut self, session: Option<String>) {
        self.session = session;
    }

    /// Envoie une requête et attend la réponse ; un statut autre que 200 est une erreur.
    pub fn request(
        &mut self,
        method: &str,
        uri: Option<&str>,
        headers: &[(&str, String)],
        body: Option<(&str, &[u8])>,
    ) -> Result<Response, String> {
        self.cseq += 1;
        let mut head = format!("{method} {} RTSP/1.0\r\nCSeq: {}\r\n", uri.unwrap_or(&self.url), self.cseq);
        head.push_str("User-Agent: iTunes/7.6.2 (Windows; N;)\r\n");
        head.push_str(&format!("Client-Instance: {}\r\nDACP-ID: {}\r\nActive-Remote: {}\r\n", self.client, self.dacp_id, self.active_remote));
        if let Some(s) = &self.session {
            head.push_str(&format!("Session: {s}\r\n"));
        }
        for (k, v) in headers {
            head.push_str(&format!("{k}: {v}\r\n"));
        }
        if let Some((content_type, data)) = body {
            head.push_str(&format!("Content-Type: {content_type}\r\nContent-Length: {}\r\n", data.len()));
        }
        head.push_str("\r\n");
        let mut packet = head.into_bytes();
        if let Some((_, data)) = body {
            packet.extend_from_slice(data);
        }
        self.stream.write_all(&packet).map_err(|e| format!("{method} : {e}"))?;

        let response = self.read_response().map_err(|e| format!("{method} : {e}"))?;
        if response.status != 200 {
            return Err(format!("{method} refusé ({})", response.status));
        }
        Ok(response)
    }

    fn read_response(&mut self) -> std::io::Result<Response> {
        let mut line = String::new();
        self.reader.read_line(&mut line)?;
        let status = parse_status(&line).ok_or_else(|| std::io::Error::other(format!("réponse illisible : {}", line.trim())))?;
        let mut headers = Vec::new();
        loop {
            line.clear();
            if self.reader.read_line(&mut line)? == 0 {
                break;
            }
            let l = line.trim_end();
            if l.is_empty() {
                break;
            }
            if let Some((k, v)) = l.split_once(':') {
                headers.push((k.trim().to_string(), v.trim().to_string()));
            }
        }
        let mut response = Response { status, headers, body: Vec::new() };
        let len = response.header("Content-Length").and_then(|v| v.parse::<usize>().ok()).unwrap_or(0);
        response.body.resize(len.min(1 << 20), 0);
        self.reader.read_exact(&mut response.body)?;
        Ok(response)
    }
}

// ─── Fil RTSP ───

/// Requête confiée au fil RTSP.
enum Job {
    /// Sans attendre la réponse. `key` : une plus récente de même clé remplace celle en attente.
    /// `fatal` : un échec signale la connexion perdue.
    Send { method: &'static str, uri: Option<&'static str>, key: String, headers: Vec<(&'static str, String)>, body: Option<(String, Vec<u8>)>, fatal: bool },
    /// La réponse est attendue par l'appelant.
    Call { method: &'static str, headers: Vec<(&'static str, String)>, reply: mpsc::Sender<Result<(), String>> },
}

/// Fil qui parle à l'appareil : l'envoi du son n'attend jamais une réponse lente.
pub struct Worker {
    jobs: Option<mpsc::Sender<Job>>,
    /// Copie de la connexion : la fermer débloque une requête en cours à l'arrêt.
    stream: Option<TcpStream>,
    error: Arc<Mutex<Option<String>>>,
    thread: Option<JoinHandle<()>>,
}

impl Worker {
    pub fn spawn(mut rtsp: Rtsp) -> Result<Self, String> {
        let stream = rtsp.stream.try_clone().ok();
        let (tx, rx) = mpsc::channel::<Job>();
        let error = Arc::new(Mutex::new(None));
        let failed = error.clone();
        let thread = std::thread::Builder::new()
            .name("rustmusic-airplay-rtsp".into())
            .spawn(move || {
                while let Ok(first) = rx.recv() {
                    let mut batch: Vec<Job> = std::iter::once(first).chain(rx.try_iter()).collect();
                    // Infos en retard : seule la dernière de chaque sorte part.
                    let keep: Vec<bool> = (0..batch.len())
                        .map(|i| match &batch[i] {
                            Job::Send { key, .. } => !batch[i + 1..].iter().any(|j| matches!(j, Job::Send { key: k, .. } if k == key)),
                            Job::Call { .. } => true,
                        })
                        .collect();
                    for (job, keep) in batch.drain(..).zip(keep) {
                        if !keep {
                            continue;
                        }
                        match job {
                            Job::Send { method, uri, headers, body, fatal, .. } => {
                                let body = body.as_ref().map(|(t, b)| (t.as_str(), b.as_slice()));
                                if let Err(e) = rtsp.request(method, uri, &headers, body) {
                                    log::debug!("📡 AirPlay : {method} sans succès ({e})");
                                    if fatal {
                                        *failed.lock().unwrap_or_else(|p| p.into_inner()) = Some(e);
                                    }
                                }
                            }
                            Job::Call { method, headers, reply } => {
                                let _ = reply.send(rtsp.request(method, None, &headers, None).map(drop));
                            }
                        }
                    }
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(Self { jobs: Some(tx), stream, error, thread: Some(thread) })
    }

    /// Dépose une requête et rend la main aussitôt.
    pub fn send(&self, method: &'static str, key: &str, headers: Vec<(&'static str, String)>, body: Option<(&str, &[u8])>, fatal: bool) {
        let uri = (method == "OPTIONS").then_some("*");
        let body = body.map(|(t, b)| (t.to_string(), b.to_vec()));
        if let Some(jobs) = &self.jobs {
            let _ = jobs.send(Job::Send { method, uri, key: key.to_string(), headers, body, fatal });
        }
    }

    /// Envoie une requête et attend sa réponse (au plus `timeout`).
    pub fn call(&self, method: &'static str, headers: Vec<(&'static str, String)>, timeout: Duration) -> Result<(), String> {
        let (reply, answer) = mpsc::channel();
        self.jobs.as_ref().ok_or("connexion fermée")?.send(Job::Call { method, headers, reply }).map_err(|_| "connexion fermée".to_string())?;
        answer.recv_timeout(timeout).map_err(|_| format!("{method} : pas de réponse"))?
    }

    /// Échec d'une requête vitale (maintien de la connexion) depuis le dernier appel.
    pub fn error(&self) -> Option<String> {
        self.error.lock().unwrap_or_else(|p| p.into_inner()).take()
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        // Plus d'expéditeur, connexion coupée : le fil s'arrête sans attendre l'appareil.
        self.jobs.take();
        if let Some(s) = &self.stream {
            let _ = s.shutdown(std::net::Shutdown::Both);
        }
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

fn parse_status(line: &str) -> Option<u16> {
    let mut parts = line.split_whitespace();
    if !parts.next()?.starts_with("RTSP/") {
        return None;
    }
    parts.next()?.parse().ok()
}

/// Valeur d'un paramètre de `Transport` (« server_port=6000 »).
pub fn transport_param(transport: &str, name: &str) -> Option<u16> {
    transport.split(';').find_map(|p| p.trim().strip_prefix(name)?.strip_prefix('=')?.parse().ok())
}

pub fn random_u64() -> u64 {
    let id = uuid::Uuid::new_v4();
    let b = id.as_bytes();
    u64::from_be_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_status_and_transport() {
        assert_eq!(parse_status("RTSP/1.0 200 OK\r\n"), Some(200));
        assert_eq!(parse_status("HTTP/1.1 200 OK"), None);
        let t = "RTP/AVP/UDP;unicast;mode=record;server_port=6000;control_port=6001;timing_port=6002";
        assert_eq!(transport_param(t, "server_port"), Some(6000));
        assert_eq!(transport_param(t, "timing_port"), Some(6002));
        assert_eq!(transport_param(t, "port"), None);
    }

    /// Faux appareil : lit les requêtes une à une, répond après `delay` pour la première.
    fn slow_device(delay: Duration) -> (SocketAddr, std::thread::JoinHandle<Vec<String>>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (s, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(s.try_clone().unwrap());
            let mut out = s;
            let mut methods = Vec::new();
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap_or(0) == 0 {
                    break;
                }
                let method = line.split_whitespace().next().unwrap_or("").to_string();
                let mut len = 0;
                loop {
                    let mut h = String::new();
                    reader.read_line(&mut h).unwrap();
                    if h.trim().is_empty() {
                        break;
                    }
                    if let Some(v) = h.strip_prefix("Content-Length: ") {
                        len = v.trim().parse().unwrap();
                    }
                }
                let mut body = vec![0u8; len];
                reader.read_exact(&mut body).unwrap();
                if methods.is_empty() {
                    std::thread::sleep(delay);
                }
                methods.push(method);
                if out.write_all(b"RTSP/1.0 200 OK\r\nContent-Length: 0\r\n\r\n").is_err() {
                    break;
                }
            }
            methods
        });
        (addr, server)
    }

    #[test]
    fn slow_device_never_blocks_the_sender() {
        let (addr, server) = slow_device(Duration::from_millis(1500));
        let worker = Worker::spawn(Rtsp::connect(addr).unwrap()).unwrap();
        let t = std::time::Instant::now();
        // Pochette, puis deux progressions : la plus ancienne en attente est sautée.
        worker.send("SET_PARAMETER", "image/jpeg", Vec::new(), Some(("image/jpeg", &[1, 2, 3])), false);
        std::thread::sleep(Duration::from_millis(50));
        worker.send("SET_PARAMETER", "text/parameters", Vec::new(), Some(("text/parameters", b"a")), false);
        worker.send("SET_PARAMETER", "text/parameters", Vec::new(), Some(("text/parameters", b"b")), false);
        assert!(t.elapsed() < Duration::from_millis(200), "l'envoi a attendu l'appareil");
        // FLUSH attend sa réponse, derrière la pochette lente.
        worker.call("FLUSH", Vec::new(), Duration::from_secs(5)).unwrap();
        assert!(t.elapsed() >= Duration::from_millis(1400));
        drop(worker);
        assert_eq!(server.join().unwrap(), ["SET_PARAMETER", "SET_PARAMETER", "FLUSH"]);
    }

    #[test]
    fn lost_connection_is_reported() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || drop(listener.accept().unwrap()));
        let worker = Worker::spawn(Rtsp::connect(addr).unwrap()).unwrap();
        server.join().unwrap();
        worker.send("OPTIONS", "keepalive", Vec::new(), None, true);
        let t = std::time::Instant::now();
        while worker.error.lock().unwrap().is_none() && t.elapsed() < Duration::from_secs(3) {
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(worker.error().is_some());
        assert!(worker.error().is_none(), "signalé une seule fois");
    }

    #[test]
    fn talks_to_a_fake_device() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            let mut buf = [0u8; 2048];
            let n = s.read(&mut buf).unwrap();
            let req = String::from_utf8_lossy(&buf[..n]).to_string();
            s.write_all(b"RTSP/1.0 200 OK\r\nCSeq: 1\r\nSession: 1A2B\r\nContent-Length: 2\r\n\r\nok").unwrap();
            req
        });
        let mut rtsp = Rtsp::connect(addr).unwrap();
        let r = rtsp.request("OPTIONS", Some("*"), &[], None).unwrap();
        assert_eq!(r.header("session"), Some("1A2B"));
        assert_eq!(r.body, b"ok");
        let req = server.join().unwrap();
        assert!(req.starts_with("OPTIONS * RTSP/1.0\r\nCSeq: 1\r\n"));
        assert!(req.contains("DACP-ID: "));
    }
}
