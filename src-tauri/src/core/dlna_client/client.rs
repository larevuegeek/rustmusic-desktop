//! Client DLNA (UPnP ContentDirectory) : description du serveur, recherche sur le réseau, parcours.
//! Pas d'identifiants : un serveur DLNA est ouvert à tout le réseau local.

use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use serde::Serialize;

use crate::core::network_player::control::call;
use crate::core::network_player::discovery::{self, origin, resolve};
use crate::core::podcast::fetch::client;
use crate::core::subsonic::client::ServerInfo;
use crate::helper::xml;

pub const MEDIA_SERVER: &str = "urn:schemas-upnp-org:device:MediaServer:1";
const CONTENT_DIRECTORY: &str = "urn:schemas-upnp-org:service:ContentDirectory:1";
/// Descriptions essayées quand l'adresse saisie n'a pas de chemin (MiniDLNA, puis les plus courantes).
const DESCRIPTIONS: [&str; 3] = ["/rootDesc.xml", "/description.xml", "/DeviceDescription.xml"];

/// Un serveur joint par l'adresse de sa description.
#[derive(Debug, Clone, PartialEq)]
pub struct Server {
    pub location: String,
}

/// Ce que dit la description d'un serveur.
#[derive(Debug, Clone, PartialEq)]
pub struct Description {
    pub udn: String,
    pub name: String,
    pub model: String,
    pub version: Option<String>,
    /// Service ContentDirectory, adresse ramenée à celle de la description.
    pub control: String,
}

/// Un serveur trouvé sur le réseau.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Found {
    pub id: String,
    pub name: String,
    pub model: String,
    pub location: String,
}

/// Service ContentDirectory déjà lu, par adresse de description.
fn controls() -> &'static Mutex<HashMap<String, String>> {
    static CONTROLS: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    CONTROLS.get_or_init(Default::default)
}

/// Adresse saisie → descriptions à essayer. `localhost` devient `127.0.0.1` : MiniDLNA refuse
/// un nom d'hôte (protection contre le « DNS rebinding »).
fn candidates(raw: &str) -> Vec<String> {
    let mut s = raw.trim().trim_end_matches('/').to_string();
    if !s.contains("://") {
        s = format!("http://{s}");
    }
    let s = s.replacen("://localhost", "://127.0.0.1", 1);
    let root = origin(&s);
    if s.len() > root.len() {
        vec![s]
    } else {
        DESCRIPTIONS.iter().map(|p| format!("{root}{p}")).collect()
    }
}

/// Une adresse donnée par le serveur, ramenée à celle par laquelle on le joint : dans Docker,
/// il annonce son adresse interne (même port, autre hôte), injoignable depuis l'ordinateur.
pub(crate) fn rebase(location: &str, url: &str) -> String {
    let (Ok(base), Ok(mut u)) = (reqwest::Url::parse(location), reqwest::Url::parse(url)) else { return url.to_string() };
    if u.port_or_known_default() == base.port_or_known_default() && u.host_str() != base.host_str() {
        if let Some(host) = base.host_str() {
            if u.set_host(Some(host)).is_ok() {
                return u.to_string();
            }
        }
    }
    url.to_string()
}

/// Lit une description de serveur de contenu (appareil racine ou sous-appareil).
pub fn parse_description(body: &str, location: &str) -> Option<Description> {
    let base = xml::text(body, "URLBase").filter(|b| !b.is_empty()).unwrap_or_else(|| origin(location));
    let control = xml::elements(body, "service").into_iter().find_map(|svc| {
        let kind = xml::text(svc, "serviceType")?;
        kind.contains(":ContentDirectory:").then(|| xml::text(svc, "controlURL")).flatten()
    })?;
    Some(Description {
        udn: xml::text(body, "UDN").unwrap_or_default(),
        name: xml::text(body, "friendlyName").filter(|n| !n.is_empty()).unwrap_or_else(|| "Serveur DLNA".into()),
        model: xml::text(body, "modelName").unwrap_or_default(),
        version: xml::text(body, "modelNumber").filter(|v| !v.is_empty()),
        control: rebase(location, &resolve(&base, &control)),
    })
}

async fn describe(location: &str) -> Result<Description, String> {
    let resp = client().get(location).timeout(Duration::from_secs(8)).send().await.map_err(|e| format!("serveur injoignable : {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("serveur : HTTP {}", resp.status().as_u16()));
    }
    let body = resp.text().await.map_err(|e| format!("serveur injoignable : {e}"))?;
    let d = parse_description(&body, location).ok_or("ce n'est pas un serveur DLNA")?;
    controls().lock().unwrap_or_else(|e| e.into_inner()).insert(location.to_string(), d.control.clone());
    Ok(d)
}

fn info_of(d: &Description) -> ServerInfo {
    ServerInfo {
        kind: Some("DLNA".into()),
        version: d.version.clone(),
        name: Some(d.name.clone()),
        device_id: Some(d.udn.clone()).filter(|u| !u.is_empty()),
        ..Default::default()
    }
}

/// Serveurs muets retrouvés ailleurs sur le réseau, par leur UDN : `(id, nouvelle adresse)`.
pub fn relocations(lost: &[(i64, String)], found: &[Found]) -> Vec<(i64, String)> {
    lost.iter()
        .filter_map(|(id, udn)| found.iter().find(|f| f.id.eq_ignore_ascii_case(udn)).map(|f| (*id, f.location.clone())))
        .collect()
}

/// Les serveurs DLNA du réseau ; `own` : l'UDN du serveur de RustMusic, écarté.
pub async fn discover(wait: Duration, own: Option<&str>) -> Vec<Found> {
    let locations = discovery::search(wait, &[MEDIA_SERVER]).await;
    let mut found = Vec::new();
    let mut seen = HashSet::new();
    for location in locations {
        let Ok(d) = describe(&location).await else { continue };
        if own.is_some_and(|o| d.udn.eq_ignore_ascii_case(o)) || !seen.insert(d.udn.clone()) {
            continue;
        }
        found.push(Found { id: d.udn, name: d.name, model: d.model, location });
    }
    found.sort_by_key(|f| f.name.to_lowercase());
    found
}

impl Server {
    /// Joint le serveur depuis l'adresse saisie (description, ou simple `hôte:port`).
    pub async fn connect(raw: &str) -> Result<(Server, ServerInfo), String> {
        if raw.trim().is_empty() {
            return Err("adresse du serveur vide".into());
        }
        let mut last = String::new();
        for location in candidates(raw) {
            match describe(&location).await {
                Ok(d) => return Ok((Server { location }, info_of(&d))),
                Err(e) => last = e,
            }
        }
        Err(last)
    }

    pub async fn ping(&self) -> Result<ServerInfo, String> {
        describe(&self.location).await.map(|d| info_of(&d))
    }

    async fn control(&self) -> Result<String, String> {
        if let Some(c) = controls().lock().unwrap_or_else(|e| e.into_inner()).get(&self.location).cloned() {
            return Ok(c);
        }
        describe(&self.location).await.map(|d| d.control)
    }

    pub(crate) fn rebase(&self, url: &str) -> String {
        rebase(&self.location, url)
    }

    /// Une page d'un dossier (`children`) ou la fiche d'un objet : DIDL, nombre rendu, total annoncé.
    pub(crate) async fn browse(&self, id: &str, children: bool, start: u32, count: u32) -> Result<(String, u32, u32), String> {
        let control = self.control().await?;
        let (start, count) = (start.to_string(), count.to_string());
        let flag = if children { "BrowseDirectChildren" } else { "BrowseMetadata" };
        let args = [("ObjectID", id), ("BrowseFlag", flag), ("Filter", "*"), ("StartingIndex", &start), ("RequestedCount", &count), ("SortCriteria", "")];
        let body = call(client(), &control, CONTENT_DIRECTORY, "Browse", &args).await.map_err(|e| {
            if e.contains("error sending request") { format!("serveur injoignable : {e}") } else { e }
        })?;
        let num = |k: &str| xml::text(&body, k).and_then(|v| v.parse().ok()).unwrap_or(0);
        Ok((xml::text(&body, "Result").unwrap_or_default(), num("NumberReturned"), num("TotalMatches")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIDLNA: &str = r#"<?xml version="1.0"?><root xmlns="urn:schemas-upnp-org:device-1-0"><device>
<deviceType>urn:schemas-upnp-org:device:MediaServer:1</deviceType><friendlyName>MiniDLNA Docker</friendlyName>
<manufacturer>Justin Maggard</manufacturer><modelName>Windows Media Connect compatible (MiniDLNA)</modelName><modelNumber>1.3.3</modelNumber>
<UDN>uuid:4d696e69-444c-164e-9d41-0242ac150002</UDN><serviceList>
<service><serviceType>urn:schemas-upnp-org:service:ContentDirectory:1</serviceType><controlURL>/ctl/ContentDir</controlURL></service>
<service><serviceType>urn:schemas-upnp-org:service:ConnectionManager:1</serviceType><controlURL>/ctl/ConnectionMgr</controlURL></service>
</serviceList></device></root>"#;

    #[test]
    fn reads_a_server_description() {
        let d = parse_description(MINIDLNA, "http://127.0.0.1:8201/rootDesc.xml").unwrap();
        assert_eq!((d.name.as_str(), d.version.as_deref()), ("MiniDLNA Docker", Some("1.3.3")));
        assert_eq!(d.control, "http://127.0.0.1:8201/ctl/ContentDir");
        assert!(d.udn.starts_with("uuid:4d69"));
        assert!(parse_description("<root><device><friendlyName>Lecteur</friendlyName></device></root>", "http://x/").is_none());
    }

    #[test]
    fn a_moved_server_is_found_by_its_udn() {
        let found = vec![Found { id: "uuid:ABC".into(), name: "NAS".into(), model: String::new(), location: "http://10.0.0.42:8200/rootDesc.xml".into() }];
        let lost = vec![(3, "uuid:abc".to_string()), (4, "uuid:zzz".to_string())];
        assert_eq!(relocations(&lost, &found), vec![(3, "http://10.0.0.42:8200/rootDesc.xml".to_string())]);
    }

    #[test]
    fn entered_addresses() {
        assert_eq!(candidates("localhost:8201")[0], "http://127.0.0.1:8201/rootDesc.xml");
        assert_eq!(candidates("http://nas:8096/dlna/abc/description.xml"), vec!["http://nas:8096/dlna/abc/description.xml"]);
    }

    // Essai réel : les serveurs DLNA du réseau local (lecture seule). Le pare-feu Windows bloque
    // les réponses vers l'exécutable de test : à lancer depuis l'appli, ou pare-feu ouvert.
    #[tokio::test]
    #[ignore = "réseau"]
    async fn real_discovery() {
        for f in discover(Duration::from_secs(3), None).await {
            println!("{} — {} ({})", f.name, f.model, f.location);
        }
    }

    #[test]
    fn docker_addresses_are_rebased() {
        let loc = "http://127.0.0.1:8201/rootDesc.xml";
        assert_eq!(rebase(loc, "http://172.21.0.2:8201/MediaItems/22.flac"), "http://127.0.0.1:8201/MediaItems/22.flac");
        // Autre port : une autre machine, laissée telle quelle.
        assert_eq!(rebase(loc, "http://10.0.0.9:80/cover.jpg"), "http://10.0.0.9:80/cover.jpg");
    }
}
