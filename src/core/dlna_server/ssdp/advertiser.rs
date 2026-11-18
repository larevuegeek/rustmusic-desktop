//! SSDP Advertiser : sends periodic `NOTIFY ssdp:alive` multicast messages
//! so DLNA clients on the LAN keep us in their device list, and a
//! `NOTIFY ssdp:byebye` on shutdown so clients drop us cleanly.
//!
//! Spec : UPnP Device Architecture 1.0, section 1.2.
//! Group address : `239.255.255.250:1900` (IPv4).

use std::net::{Ipv4Addr, SocketAddr, IpAddr};
use std::sync::Arc;
use std::time::Duration;

use tokio::net::UdpSocket;
use tokio::sync::broadcast;

use crate::core::dlna_server::config::DlnaConfig;
use crate::core::dlna_server::error::DlnaError;
use crate::core::dlna_server::ssdp::listener::build_usns;

const MULTICAST_TARGET: &str = "239.255.255.250:1900";

/// Re-announce period. Must be < CACHE-CONTROL max-age (1800s).
/// We pick 60s during development for quicker discovery feedback.
const ALIVE_INTERVAL_SECS: u64 = 60;

/// Spawn the advertiser loop. Runs until `shutdown` fires, then sends byebye.
pub async fn run_advertiser(
    config: Arc<DlnaConfig>,
    local_ip: Ipv4Addr,
    mut shutdown: broadcast::Receiver<()>,
) -> Result<(), DlnaError> {
    // Une annonce par carte locale (Ethernet, Wi-Fi), chacune avec sa propre adresse :
    // jamais dans un tunnel VPN, l'ampli du salon reçoit l'adresse du réseau local.
    let mut ips = crate::core::network_player::interfaces::lan_ipv4s();
    if ips.is_empty() {
        ips.push(local_ip);
    }
    let mut sockets = Vec::new();
    for ip in ips {
        match open(ip) {
            Ok(socket) => sockets.push((socket, format!("http://{}:{}/description.xml", ip, config.port))),
            Err(e) => log::debug!("SSDP : annonce impossible sur {ip} ({e})"),
        }
    }
    if sockets.is_empty() {
        return Err(DlnaError::Io(std::io::Error::other("aucune carte réseau pour annoncer le serveur")));
    }
    let usns = build_usns(&config.uuid);

    log::debug!(
        "SSDP advertiser → {} carte(s) [{}] (re-announce {}s, {} NT)",
        sockets.len(),
        sockets.iter().map(|(_, l)| l.as_str()).collect::<Vec<_>>().join(", "),
        ALIVE_INTERVAL_SECS,
        usns.len()
    );

    // Initial burst : send 3 times rapidly so freshly-listening clients catch us.
    for _ in 0..3 {
        for (socket, location) in &sockets {
            send_alive(socket, location, &usns).await;
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
    }

    // Periodic re-announce
    loop {
        tokio::select! {
            _ = shutdown.recv() => {
                for (socket, _) in &sockets {
                    send_byebye(socket, &usns).await;
                }
                break;
            }
            _ = tokio::time::sleep(Duration::from_secs(ALIVE_INTERVAL_SECS)) => {
                for (socket, location) in &sockets {
                    send_alive(socket, location, &usns).await;
                }
                log::debug!("SSDP NOTIFY alive sent ({} NT entries)", usns.len());
            }
        }
    }
    log::debug!("SSDP advertiser stopped (byebye sent)");
    Ok(())
}

/// Socket d'annonce sur la carte `ip` : la lier à son adresse fixe la carte de sortie du multicast.
fn open(ip: Ipv4Addr) -> std::io::Result<UdpSocket> {
    let std_sock = std::net::UdpSocket::bind(SocketAddr::new(IpAddr::V4(ip), 0))?;
    std_sock.set_nonblocking(true)?;
    // Bump multicast TTL to 4 (default is 1) so packets traverse a few hops
    // if routers/switches reduce TTL on the way.
    if let Err(e) = std_sock.set_multicast_ttl_v4(4) {
        log::warn!("set_multicast_ttl_v4 failed: {}", e);
    }
    UdpSocket::from_std(std_sock)
}

// ─── Message senders ─────────────────────────────────────────────────

async fn send_alive(socket: &UdpSocket, location: &str, usns: &[(String, String)]) {
    for (nt, usn) in usns {
        let msg = format!(
            "NOTIFY * HTTP/1.1\r\n\
             HOST: 239.255.255.250:1900\r\n\
             CACHE-CONTROL: max-age=1800\r\n\
             LOCATION: {location}\r\n\
             NT: {nt}\r\n\
             NTS: ssdp:alive\r\n\
             SERVER: RustMusic/1.0 UPnP/1.0\r\n\
             USN: {usn}\r\n\
             \r\n"
        );
        if let Err(e) = socket.send_to(msg.as_bytes(), MULTICAST_TARGET).await {
            log::debug!("SSDP send_alive failed: {}", e);
        }
    }
}

async fn send_byebye(socket: &UdpSocket, usns: &[(String, String)]) {
    for (nt, usn) in usns {
        let msg = format!(
            "NOTIFY * HTTP/1.1\r\n\
             HOST: 239.255.255.250:1900\r\n\
             NT: {nt}\r\n\
             NTS: ssdp:byebye\r\n\
             USN: {usn}\r\n\
             \r\n"
        );
        let _ = socket.send_to(msg.as_bytes(), MULTICAST_TARGET).await;
    }
}
