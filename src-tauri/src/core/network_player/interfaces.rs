//! Adresses IPv4 des cartes réseau actives, pour chercher les lecteurs sur chaque réseau.
//!
//! Avec un VPN, l'interface par défaut n'est pas celle du salon : la recherche
//! multicast part donc par chaque carte. Windows : `GetAdaptersAddresses` ;
//! Linux et macOS : `getifaddrs`.

use std::net::Ipv4Addr;

/// Une adresse d'une carte active.
pub struct Iface {
    pub ip: Ipv4Addr,
    /// Vraie carte du réseau local (Ethernet, Wi-Fi), pas un tunnel VPN ni une carte virtuelle.
    pub lan: bool,
}

fn usable() -> Vec<Iface> {
    let mut all = platform::list();
    all.retain(|i| !i.ip.is_loopback() && !i.ip.is_link_local() && !i.ip.is_unspecified());
    all
}

/// Adresses IPv4 utilisables (ni bouclage, ni lien local 169.254), sans doublon.
pub fn local_ipv4s() -> Vec<Ipv4Addr> {
    let mut ips: Vec<Ipv4Addr> = usable().into_iter().map(|i| i.ip).collect();
    ips.sort();
    ips.dedup();
    ips
}

/// Adresses des vraies cartes locales (Ethernet, Wi-Fi) : ce que le serveur de
/// bibliothèque annonce, pour ne pas s'exposer dans un tunnel VPN. Toutes, à défaut.
pub fn lan_ipv4s() -> Vec<Ipv4Addr> {
    let all = usable();
    let mut ips: Vec<Ipv4Addr> = all.iter().filter(|i| i.lan).map(|i| i.ip).collect();
    if ips.is_empty() {
        ips = all.iter().map(|i| i.ip).collect();
    }
    ips.sort();
    ips.dedup();
    ips
}

/// L'adresse de l'ordinateur sur le réseau de la maison. Avec un VPN (WireGuard…),
/// la route par défaut passe par le tunnel : on préfère la vraie carte locale.
pub fn lan_ipv4() -> Option<Ipv4Addr> {
    let all = usable();
    all.iter()
        .find(|i| i.lan && i.ip.is_private())
        .or_else(|| all.iter().find(|i| i.lan))
        .or_else(|| all.iter().find(|i| i.ip.is_private()))
        .map(|i| i.ip)
}

/// L'adresse locale par laquelle on joint `peer` : celle à donner au lecteur
/// pour qu'il vienne chercher la musique. Aucun paquet n'est envoyé.
pub fn local_ip_toward(peer: Ipv4Addr) -> Option<Ipv4Addr> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect((peer, 9)).ok()?;
    match socket.local_addr().ok()? {
        std::net::SocketAddr::V4(v4) => Some(*v4.ip()),
        _ => None,
    }
}

#[cfg(unix)]
mod platform {
    use std::ffi::CStr;
    use std::net::Ipv4Addr;

    use super::Iface;

    /// Tunnels et cartes virtuelles, d'après leur nom.
    const VIRTUAL: &[&str] = &["wg", "tun", "tap", "utun", "ipsec", "ppp", "tailscale", "zt", "docker", "veth", "br-", "virbr", "vmnet", "vboxnet", "lo"];

    pub fn list() -> Vec<Iface> {
        let mut out = Vec::new();
        let mut head: *mut libc::ifaddrs = std::ptr::null_mut();
        // SAFETY : liste allouée par le système, parcourue puis rendue par `freeifaddrs`.
        unsafe {
            if libc::getifaddrs(&mut head) != 0 {
                return out;
            }
            let mut cur = head;
            while !cur.is_null() {
                let ifa = &*cur;
                let up = ifa.ifa_flags & (libc::IFF_UP as libc::c_uint) != 0;
                if up && !ifa.ifa_addr.is_null() && (*ifa.ifa_addr).sa_family as i32 == libc::AF_INET {
                    let sin = &*(ifa.ifa_addr as *const libc::sockaddr_in);
                    let name = if ifa.ifa_name.is_null() { String::new() } else { CStr::from_ptr(ifa.ifa_name).to_string_lossy().into_owned() };
                    let point_to_point = ifa.ifa_flags & (libc::IFF_POINTOPOINT as libc::c_uint) != 0;
                    let lan = !point_to_point && !VIRTUAL.iter().any(|p| name.starts_with(p));
                    out.push(Iface { ip: Ipv4Addr::from(u32::from_be(sin.sin_addr.s_addr)), lan });
                }
                cur = ifa.ifa_next;
            }
            libc::freeifaddrs(head);
        }
        out
    }
}

#[cfg(windows)]
mod platform {
    use std::net::Ipv4Addr;

    use windows::Win32::NetworkManagement::IpHelper::{
        GetAdaptersAddresses, GAA_FLAG_INCLUDE_GATEWAYS, GAA_FLAG_SKIP_ANYCAST, GAA_FLAG_SKIP_DNS_SERVER,
        GAA_FLAG_SKIP_MULTICAST, IP_ADAPTER_ADDRESSES_LH,
    };
    use windows::Win32::NetworkManagement::Ndis::IfOperStatusUp;
    use windows::Win32::Networking::WinSock::{AF_INET, SOCKADDR_IN};

    use super::Iface;

    /// Ethernet et Wi-Fi ; un tunnel WireGuard ou OpenVPN est d'un autre type.
    const IF_TYPE_ETHERNET: u32 = 6;
    const IF_TYPE_WIFI: u32 = 71;

    pub fn list() -> Vec<Iface> {
        let mut out = Vec::new();
        let flags = GAA_FLAG_SKIP_ANYCAST | GAA_FLAG_SKIP_MULTICAST | GAA_FLAG_SKIP_DNS_SERVER | GAA_FLAG_INCLUDE_GATEWAYS;
        // Taille conseillée par la doc : 15 Ko, agrandie si le système en demande plus.
        let mut size: u32 = 15 * 1024;
        let mut buf: Vec<u64> = Vec::new();
        for _ in 0..3 {
            buf = vec![0u64; (size as usize).div_ceil(8)];
            // SAFETY : tampon aligné sur 8 octets, de la taille annoncée.
            let ret = unsafe {
                GetAdaptersAddresses(
                    AF_INET.0 as u32,
                    flags,
                    None,
                    Some(buf.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH),
                    &mut size,
                )
            };
            match ret {
                0 => break,
                111 => continue, // ERROR_BUFFER_OVERFLOW : `size` a été mis à jour.
                _ => return out,
            }
        }
        // SAFETY : liste chaînée écrite par le système dans `buf`, qui vit jusqu'à la fin.
        unsafe {
            let mut adapter = buf.as_ptr() as *const IP_ADAPTER_ADDRESSES_LH;
            while !adapter.is_null() {
                let a = &*adapter;
                if a.OperStatus == IfOperStatusUp {
                    // Carte physique avec passerelle : celle du réseau de la maison.
                    let lan = matches!(a.IfType, IF_TYPE_ETHERNET | IF_TYPE_WIFI) && !a.FirstGatewayAddress.is_null();
                    let mut unicast = a.FirstUnicastAddress;
                    while !unicast.is_null() {
                        let u = &*unicast;
                        let sa = u.Address.lpSockaddr;
                        if !sa.is_null() && (*sa).sa_family == AF_INET {
                            let sin = &*(sa as *const SOCKADDR_IN);
                            out.push(Iface { ip: Ipv4Addr::from(u32::from_be(sin.sin_addr.S_un.S_addr)), lan });
                        }
                        unicast = u.Next;
                    }
                }
                adapter = a.Next;
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Adresse retenue sur cette machine : `cargo test --lib lan_here -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn lan_here() {
        for i in usable() {
            println!("{} lan={}", i.ip, i.lan);
        }
        println!("retenue : {:?}", lan_ipv4());
    }

    #[test]
    fn lists_without_loopback() {
        let ips = local_ipv4s();
        assert!(ips.iter().all(|ip| !ip.is_loopback()));
    }
}
