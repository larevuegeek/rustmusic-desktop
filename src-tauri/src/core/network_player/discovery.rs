//! Recherche des lecteurs réseau (SSDP M-SEARCH) et lecture de leur description.
//!
//! La recherche part de chaque carte réseau : avec un VPN, l'interface par défaut
//! n'est pas celle du salon.

use std::collections::HashSet;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::time::Duration;

use serde::Serialize;
use socket2::{Domain, Protocol, Socket, Type};
use tokio::net::UdpSocket;

use super::interfaces::local_ipv4s;
use crate::helper::xml;

const SSDP_ADDR: Ipv4Addr = Ipv4Addr::new(239, 255, 255, 250);
const SSDP_PORT: u16 = 1900;
const RENDERER_TYPE: &str = "urn:schemas-upnp-org:device:MediaRenderer:1";
/// Les Sonos s'annoncent d'abord ainsi ; leur lecteur est un sous-appareil.
const SONOS_TYPE: &str = "urn:schemas-upnp-org:device:ZonePlayer:1";

/// Un lecteur réseau prêt à être piloté.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Renderer {
    /// UDN (`uuid:…`) : stable d'une session à l'autre, sert d'identifiant.
    pub id: String,
    pub name: String,
    pub manufacturer: String,
    pub model: String,
    pub icon_url: Option<String>,
    /// Adresse IP de l'appareil.
    pub ip: Ipv4Addr,
    /// Adresse de sa description : de quoi le retrouver au lancement sans recherche.
    #[serde(skip)]
    pub location: String,
    #[serde(skip)]
    pub av_transport: String,
    #[serde(skip)]
    pub rendering_control: Option<String>,
    #[serde(skip)]
    pub connection_manager: Option<String>,
    /// Sonos : service des groupes, pour trouver l'enceinte qui pilote le groupe.
    #[serde(skip)]
    pub group_topology: Option<String>,
}

/// Cherche les lecteurs pendant `wait`, sur chaque réseau. Les appareils muets ou
/// dont la description est illisible sont ignorés.
pub async fn discover(wait: Duration) -> Vec<Renderer> {
    let locations = search(wait, &[RENDERER_TYPE, SONOS_TYPE]).await;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .unwrap_or_default();

    let fetches = locations.into_iter().map(|location| {
        let client = client.clone();
        async move {
            let body = client.get(&location).send().await.ok()?.text().await.ok()?;
            let renderer = parse_description(&body, &location);
            if renderer.is_none() {
                log::debug!("Lecteur réseau ignoré (description sans AVTransport) : {location}");
            }
            renderer
        }
    });
    let mut found: Vec<Renderer> = futures_join_all(fetches).await.into_iter().flatten().collect();
    // Un appareil répond parfois par plusieurs cartes : un seul exemplaire par UDN.
    let mut seen = HashSet::new();
    found.retain(|r| seen.insert(r.id.clone()));
    found.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    found
}

/// Attend toutes les tâches (sans dépendre du crate `futures`).
async fn futures_join_all<F, T>(futs: impl IntoIterator<Item = F>) -> Vec<T>
where
    F: std::future::Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    let handles: Vec<_> = futs.into_iter().map(tokio::spawn).collect();
    let mut out = Vec::with_capacity(handles.len());
    for h in handles {
        if let Ok(v) = h.await {
            out.push(v);
        }
    }
    out
}

/// Envoie la recherche des appareils `types` sur chaque carte et rend les adresses de description reçues.
pub(crate) async fn search(wait: Duration, types: &[&str]) -> Vec<String> {
    let types: Vec<String> = types.iter().map(|t| t.to_string()).collect();
    let requests: Vec<String> = types
        .iter()
        .map(|st| format!("M-SEARCH * HTTP/1.1\r\nHOST: {SSDP_ADDR}:{SSDP_PORT}\r\nMAN: \"ssdp:discover\"\r\nMX: 2\r\nST: {st}\r\n\r\n"))
        .collect();
    let mut sockets = Vec::new();
    for ip in local_ipv4s() {
        match open_socket(ip) {
            Ok(s) => sockets.push(s),
            Err(e) => log::debug!("Recherche réseau impossible depuis {ip} : {e}"),
        }
    }

    let target = SocketAddrV4::new(SSDP_ADDR, SSDP_PORT);
    let deadline = tokio::time::Instant::now() + wait;
    let mut tasks = Vec::new();
    for socket in sockets {
        let (requests, types) = (requests.clone(), types.clone());
        tasks.push(tokio::spawn(async move {
            let mut found = Vec::new();
            // Deux envois : l'UDP multicast se perd volontiers.
            for _ in 0..2 {
                for request in &requests {
                    let _ = socket.send_to(request.as_bytes(), target).await;
                }
                tokio::time::sleep(Duration::from_millis(150)).await;
            }
            let mut buf = vec![0u8; 2048];
            while let Ok(Ok((n, _))) = tokio::time::timeout_at(deadline, socket.recv_from(&mut buf)).await {
                if let Some(location) = parse_search_response(&String::from_utf8_lossy(&buf[..n]), &types) {
                    found.push(location);
                }
            }
            found
        }));
    }

    let mut locations: Vec<String> = Vec::new();
    for t in tasks {
        if let Ok(found) = t.await {
            locations.extend(found);
        }
    }
    locations.sort();
    locations.dedup();
    locations
}

/// Socket UDP liée à une carte, multicast sortant par cette même carte.
pub(super) fn open_socket(ip: Ipv4Addr) -> std::io::Result<UdpSocket> {
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    socket.set_multicast_if_v4(&ip)?;
    socket.set_multicast_ttl_v4(2)?;
    socket.bind(&SocketAddrV4::new(ip, 0).into())?;
    socket.set_nonblocking(true)?;
    UdpSocket::from_std(socket.into())
}

/// `LOCATION` d'une réponse à la recherche, si elle vient d'un appareil cherché.
fn parse_search_response(response: &str, types: &[String]) -> Option<String> {
    let mut lines = response.lines();
    if !lines.next()?.contains("200") {
        return None;
    }
    let mut location = None;
    let mut wanted = false;
    for line in lines {
        let Some((key, value)) = line.split_once(':') else { continue };
        let value = value.trim();
        match key.trim().to_ascii_lowercase().as_str() {
            "location" => location = Some(value.to_string()),
            "st" => wanted = types.iter().any(|t| value.eq_ignore_ascii_case(t)),
            _ => {}
        }
    }
    location.filter(|_| wanted)
}

/// Lit une description d'appareil : le lecteur peut être l'appareil racine ou un sous-appareil.
pub fn parse_description(body: &str, location: &str) -> Option<Renderer> {
    let base = xml::text(body, "URLBase")
        .filter(|b| !b.is_empty())
        .unwrap_or_else(|| origin(location));
    let ip = host_ipv4(location)?;
    xml::elements(body, "device").into_iter().find_map(|device| {
        let r = find_renderer(device, &base, ip)?;
        // Sonos : la pièce plutôt que « Salon - Sonos One Media Renderer », et le service des groupes.
        let root = xml::without(device, "deviceList");
        let room = xml::text(&root, "roomName").filter(|n| !n.is_empty());
        let group_topology = xml::elements(device, "service").into_iter().find_map(|svc| {
            let kind = xml::text(svc, "serviceType")?;
            if !kind.contains(":ZoneGroupTopology:") {
                return None;
            }
            xml::text(svc, "controlURL").map(|u| resolve(&base, &u))
        });
        let fill = |own: String, from_root: &str| if own.is_empty() { xml::text(&root, from_root).unwrap_or_default() } else { own };
        Some(Renderer {
            name: room.unwrap_or(r.name.clone()),
            manufacturer: fill(r.manufacturer.clone(), "manufacturer"),
            model: fill(r.model.clone(), "modelName"),
            location: location.to_string(),
            group_topology,
            ..r
        })
    })
}

/// Relit un lecteur déjà connu depuis l'adresse de sa description.
pub async fn fetch(location: &str) -> Option<Renderer> {
    let client = reqwest::Client::builder().timeout(Duration::from_secs(3)).build().ok()?;
    let body = client.get(location).send().await.ok()?.text().await.ok()?;
    parse_description(&body, location)
}

fn find_renderer(device: &str, base: &str, ip: Ipv4Addr) -> Option<Renderer> {
    // Les champs de CET appareil, sans ceux de ses sous-appareils.
    let own = xml::without(device, "deviceList");
    let is_renderer = xml::text(&own, "deviceType").is_some_and(|t| t.contains(":MediaRenderer:"));
    if is_renderer {
        if let Some(r) = renderer_from(&own, base, ip) {
            return Some(r);
        }
    }
    let children = xml::first(device, "deviceList")?;
    xml::elements(children, "device")
        .into_iter()
        .find_map(|d| find_renderer(d, base, ip))
}

fn renderer_from(own: &str, base: &str, ip: Ipv4Addr) -> Option<Renderer> {
    let mut av_transport = None;
    let mut rendering_control = None;
    let mut connection_manager = None;
    for service in xml::elements(own, "service") {
        let kind = xml::text(service, "serviceType").unwrap_or_default();
        let Some(url) = xml::text(service, "controlURL").map(|u| resolve(base, &u)) else { continue };
        if kind.contains(":AVTransport:") {
            av_transport = Some(url);
        } else if kind.contains(":RenderingControl:") {
            rendering_control = Some(url);
        } else if kind.contains(":ConnectionManager:") {
            connection_manager = Some(url);
        }
    }

    let icon_url = xml::elements(own, "icon")
        .into_iter()
        .filter_map(|icon| {
            let url = xml::text(icon, "url")?;
            let png = xml::text(icon, "mimetype").is_some_and(|m| m.contains("png"));
            let width: u32 = xml::text(icon, "width").and_then(|w| w.parse().ok()).unwrap_or(0);
            Some((png, width, resolve(base, &url)))
        })
        .max_by_key(|(png, width, _)| (*png, *width))
        .map(|(_, _, url)| url);

    Some(Renderer {
        id: xml::text(own, "UDN")?,
        name: xml::text(own, "friendlyName").unwrap_or_else(|| "Lecteur réseau".to_string()),
        manufacturer: xml::text(own, "manufacturer").unwrap_or_default(),
        model: xml::text(own, "modelName").unwrap_or_default(),
        icon_url,
        ip,
        location: String::new(),
        av_transport: av_transport?,
        rendering_control,
        connection_manager,
        group_topology: None,
    })
}

/// `http://hôte:port` d'une adresse complète.
pub(crate) fn origin(url: &str) -> String {
    let after_scheme = url.find("://").map_or(0, |i| i + 3);
    let end = url[after_scheme..].find('/').map_or(url.len(), |i| after_scheme + i);
    url[..end].to_string()
}

fn host_ipv4(url: &str) -> Option<Ipv4Addr> {
    let host = origin(url);
    let host = host.rsplit("://").next()?;
    host.split(':').next()?.parse().ok()
}

/// Adresse absolue d'une URL de description (absolue, depuis la racine ou relative).
pub(crate) fn resolve(base: &str, url: &str) -> String {
    if url.starts_with("http://") || url.starts_with("https://") {
        return url.to_string();
    }
    let root = origin(base);
    if url.starts_with('/') {
        return format!("{root}{url}");
    }
    // Relative : au dossier de la base, ou à la racine si la base n'en a pas.
    let base_path = &base[root.len()..];
    let dir = base_path.rfind('/').map_or("", |i| &base_path[..i]);
    format!("{root}{dir}/{url}")
}

#[cfg(test)]
mod tests {
    use super::*;

    const WIIM: &str = r#"<?xml version="1.0"?>
<root xmlns="urn:schemas-upnp-org:device-1-0" xmlns:dlna="urn:schemas-dlna-org:device-1-0">
 <specVersion><major>1</major><minor>0</minor></specVersion>
 <device>
  <deviceType>urn:schemas-upnp-org:device:MediaRenderer:1</deviceType>
  <friendlyName>WiiM Pro &amp; Salon</friendlyName>
  <manufacturer>Linkplay Technology Inc.</manufacturer>
  <modelName>WiiM Pro</modelName>
  <UDN>uuid:FF31F09E-1A5A-B1D9-1A1B-3C6D5E7F8A9B</UDN>
  <dlna:X_DLNADOC>DMR-1.50</dlna:X_DLNADOC>
  <iconList>
   <icon><mimetype>image/jpeg</mimetype><width>48</width><height>48</height><url>/upnp/grender-48x48.jpg</url></icon>
   <icon><mimetype>image/png</mimetype><width>120</width><height>120</height><url>/upnp/grender-120x120.png</url></icon>
  </iconList>
  <serviceList>
   <service><serviceType>urn:schemas-upnp-org:service:AVTransport:1</serviceType><controlURL>/upnp/control/rendertransport1</controlURL></service>
   <service><serviceType>urn:schemas-upnp-org:service:ConnectionManager:1</serviceType><controlURL>/upnp/control/rendercm1</controlURL></service>
   <service><serviceType>urn:schemas-upnp-org:service:RenderingControl:1</serviceType><controlURL>/upnp/control/rendercontrol1</controlURL></service>
  </serviceList>
 </device>
</root>"#;

    /// Ampli dont le lecteur est un sous-appareil, avec URL relatives et `URLBase`.
    const AMPLI: &str = r#"<root><URLBase>http://192.168.1.40:8888/desc/</URLBase>
 <device>
  <deviceType>urn:schemas-upnp-org:device:Basic:1</deviceType>
  <friendlyName>TX-NR686</friendlyName><UDN>uuid:root-0001</UDN>
  <deviceList>
   <device>
    <deviceType>urn:schemas-upnp-org:device:MediaRenderer:1</deviceType>
    <friendlyName>TX-NR686 Renderer</friendlyName><manufacturer>Onkyo</manufacturer>
    <modelName>TX-NR686</modelName><UDN>uuid:renderer-0002</UDN>
    <serviceList>
     <service><serviceType>urn:schemas-upnp-org:service:AVTransport:1</serviceType><controlURL>AVTransport/ctrl</controlURL></service>
    </serviceList>
   </device>
  </deviceList>
 </device>
</root>"#;

    /// Sonos : appareil racine « ZonePlayer », lecteur en sous-appareil, groupes à la racine.
    const SONOS: &str = r#"<root xmlns="urn:schemas-upnp-org:device-1-0"><device>
  <deviceType>urn:schemas-upnp-org:device:ZonePlayer:1</deviceType>
  <friendlyName>192.168.1.60 - Sonos One - RINCON_48A6B8C1D2E301400</friendlyName>
  <manufacturer>Sonos, Inc.</manufacturer><modelName>Sonos One</modelName>
  <UDN>uuid:RINCON_48A6B8C1D2E301400</UDN><roomName>Salon</roomName>
  <serviceList>
   <service><serviceType>urn:schemas-upnp-org:service:ZoneGroupTopology:1</serviceType><controlURL>/ZoneGroupTopology/Control</controlURL></service>
  </serviceList>
  <deviceList>
   <device><deviceType>urn:schemas-upnp-org:device:MediaServer:1</deviceType><UDN>uuid:RINCON_48A6B8C1D2E301400_MS</UDN>
    <serviceList><service><serviceType>urn:schemas-upnp-org:service:ContentDirectory:1</serviceType><controlURL>/MediaServer/ContentDirectory/Control</controlURL></service></serviceList></device>
   <device><deviceType>urn:schemas-upnp-org:device:MediaRenderer:1</deviceType>
    <friendlyName>Salon - Sonos One Media Renderer</friendlyName><UDN>uuid:RINCON_48A6B8C1D2E301400_MR</UDN>
    <serviceList>
     <service><serviceType>urn:schemas-upnp-org:service:RenderingControl:1</serviceType><controlURL>/MediaRenderer/RenderingControl/Control</controlURL></service>
     <service><serviceType>urn:schemas-upnp-org:service:AVTransport:1</serviceType><controlURL>/MediaRenderer/AVTransport/Control</controlURL></service>
    </serviceList></device>
  </deviceList>
 </device></root>"#;

    #[test]
    fn reads_a_sonos_speaker_by_its_room() {
        let r = parse_description(SONOS, "http://192.168.1.60:1400/xml/device_description.xml").unwrap();
        assert_eq!(r.name, "Salon");
        assert_eq!((r.manufacturer.as_str(), r.model.as_str()), ("Sonos, Inc.", "Sonos One"));
        assert_eq!(r.id, "uuid:RINCON_48A6B8C1D2E301400_MR");
        assert_eq!(r.av_transport, "http://192.168.1.60:1400/MediaRenderer/AVTransport/Control");
        assert_eq!(r.group_topology.as_deref(), Some("http://192.168.1.60:1400/ZoneGroupTopology/Control"));
    }

    #[test]
    fn reads_a_root_renderer() {
        let r = parse_description(WIIM, "http://192.168.1.23:49152/description.xml").unwrap();
        assert_eq!(r.name, "WiiM Pro & Salon");
        assert_eq!(r.model, "WiiM Pro");
        assert_eq!(r.ip, Ipv4Addr::new(192, 168, 1, 23));
        assert_eq!(r.av_transport, "http://192.168.1.23:49152/upnp/control/rendertransport1");
        assert_eq!(r.rendering_control.as_deref(), Some("http://192.168.1.23:49152/upnp/control/rendercontrol1"));
        assert_eq!(r.icon_url.as_deref(), Some("http://192.168.1.23:49152/upnp/grender-120x120.png"));
    }

    #[test]
    fn finds_a_nested_renderer_with_relative_urls() {
        let r = parse_description(AMPLI, "http://192.168.1.40:8888/desc/device.xml").unwrap();
        assert_eq!(r.id, "uuid:renderer-0002");
        assert_eq!(r.manufacturer, "Onkyo");
        assert_eq!(r.av_transport, "http://192.168.1.40:8888/desc/AVTransport/ctrl");
        assert_eq!(r.rendering_control, None);
    }

    /// Vraie recherche sur le réseau local : `cargo test discovers_here -- --ignored --nocapture`.
    #[tokio::test]
    #[ignore]
    async fn discovers_here() {
        for r in discover(Duration::from_secs(3)).await {
            eprintln!("{} — {} {} — {} — {}", r.name, r.manufacturer, r.model, r.ip, r.av_transport);
        }
    }

    #[test]
    fn keeps_only_renderer_search_answers() {
        let types = [RENDERER_TYPE.to_string(), SONOS_TYPE.to_string()];
        let ok ="HTTP/1.1 200 OK\r\nCACHE-CONTROL: max-age=1800\r\nLocation: http://10.0.0.5:49152/d.xml\r\nST: urn:schemas-upnp-org:device:MediaRenderer:1\r\n\r\n";
        assert_eq!(parse_search_response(ok, &types).as_deref(), Some("http://10.0.0.5:49152/d.xml"));
        let other = ok.replace("MediaRenderer", "MediaServer");
        assert_eq!(parse_search_response(&other, &types), None);
        let sonos = ok.replace("device:MediaRenderer:1", "device:ZonePlayer:1");
        assert!(parse_search_response(&sonos, &types).is_some());
    }
}
