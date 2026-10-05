//! Réservation exclusive d'une carte audio via `org.freedesktop.ReserveDevice1`
//! (D-Bus) — Linux uniquement.
//!
//! # Pourquoi
//! Pour du DoP bit-perfect on ouvre le `hw:` ALSA en direct, mais PipeWire/Pulse
//! tient déjà la carte → ouverture impossible (EBUSY). Le protocole standard
//! `org.freedesktop.ReserveDevice1` (utilisé par JACK, Roon, PulseAudio…) permet
//! de **demander poliment** au serveur son de relâcher la carte : on acquiert le
//! nom de bus `org.freedesktop.ReserveDevice1.Audio<N>`, PipeWire reçoit
//! `RequestRelease`, ferme le device ALSA, et nous cède le nom. À la fin, on
//! relâche le nom → PipeWire reprend la carte.
//!
//! Tant qu'une instance de [`DeviceReservation`] vit, la carte est à nous.
//!
//! # PipeWire d'abord
//! Quand PipeWire répond, on ne passe pas par D-Bus : on lui fait mettre la
//! carte hors service (profil « off », voir [`super::profil_pipewire`]). La
//! réservation D-Bus fait recréer la carte à WirePlumber 0.4, qui la renomme et
//! perd la sortie par défaut. Elle reste le repli des systèmes sans PipeWire.
//!
//! # Le détenteur doit répondre
//! Le protocole impose au détenteur du nom d'exposer l'objet
//! `/org/freedesktop/ReserveDevice1/Audio<N>` (méthode `RequestRelease`,
//! propriétés `Priority`, `ApplicationName`, `ApplicationDeviceName`).
//! WirePlumber, dès qu'il a cédé la carte, nous envoie lui-même un
//! `RequestRelease` pour la récupérer. Sans objet pour lui répondre, son appel
//! échouait, et il ne redemandait plus jamais le nom une fois rendu : après une
//! lecture DSD, la carte disparaissait de PipeWire jusqu'au redémarrage.

#![cfg(target_os = "linux")]

use std::time::Duration;

use zbus::blocking::{Connection, Proxy};
use zbus::fdo::{RequestNameFlags, RequestNameReply};
use zbus::interface;

/// Priorité de réservation passée à `RequestRelease`. Le propriétaire actuel
/// (PipeWire) cède si notre priorité est ≥ la sienne. Valeur haute : on veut
/// vraiment la carte (l'utilisateur a explicitement demandé le DoP bit-perfect).
const RESERVE_PRIORITY: i32 = 1_000_000;

/// L'objet que le protocole demande au détenteur du nom.
struct Detenteur {
    card_index: u32,
}

#[interface(name = "org.freedesktop.ReserveDevice1")]
impl Detenteur {
    /// Un autre programme veut la carte. On la garde : la lecture DSD tourne, et
    /// la rendre en plein morceau n'est pas prévu. WirePlumber, dont la priorité
    /// est bien plus basse, attend alors qu'on rende le nom pour le reprendre.
    fn request_release(&self, _priority: i32) -> bool {
        false
    }

    #[zbus(property)]
    fn priority(&self) -> i32 {
        RESERVE_PRIORITY
    }

    #[zbus(property)]
    fn application_name(&self) -> String {
        "RustMusic".into()
    }

    #[zbus(property)]
    fn application_device_name(&self) -> String {
        format!("hw:{}", self.card_index)
    }
}

/// Réservation vivante d'une carte ALSA. Le `Drop` relâche automatiquement.
pub struct DeviceReservation {
    card_index: u32,
    mode: Mode,
}

enum Mode {
    /// PipeWire a mis la carte hors service à notre demande.
    Profil,
    /// Nom D-Bus `ReserveDevice1` détenu (serveur son sans PipeWire).
    DBus { conn: Connection, name: String },
}

impl DeviceReservation {
    /// Tente de réserver la carte ALSA `card_index` (bypass PipeWire/Pulse).
    ///
    /// Retourne `Ok` si la carte est à nous (PipeWire l'a relâchée), `Err`
    /// sinon (pas de session D-Bus, propriétaire qui refuse, etc.) → l'appelant
    /// retombe alors sur le chemin PCM classique.
    pub fn acquire(card_index: u32) -> Result<Self, String> {
        match super::profil_pipewire::acquire(card_index) {
            Ok(()) => return Ok(Self { card_index, mode: Mode::Profil }),
            Err(e) => log::debug!("🔒 PipeWire indisponible pour la carte {card_index} ({e}) → réservation D-Bus"),
        }
        Self::acquire_dbus(card_index)
    }

    fn acquire_dbus(card_index: u32) -> Result<Self, String> {
        let conn = Connection::session().map_err(|e| format!("session D-Bus: {e}"))?;
        let name = format!("org.freedesktop.ReserveDevice1.Audio{card_index}");
        let path = format!("/org/freedesktop/ReserveDevice1/Audio{card_index}");

        // L'objet d'abord : WirePlumber l'interroge dès qu'on détient le nom.
        conn.object_server()
            .at(path.as_str(), Detenteur { card_index })
            .map_err(|e| format!("objet ReserveDevice1: {e}"))?;

        let base = RequestNameFlags::AllowReplacement | RequestNameFlags::DoNotQueue;

        // 1er essai (sans REPLACE) : réussit direct si personne ne tient la carte.
        // NB : zbus mappe la réponse `Exists` en `Err(Error::NameTaken)` — donc
        // "occupée" arrive sous forme d'erreur, pas de réponse.
        match conn.request_name_with_flags(name.as_str(), base) {
            Ok(RequestNameReply::PrimaryOwner) | Ok(RequestNameReply::AlreadyOwner) => {
                log::info!("🔒 Carte audio {card_index} réservée (D-Bus, libre)");
                return Ok(Self { card_index, mode: Mode::DBus { conn, name } });
            }
            // `Exists` remonte sous forme d'erreur `NameTaken` chez zbus.
            Ok(_) | Err(zbus::Error::NameTaken) => {
                log::debug!("🔒 Carte {card_index} tenue par le serveur son → négociation release");
            }
            Err(e) => return Err(format!("request_name: {e}")),
        }

        // Occupée : demander la libération au propriétaire actuel (PipeWire).
        // Chaînes possédées (String) → le Proxy n'emprunte pas `name`, ce qui
        // permet de le déplacer ensuite dans `Self`.
        let proxy = Proxy::new(
            &conn,
            name.clone(),
            path,
            "org.freedesktop.ReserveDevice1",
        )
        .map_err(|e| format!("proxy ReserveDevice1: {e}"))?;

        let released: bool = proxy
            .call("RequestRelease", &(RESERVE_PRIORITY,))
            .map_err(|e| format!("RequestRelease: {e}"))?;
        log::debug!("🔒 RequestRelease(prio={RESERVE_PRIORITY}) → released={released}");
        if !released {
            return Err("le serveur son refuse de libérer la carte".into());
        }

        // Le propriétaire a cédé le nom → on le prend (REPLACE au cas où).
        let flags = base | RequestNameFlags::ReplaceExisting;
        let reply2 = conn
            .request_name_with_flags(name.as_str(), flags)
            .map_err(|e| format!("request_name (2e): {e}"))?;
        match reply2 {
            RequestNameReply::PrimaryOwner | RequestNameReply::AlreadyOwner => {
                // Laisser PipeWire finir de fermer le device ALSA.
                std::thread::sleep(Duration::from_millis(200));
                log::info!("🔒 Carte audio {card_index} réservée (D-Bus, après release PipeWire)");
                Ok(Self { card_index, mode: Mode::DBus { conn, name } })
            }
            other => Err(format!("réservation refusée après release: {other}")),
        }
    }
}

impl Drop for DeviceReservation {
    fn drop(&mut self) {
        match &self.mode {
            Mode::Profil => super::profil_pipewire::release(self.card_index),
            Mode::DBus { conn, name } => {
                let _ = conn.release_name(name.as_str());
                log::info!("🔓 Carte audio {} relâchée (D-Bus)", self.card_index);
            }
        }
    }
}

/// Extrait l'index de carte d'un identifiant ALSA `hw:N,0` → `N`.
pub fn card_index_from_hw_id(hw_id: &str) -> Option<u32> {
    hw_id
        .strip_prefix("hw:")?
        .split(',')
        .next()?
        .parse()
        .ok()
}
