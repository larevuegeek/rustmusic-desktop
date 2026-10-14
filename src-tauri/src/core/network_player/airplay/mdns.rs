//! Recherche mDNS (Bonjour) minimale : une question PTR, les réponses lues telles quelles.
//!
//! La question part d'un port quelconque : l'appareil répond alors directement à
//! l'expéditeur (RFC 6762 §6.7), sans avoir à écouter le port 5353 que Bonjour occupe.

use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::time::Duration;

use super::super::discovery::open_socket;
use super::super::interfaces::local_ipv4s;

const MDNS_ADDR: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 251);
const MDNS_PORT: u16 = 5353;

const TYPE_A: u16 = 1;
const TYPE_PTR: u16 = 12;
const TYPE_TXT: u16 = 16;
const TYPE_SRV: u16 = 33;

/// Une instance de service annoncée.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Service {
    /// Premier label du nom d'instance (« 0011223344@Salon »).
    pub instance: String,
    pub port: u16,
    pub ip: Option<Ipv4Addr>,
    pub txt: Vec<(String, String)>,
}

impl Service {
    pub fn txt(&self, key: &str) -> Option<&str> {
        self.txt.iter().find(|(k, _)| k.eq_ignore_ascii_case(key)).map(|(_, v)| v.as_str())
    }
}

/// Instances de `service` (« _raop._tcp.local ») qui répondent pendant `wait`.
pub async fn browse(service: &str, wait: Duration) -> Vec<Service> {
    let query = build_query(service);
    let sockets: Vec<_> = local_ipv4s().into_iter().filter_map(|ip| open_socket(ip).ok()).collect();
    let target = SocketAddr::V4(SocketAddrV4::new(MDNS_ADDR, MDNS_PORT));

    let mut records = Records::default();
    let deadline = tokio::time::Instant::now() + wait;
    let mut buf = vec![0u8; 9000];
    let mut resend = tokio::time::Instant::now();
    loop {
        let now = tokio::time::Instant::now();
        if now >= deadline {
            break;
        }
        // Deux envois : un paquet multicast se perd facilement en Wi-Fi.
        if now >= resend {
            for s in &sockets {
                let _ = s.send_to(&query, target).await;
            }
            resend = now + Duration::from_millis(1000);
        }
        let step = deadline.min(resend);
        for s in &sockets {
            while let Ok((n, from)) = s.try_recv_from(&mut buf) {
                if let SocketAddr::V4(from) = from {
                    records.read_packet(&buf[..n], *from.ip());
                }
            }
        }
        tokio::time::sleep_until(step.min(now + Duration::from_millis(50))).await;
    }
    records.services(service)
}

/// Question PTR pour `service`.
fn build_query(service: &str) -> Vec<u8> {
    let mut q = vec![0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0];
    write_name(&mut q, service);
    q.extend_from_slice(&TYPE_PTR.to_be_bytes());
    q.extend_from_slice(&1u16.to_be_bytes());
    q
}

fn write_name(out: &mut Vec<u8>, name: &str) {
    for label in name.trim_end_matches('.').split('.') {
        out.push(label.len() as u8);
        out.extend_from_slice(label.as_bytes());
    }
    out.push(0);
}

/// Enregistrements accumulés au fil des réponses ; noms en minuscules.
#[derive(Default)]
struct Records {
    /// service → instances (nom complet).
    ptr: HashMap<String, Vec<String>>,
    /// instance → (hôte, port).
    srv: HashMap<String, (String, u16)>,
    txt: HashMap<String, Vec<(String, String)>>,
    a: HashMap<String, Ipv4Addr>,
    /// instance → adresse de l'expéditeur, si l'enregistrement A manque.
    source: HashMap<String, Ipv4Addr>,
    /// nom complet en minuscules → premier label tel qu'annoncé.
    label: HashMap<String, String>,
}

impl Records {
    fn read_packet(&mut self, p: &[u8], from: Ipv4Addr) {
        let _ = self.try_read(p, from);
    }

    fn try_read(&mut self, p: &[u8], from: Ipv4Addr) -> Option<()> {
        if p.len() < 12 || p[2] & 0x80 == 0 {
            return None; // pas une réponse
        }
        let count = |i: usize| u16::from_be_bytes([p[i], p[i + 1]]) as usize;
        let (qd, total) = (count(4), count(6) + count(8) + count(10));
        let mut pos = 12;
        for _ in 0..qd {
            pos = read_name(p, pos)?.1 + 4;
        }
        for _ in 0..total {
            let (labels, after) = read_name(p, pos)?;
            let head = p.get(after..after + 10)?;
            let rtype = u16::from_be_bytes([head[0], head[1]]);
            let len = u16::from_be_bytes([head[8], head[9]]) as usize;
            let start = after + 10;
            let data = p.get(start..start + len)?;
            let name = join(&labels);
            match rtype {
                TYPE_PTR => {
                    let (target, _) = read_name(p, start)?;
                    let full = join(&target);
                    if let Some(first) = target.first() {
                        self.label.insert(full.clone(), first.clone());
                    }
                    self.source.insert(full.clone(), from);
                    let list = self.ptr.entry(name).or_default();
                    if !list.contains(&full) {
                        list.push(full);
                    }
                }
                TYPE_SRV if len >= 7 => {
                    let port = u16::from_be_bytes([data[4], data[5]]);
                    let (host, _) = read_name(p, start + 6)?;
                    self.source.insert(name.clone(), from);
                    self.srv.insert(name, (join(&host), port));
                }
                TYPE_TXT => {
                    self.txt.insert(name, parse_txt(data));
                }
                TYPE_A if len == 4 => {
                    self.a.insert(name, Ipv4Addr::new(data[0], data[1], data[2], data[3]));
                }
                _ => {}
            }
            pos = start + len;
        }
        Some(())
    }

    fn services(&self, service: &str) -> Vec<Service> {
        let key = service.trim_end_matches('.').to_ascii_lowercase();
        let Some(instances) = self.ptr.get(&key) else { return Vec::new() };
        instances
            .iter()
            .filter_map(|full| {
                let (host, port) = self.srv.get(full)?;
                Some(Service {
                    instance: self.label.get(full).cloned().unwrap_or_default(),
                    port: *port,
                    ip: self.a.get(host).copied().or_else(|| self.source.get(full).copied()),
                    txt: self.txt.get(full).cloned().unwrap_or_default(),
                })
            })
            .collect()
    }
}

fn join(labels: &[String]) -> String {
    labels.iter().map(|l| l.to_lowercase()).collect::<Vec<_>>().join(".")
}

/// Nom DNS à `pos`, pointeurs de compression compris ; rend les labels et la fin du nom.
fn read_name(p: &[u8], mut pos: usize) -> Option<(Vec<String>, usize)> {
    let mut labels = Vec::new();
    let mut end = None;
    // Garde-fou contre les boucles de pointeurs.
    for _ in 0..64 {
        let len = *p.get(pos)? as usize;
        match len {
            0 => return Some((labels, end.unwrap_or(pos + 1))),
            l if l & 0xC0 == 0xC0 => {
                let target = ((l & 0x3F) << 8) | *p.get(pos + 1)? as usize;
                end.get_or_insert(pos + 2);
                pos = target;
            }
            l => {
                let raw = p.get(pos + 1..pos + 1 + l)?;
                labels.push(String::from_utf8_lossy(raw).into_owned());
                pos += 1 + l;
            }
        }
    }
    None
}

fn parse_txt(data: &[u8]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut pos = 0;
    while pos < data.len() {
        let len = data[pos] as usize;
        let Some(entry) = data.get(pos + 1..pos + 1 + len) else { break };
        let entry = String::from_utf8_lossy(entry);
        match entry.split_once('=') {
            Some((k, v)) => out.push((k.to_string(), v.to_string())),
            None if !entry.is_empty() => out.push((entry.to_string(), String::new())),
            None => {}
        }
        pos += 1 + len;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Réponse typique : PTR, puis SRV/TXT/A en section additionnelle, noms compressés.
    fn response() -> Vec<u8> {
        let mut p = vec![0, 0, 0x84, 0, 0, 0, 0, 1, 0, 0, 0, 3];
        // PTR _raop._tcp.local → 0011@Salon._raop._tcp.local
        let service_at = p.len();
        write_name(&mut p, "_raop._tcp.local");
        p.extend_from_slice(&[0, 12, 0, 1, 0, 0, 0x11, 0x94]);
        let instance_rdata = {
            let mut r = vec![10];
            r.extend_from_slice(b"0011@Salon");
            r.extend_from_slice(&[0xC0, service_at as u8]);
            r
        };
        p.extend_from_slice(&(instance_rdata.len() as u16).to_be_bytes());
        let instance_at = p.len();
        p.extend_from_slice(&instance_rdata);
        // SRV : port 7000, hôte onkyo.local
        p.extend_from_slice(&[0xC0, instance_at as u8, 0, 33, 0x80, 1, 0, 0, 0, 120]);
        let mut srv = vec![0, 0, 0, 0, 0x1B, 0x58];
        let host_at = p.len() + 2 + srv.len();
        srv.extend_from_slice(&[5]);
        srv.extend_from_slice(b"onkyo");
        srv.extend_from_slice(&[5]);
        srv.extend_from_slice(b"local");
        srv.push(0);
        p.extend_from_slice(&(srv.len() as u16).to_be_bytes());
        p.extend_from_slice(&srv);
        // TXT
        p.extend_from_slice(&[0xC0, instance_at as u8, 0, 16, 0x80, 1, 0, 0, 0, 120]);
        let mut txt = Vec::new();
        for e in ["et=0,3,5", "cn=0,1", "am=TX-RZ900", "flag"] {
            txt.push(e.len() as u8);
            txt.extend_from_slice(e.as_bytes());
        }
        p.extend_from_slice(&(txt.len() as u16).to_be_bytes());
        p.extend_from_slice(&txt);
        // A
        p.extend_from_slice(&[0xC0, host_at as u8, 0, 1, 0x80, 1, 0, 0, 0, 120, 0, 4, 192, 168, 1, 30]);
        p
    }

    #[test]
    fn reads_a_compressed_answer() {
        let mut r = Records::default();
        r.read_packet(&response(), Ipv4Addr::new(192, 168, 1, 99));
        let found = r.services("_raop._tcp.local");
        assert_eq!(found.len(), 1);
        let s = &found[0];
        assert_eq!(s.instance, "0011@Salon");
        assert_eq!(s.port, 7000);
        assert_eq!(s.ip, Some(Ipv4Addr::new(192, 168, 1, 30)));
        assert_eq!(s.txt("et"), Some("0,3,5"));
        assert_eq!(s.txt("AM"), Some("TX-RZ900"));
        assert_eq!(s.txt("flag"), Some(""));
    }

    #[test]
    fn falls_back_to_the_sender_address() {
        let mut p = response();
        // Coupe l'enregistrement A (16 derniers octets) et corrige le compte.
        p.truncate(p.len() - 16);
        p[11] = 2;
        let mut r = Records::default();
        r.read_packet(&p, Ipv4Addr::new(192, 168, 1, 99));
        assert_eq!(r.services("_raop._tcp.local")[0].ip, Some(Ipv4Addr::new(192, 168, 1, 99)));
    }

    #[test]
    fn rejects_pointer_loops_and_queries() {
        assert!(read_name(&[0xC0, 0], 0).is_none());
        let mut r = Records::default();
        r.read_packet(&build_query("_raop._tcp.local"), Ipv4Addr::LOCALHOST);
        assert!(r.services("_raop._tcp.local").is_empty());
    }
}
