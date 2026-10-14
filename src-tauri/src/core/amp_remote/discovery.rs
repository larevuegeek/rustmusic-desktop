//! Recherche des amplis pilotables sur le réseau local.

use std::collections::HashSet;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::time::Duration;

use tokio::net::UdpSocket;

use super::model::Amp;
use super::onkyo;
use crate::core::network_player::interfaces::local_ipv4s;

/// Amplis qui répondent pendant `wait`, triés par nom.
pub async fn discover(wait: Duration) -> Vec<Amp> {
    let mut found = onkyo_search(wait).await;
    let mut seen = HashSet::new();
    found.retain(|a| seen.insert(a.id.clone()));
    found.sort_by_key(|a| a.name.to_lowercase());
    found
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
    for socket in &sockets {
        for q in onkyo::discovery_queries() {
            let _ = socket.send_to(&q, target).await;
        }
    }

    let mut found = Vec::new();
    let deadline = tokio::time::Instant::now() + wait;
    let mut buf = vec![0u8; 2048];
    while tokio::time::Instant::now() < deadline {
        for socket in &sockets {
            while let Ok((n, from)) = socket.try_recv_from(&mut buf) {
                if let Some(amp) = onkyo::parse_discovery(&buf[..n], &from.ip().to_string()) {
                    found.push(amp);
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    found
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
