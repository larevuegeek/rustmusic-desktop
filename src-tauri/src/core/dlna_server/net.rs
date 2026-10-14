//! Tiny networking helpers shared by the DLNA server and its Tauri commands.

use std::net::{Ipv4Addr, SocketAddr};

/// Adresse de l'ordinateur sur le réseau de la maison, affichée et annoncée par défaut.
/// La vraie carte locale passe avant la route par défaut, qu'un VPN (WireGuard…) détourne.
pub fn detect_local_ipv4() -> Ipv4Addr {
    if let Some(ip) = crate::core::network_player::interfaces::lan_ipv4() {
        return ip;
    }
    let socket = match std::net::UdpSocket::bind("0.0.0.0:0") {
        Ok(s) => s,
        Err(_) => return Ipv4Addr::LOCALHOST,
    };
    if socket.connect("8.8.8.8:80").is_err() {
        return Ipv4Addr::LOCALHOST;
    }
    match socket.local_addr() {
        Ok(SocketAddr::V4(v4)) => *v4.ip(),
        _ => Ipv4Addr::LOCALHOST,
    }
}
