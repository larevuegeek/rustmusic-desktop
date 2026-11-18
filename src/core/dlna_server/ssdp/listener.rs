//! SSDP M-SEARCH responder.
//!
//! Listens on the SSDP multicast group `239.255.255.250:1900` for
//! `M-SEARCH * HTTP/1.1` requests sent by clients looking for media
//! servers. When the search target matches one of our USNs (`ssdp:all`,
//! `upnp:rootdevice`, our device type, our service types, or our UUID),
//! we reply unicast to the requester with a 200 OK pointing to our
//! description URL.
//!
//! Reference : UPnP Device Architecture 1.0, section 1.3.

use std::net::Ipv4Addr;
use std::sync::Arc;

use tokio::net::UdpSocket;
use tokio::sync::broadcast;

use crate::core::dlna_server::config::DlnaConfig;
use crate::core::dlna_server::error::DlnaError;

const MULTICAST_ADDR: Ipv4Addr = Ipv4Addr::new(239, 255, 255, 250);
const SSDP_PORT: u16 = 1900;

/// Spawn the M-SEARCH listener loop. Runs until `shutdown` fires.
/// If port 1900 is already in use (typical on Windows : SSDP Discovery
/// service), we log a warning and exit silently — the advertiser still
/// sends NOTIFY alive periodically so clients can find us that way.
pub async fn run_listener(
    config: Arc<DlnaConfig>,
    local_ip: Ipv4Addr,
    mut shutdown: broadcast::Receiver<()>,
) -> Result<(), DlnaError> {
    let std_sock = match std::net::UdpSocket::bind(("0.0.0.0", SSDP_PORT)) {
        Ok(s) => s,
        Err(e) => {
            log::warn!(
                "SSDP listener cannot bind 0.0.0.0:{} ({}). \
                 M-SEARCH responses disabled (NOTIFY alive still active). \
                 On Windows, this is usually the SSDP Discovery service holding the port.",
                SSDP_PORT,
                e
            );
            return Ok(());
        }
    };
    std_sock.set_nonblocking(true).map_err(DlnaError::Io)?;
    // Groupe rejoint sur chaque carte locale, pas sur les tunnels VPN.
    let mut joined = Vec::new();
    for ip in crate::core::network_player::interfaces::lan_ipv4s() {
        if std_sock.join_multicast_v4(&MULTICAST_ADDR, &ip).is_ok() {
            joined.push(ip);
        }
    }
    if joined.is_empty() {
        std_sock.join_multicast_v4(&MULTICAST_ADDR, &local_ip).map_err(DlnaError::Io)?;
        joined.push(local_ip);
    }
    let socket = UdpSocket::from_std(std_sock).map_err(DlnaError::Io)?;

    log::debug!("SSDP listener bound on 0.0.0.0:1900, multicast joined on {:?} (M-SEARCH responder)", joined);

    let mut buf = [0u8; 2048];
    loop {
        tokio::select! {
            _ = shutdown.recv() => break,
            recv = socket.recv_from(&mut buf) => {
                match recv {
                    Ok((n, src)) => {
                        let msg = std::str::from_utf8(&buf[..n]).unwrap_or("");
                        if msg.starts_with("M-SEARCH") {
                            // L'adresse donnée est celle de la carte qui mène au demandeur.
                            let reply_ip = match src.ip() {
                                std::net::IpAddr::V4(peer) => crate::core::network_player::interfaces::local_ip_toward(peer).unwrap_or(local_ip),
                                _ => local_ip,
                            };
                            if let Some(responses) = handle_msearch(msg, &config, reply_ip) {
                                log::debug!(
                                    "M-SEARCH from {} → replying with {} USN(s)",
                                    src, responses.len()
                                );
                                for response in responses {
                                    let _ = socket.send_to(response.as_bytes(), src).await;
                                }
                            }
                        }
                    }
                    Err(e) => log::debug!("SSDP recv error: {}", e),
                }
            }
        }
    }
    log::debug!("SSDP listener stopped");
    Ok(())
}

// ─── M-SEARCH parsing & response ─────────────────────────────────────

fn handle_msearch(msg: &str, config: &DlnaConfig, local_ip: Ipv4Addr) -> Option<Vec<String>> {
    if !msg.starts_with("M-SEARCH") {
        return None;
    }

    // Extract ST header (case-insensitive)
    let mut st = String::new();
    for line in msg.lines().skip(1) {
        let lower = line.to_ascii_lowercase();
        if let Some(rest) = lower.strip_prefix("st:") {
            st = rest.trim().to_string();
            break;
        }
    }

    if st.is_empty() {
        return None;
    }

    let location = format!("http://{}:{}/description.xml", local_ip, config.port);
    let usns = build_usns(&config.uuid);

    let mut responses = Vec::new();
    for (target, usn) in &usns {
        if st == "ssdp:all" || st == *target {
            responses.push(build_response(&location, target, usn));
        }
    }

    if responses.is_empty() {
        None
    } else {
        Some(responses)
    }
}

fn build_response(location: &str, st: &str, usn: &str) -> String {
    format!(
        "HTTP/1.1 200 OK\r\n\
         CACHE-CONTROL: max-age=1800\r\n\
         EXT:\r\n\
         LOCATION: {location}\r\n\
         SERVER: RustMusic/1.0 UPnP/1.0\r\n\
         ST: {st}\r\n\
         USN: {usn}\r\n\
         \r\n"
    )
}

// ─── USN list (shared with the advertiser) ───────────────────────────

/// Build the list of (NT, USN) pairs we advertise / respond for.
/// Order matters: clients expect rootdevice, uuid, then device + services.
pub(in crate::core::dlna_server::ssdp) fn build_usns(uuid: &str) -> Vec<(String, String)> {
    vec![
        (
            "upnp:rootdevice".to_string(),
            format!("uuid:{uuid}::upnp:rootdevice"),
        ),
        (format!("uuid:{uuid}"), format!("uuid:{uuid}")),
        (
            "urn:schemas-upnp-org:device:MediaServer:1".to_string(),
            format!("uuid:{uuid}::urn:schemas-upnp-org:device:MediaServer:1"),
        ),
        (
            "urn:schemas-upnp-org:service:ContentDirectory:1".to_string(),
            format!("uuid:{uuid}::urn:schemas-upnp-org:service:ContentDirectory:1"),
        ),
        (
            "urn:schemas-upnp-org:service:ConnectionManager:1".to_string(),
            format!("uuid:{uuid}::urn:schemas-upnp-org:service:ConnectionManager:1"),
        ),
    ]
}
