//! Appareils AirPlay trouvés par Bonjour, et ce qu'ils acceptent.

use std::net::Ipv4Addr;
use std::time::Duration;

use super::mdns::{self, Service};

const RAOP_SERVICE: &str = "_raop._tcp.local";

/// Chiffrement du flux audio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encryption {
    None,
    /// AES-128 dont la clé est chiffrée par la clé publique RSA des AirPort.
    Rsa,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AirplayDevice {
    /// « airplay:<instance> » : l'instance commence par l'adresse MAC, stable.
    pub id: String,
    pub name: String,
    pub model: String,
    pub ip: Ipv4Addr,
    pub port: u16,
    pub encryption: Encryption,
    /// Ce que l'appareil sait afficher (champ `md`) : titre/artiste, pochette, progression.
    pub shows: Shows,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Shows {
    pub text: bool,
    pub artwork: bool,
    pub progress: bool,
}

impl Shows {
    /// `md=0,1,2` ; absent, on tente le texte seul.
    fn parse(md: Option<&str>) -> Self {
        let Some(md) = md else { return Self { text: true, ..Self::default() } };
        let has = |v: &str| md.split(',').any(|x| x.trim() == v);
        Self { text: has("0"), artwork: has("1"), progress: has("2") }
    }
}

/// Appareils AirPlay qui répondent pendant `wait`, triés par nom.
pub async fn discover(wait: Duration) -> Vec<AirplayDevice> {
    let mut found: Vec<AirplayDevice> = mdns::browse(RAOP_SERVICE, wait)
        .await
        .iter()
        .filter_map(|s| match from_service(s) {
            Ok(d) => Some(d),
            Err(why) => {
                log::info!("📡 AirPlay ignoré ({}) : {why}", s.instance);
                None
            }
        })
        .collect();
    found.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    found.dedup_by(|a, b| a.id == b.id);
    found
}

/// Retient l'appareil s'il parle notre AirPlay : UDP, ALAC, sans mot de passe.
fn from_service(s: &Service) -> Result<AirplayDevice, &'static str> {
    let ip = s.ip.ok_or("adresse inconnue")?;
    if s.txt("pw").is_some_and(|v| v.eq_ignore_ascii_case("true")) {
        return Err("protégé par mot de passe");
    }
    let has = |key: &str, value: &str| s.txt(key).map_or(true, |list| list.split(',').any(|v| v.trim() == value));
    if !has("tp", "UDP") {
        return Err("transport UDP absent");
    }
    if !has("cn", "1") {
        return Err("ALAC non accepté");
    }
    // Sans chiffrement si possible : plus simple, accepté par la plupart des appareils.
    let encryption = if has("et", "0") {
        Encryption::None
    } else if has("et", "1") {
        Encryption::Rsa
    } else {
        return Err("chiffrement non pris en charge");
    };
    // « 0011223344@Salon » : le nom suit l'adresse MAC.
    let name = s.instance.split_once('@').map_or(s.instance.as_str(), |(_, n)| n).to_string();
    Ok(AirplayDevice {
        id: format!("airplay:{}", s.instance),
        name,
        model: s.txt("am").unwrap_or_default().to_string(),
        ip,
        port: s.port,
        encryption,
        shows: Shows::parse(s.txt("md")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn service(txt: &[(&str, &str)]) -> Service {
        Service {
            instance: "0011223344@Onkyo TX-RZ900".into(),
            port: 7000,
            ip: Some(Ipv4Addr::new(192, 168, 1, 30)),
            txt: txt.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
        }
    }

    #[test]
    fn keeps_what_we_can_play() {
        let d = from_service(&service(&[("et", "0,3,5"), ("cn", "0,1,2,3"), ("tp", "UDP"), ("am", "TX-RZ900")])).unwrap();
        assert_eq!(d.name, "Onkyo TX-RZ900");
        assert_eq!(d.id, "airplay:0011223344@Onkyo TX-RZ900");
        assert_eq!(d.encryption, Encryption::None);
        assert_eq!(d.model, "TX-RZ900");
        assert_eq!(d.shows, Shows { text: true, artwork: false, progress: false });
        let onkyo = from_service(&service(&[("md", "0,1,2")])).unwrap();
        assert_eq!(onkyo.shows, Shows { text: true, artwork: true, progress: true });
        // AirPort Express d'origine : RSA seulement.
        assert_eq!(from_service(&service(&[("et", "1"), ("cn", "0,1")])).unwrap().encryption, Encryption::Rsa);
    }

    #[test]
    fn rejects_what_we_cannot() {
        assert!(from_service(&service(&[("pw", "true")])).is_err());
        assert!(from_service(&service(&[("et", "3,5")])).is_err());
        assert!(from_service(&service(&[("cn", "2,3")])).is_err());
        assert!(from_service(&service(&[("tp", "TCP")])).is_err());
    }
}

#[cfg(test)]
mod live {
    /// Recherche réelle sur le réseau local : `cargo test --lib airplay_here -- --ignored --nocapture`.
    #[tokio::test]
    #[ignore]
    async fn airplay_here() {
        for s in super::mdns::browse(super::RAOP_SERVICE, std::time::Duration::from_secs(3)).await {
            println!("{} {:?}:{} {:?}", s.instance, s.ip, s.port, s.txt);
        }
        println!("retenus : {:#?}", super::discover(std::time::Duration::from_secs(2)).await);
    }
}
