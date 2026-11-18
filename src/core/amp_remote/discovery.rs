//! Recherche des amplis pilotables sur le réseau local.

use std::collections::HashSet;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::time::Duration;

use tokio::net::UdpSocket;

use super::model::Amp;
use super::{denon, onkyo, yamaha};
use crate::core::network_player::interfaces::{lan_ipv4s, local_ipv4s};

/// Amplis qui répondent pendant `wait`, triés par nom.
pub async fn discover(wait: Duration) -> Vec<Amp> {
    let (mut found, upnp) = tokio::join!(onkyo_search(wait), upnp_search(wait));
    found.extend(upnp);
    let mut seen = HashSet::new();
    found.retain(|a| seen.insert(a.id.clone()));
    found.sort_by_key(|a| a.name.to_lowercase());
    found
}

/// L'ampli `id`, où qu'il soit : son adresse a pu changer (bail DHCP renouvelé).
pub async fn find(id: &str, wait: Duration) -> Option<Amp> {
    discover(wait).await.into_iter().find(|a| a.id == id)
}

/// Denon, Marantz et Yamaha s'annoncent comme lecteurs UPnP : on garde les amplis.
async fn upnp_search(wait: Duration) -> Vec<Amp> {
    crate::core::network_player::discovery::discover(wait)
        .await
        .into_iter()
        .filter_map(|r| {
            let ip = r.ip.to_string();
            denon::amp_from_upnp(&r.id, &r.name, &r.manufacturer, &r.model, &ip)
                .or_else(|| yamaha::amp_from_upnp(&r.id, &r.name, &r.manufacturer, &r.model, &ip))
        })
        .collect()
}

/// Onkyo/Pioneer/Integra : question diffusée sur chaque carte réseau, réponse directe de l'ampli.
async fn onkyo_search(wait: Duration) -> Vec<Amp> {
    let mut sockets = Vec::new();
    for ip in local_ipv4s() {
        let Ok(socket) = UdpSocket::bind(SocketAddrV4::new(ip, 0)).await else { continue };
        if socket.set_broadcast(true).is_ok() {
            sockets.push(socket);
        }
    }
    let target = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::BROADCAST, onkyo::PORT));
    let lan = lan_ipv4s();
    // Deux passages : au premier, Windows résout encore les adresses (ARP) et en perd.
    for round in 0..2 {
        if round > 0 {
            tokio::time::sleep(Duration::from_millis(400)).await;
        }
        for socket in &sockets {
            for q in onkyo::discovery_queries() {
                let _ = socket.send_to(&q, target).await;
            }
            // Certains amplis ignorent la diffusion mais répondent à une question directe :
            // on la pose à chaque adresse du réseau local (cartes Ethernet/Wi-Fi seulement).
            let Ok(SocketAddr::V4(own)) = socket.local_addr() else { continue };
            if !lan.contains(own.ip()) {
                continue;
            }
            for host in neighbours(*own.ip()) {
                for q in onkyo::discovery_queries() {
                    let _ = socket.send_to(&q, SocketAddrV4::new(host, onkyo::PORT)).await;
                }
            }
        }
    }

    let mut found = Vec::new();
    let deadline = tokio::time::Instant::now() + wait;
    let mut buf = vec![0u8; 2048];
    while tokio::time::Instant::now() < deadline {
        for socket in &sockets {
            loop {
                match socket.try_recv_from(&mut buf) {
                    Ok((n, from)) => {
                        if let Some(amp) = onkyo::parse_discovery(&buf[..n], &from.ip().to_string()) {
                            found.push(amp);
                        }
                    }
                    // Windows : le « port injoignable » d'un voisin interrogé remonte ici ; la suite reste à lire.
                    Err(e) if e.kind() == std::io::ErrorKind::ConnectionReset => continue,
                    Err(_) => break,
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    found
}

/// Les autres adresses du /24 de `own`.
fn neighbours(own: Ipv4Addr) -> impl Iterator<Item = Ipv4Addr> {
    let [a, b, c, me] = own.octets();
    (1..=254u8).filter(move |&d| d != me).map(move |d| Ipv4Addr::new(a, b, c, d))
}

#[cfg(test)]
mod tests {
    #[test]
    fn sweeps_the_local_network_but_not_itself() {
        let all: Vec<_> = super::neighbours("10.0.0.121".parse().unwrap()).collect();
        assert_eq!(all.len(), 253);
        assert!(!all.contains(&"10.0.0.121".parse().unwrap()));
        assert!(all.contains(&"10.0.0.219".parse().unwrap()));
    }
}

#[cfg(test)]
mod live {
    /// Recherche réelle : `cargo test --lib amps_here -- --ignored --nocapture`.
    #[tokio::test]
    #[ignore]
    async fn amps_here() {
        println!("{:#?}", super::discover(std::time::Duration::from_secs(2)).await);
    }
}
