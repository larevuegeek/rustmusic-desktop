//! Commandes du lecteur réseau : recherche, choix et restauration au lancement.
//! Deux familles d'appareils : UPnP/DLNA (l'appareil lit nos fichiers) et AirPlay (on lui envoie le son).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use tauri::State;

use crate::core::network_player::airplay::device::{self as airplay_device, AirplayDevice};
use crate::core::network_player::airplay::sender as airplay;
use crate::core::network_player::discovery::{self, Renderer};
use crate::core::network_player::session;
use crate::repository::settings::settings_repository::SettingsRepository;
use crate::state::AppState;

const ID_KEY: &str = "network_player_id";
const LOCATION_KEY: &str = "network_player_location";
/// Réglages « Lecteurs UPnP/DLNA » et « AirPlay » : désactivés par défaut, rien ne part sur le réseau.
const UPNP_KEY: &str = "network_players";
const AIRPLAY_KEY: &str = "airplay";
const SEARCH: Duration = Duration::from_secs(3);

static UPNP_ENABLED: AtomicBool = AtomicBool::new(false);
static AIRPLAY_ENABLED: AtomicBool = AtomicBool::new(false);

fn upnp_enabled() -> bool {
    UPNP_ENABLED.load(Ordering::Relaxed)
}

fn airplay_enabled() -> bool {
    AIRPLAY_ENABLED.load(Ordering::Relaxed)
}

#[derive(Debug, Clone)]
enum Device {
    Upnp(Renderer),
    Airplay(AirplayDevice),
}

/// Appareil tel que le voit l'interface.
#[derive(Debug, Clone, Serialize)]
pub struct DeviceView {
    pub id: String,
    pub name: String,
    pub manufacturer: String,
    pub model: String,
    pub icon_url: Option<String>,
    pub ip: String,
    /// « upnp » ou « airplay ».
    pub kind: &'static str,
}

impl Device {
    /// Sa famille est-elle activée dans les réglages ?
    fn allowed(&self) -> bool {
        match self {
            Device::Upnp(_) => upnp_enabled(),
            Device::Airplay(_) => airplay_enabled(),
        }
    }

    fn id(&self) -> &str {
        match self {
            Device::Upnp(r) => &r.id,
            Device::Airplay(d) => &d.id,
        }
    }

    fn view(&self) -> DeviceView {
        match self {
            Device::Upnp(r) => DeviceView {
                id: r.id.clone(),
                name: r.name.clone(),
                manufacturer: r.manufacturer.clone(),
                model: r.model.clone(),
                icon_url: r.icon_url.clone(),
                ip: r.ip.to_string(),
                kind: "upnp",
            },
            Device::Airplay(d) => DeviceView {
                id: d.id.clone(),
                name: d.name.clone(),
                manufacturer: String::new(),
                model: d.model.clone(),
                icon_url: None,
                ip: d.ip.to_string(),
                kind: "airplay",
            },
        }
    }
}

/// Derniers appareils trouvés : le choix se fait parmi eux.
static FOUND: Mutex<Vec<Device>> = Mutex::new(Vec::new());

fn found() -> std::sync::MutexGuard<'static, Vec<Device>> {
    FOUND.lock().unwrap_or_else(|e| e.into_inner())
}

fn current() -> Option<Device> {
    session::selected().map(Device::Upnp).or_else(|| airplay::selected().map(Device::Airplay))
}

/// Cherche les lecteurs réseau des familles activées, en même temps (~3 s).
#[tauri::command]
pub async fn network_players_discover() -> Vec<DeviceView> {
    if !upnp_enabled() && !airplay_enabled() {
        return Vec::new();
    }
    let upnp = async { if upnp_enabled() { discovery::discover(SEARCH).await } else { Vec::new() } };
    let air = async { if airplay_enabled() { airplay_device::discover(SEARCH).await } else { Vec::new() } };
    let (upnp, air) = tokio::join!(upnp, air);
    log::info!("📡 {} lecteur(s) UPnP et {} appareil(s) AirPlay trouvé(s)", upnp.len(), air.len());
    let mut list: Vec<Device> = upnp.into_iter().map(Device::Upnp).chain(air.into_iter().map(Device::Airplay)).collect();
    // L'appareil choisi reste proposé même s'il n'a pas répondu cette fois.
    if let Some(sel) = current() {
        if !list.iter().any(|d| d.id() == sel.id()) {
            list.push(sel);
        }
    }
    list.sort_by_key(|d| d.view().name.to_lowercase());
    let views = list.iter().map(Device::view).collect();
    *found() = list;
    views
}

/// Le lecteur réseau choisi, ou `None` pour la sortie locale.
#[tauri::command]
pub fn network_player_selected() -> Option<DeviceView> {
    current().map(|d| d.view())
}

/// Choisit un lecteur (`None` : retour à la sortie locale) et le mémorise.
#[tauri::command]
pub async fn network_player_select(state: State<'_, AppState>, id: Option<String>) -> Result<Option<DeviceView>, String> {
    let device = match &id {
        Some(id) => Some(found().iter().find(|d| d.id() == id).cloned().ok_or("lecteur inconnu : relancez la recherche")?),
        None => None,
    };
    if device.as_ref().is_some_and(|d| !d.allowed()) {
        return Err("lecteurs réseau désactivés dans les réglages".into());
    }

    // Une seule sortie joue : ce qui joue s'arrête avant le changement.
    crate::commands::playback::stop_all().await;
    apply(device.clone()).await;

    let (id, location) = match &device {
        Some(Device::Upnp(r)) => (r.id.clone(), r.location.clone()),
        Some(Device::Airplay(d)) => (d.id.clone(), String::new()),
        None => (String::new(), String::new()),
    };
    SettingsRepository::set(&state.pool, ID_KEY, &id).await.map_err(|e| e.to_string())?;
    SettingsRepository::set(&state.pool, LOCATION_KEY, &location).await.map_err(|e| e.to_string())?;
    match &device {
        Some(d) => log::info!("📡 Sortie : {} ({}, {})", d.view().name, d.view().kind, d.view().ip),
        None => log::info!("📡 Sortie : retour au lecteur local"),
    }
    Ok(device.map(|d| d.view()))
}

/// Rend `device` actif, l'autre famille inactive.
async fn apply(device: Option<Device>) {
    let (upnp, air) = match device {
        Some(Device::Upnp(r)) => (Some(r), None),
        Some(Device::Airplay(d)) => (None, Some(d)),
        None => (None, None),
    };
    session::select(upnp).await;
    // Fermer une session AirPlay attend la fin de son fil : hors du moteur async.
    let _ = tokio::task::spawn_blocking(move || airplay::select(air)).await;
}

/// Active ou désactive les lecteurs UPnP/DLNA. Activer ne rebascule pas le son d'office :
/// l'utilisateur choisit l'appareil. Désactiver revient en local, le lecteur mémorisé
/// est gardé et retrouvé au prochain lancement.
#[tauri::command]
pub async fn set_network_players_enabled(enabled: bool) -> Result<(), String> {
    UPNP_ENABLED.store(enabled, Ordering::Relaxed);
    drop_disallowed().await;
    Ok(())
}

/// Même chose pour AirPlay.
#[tauri::command]
pub async fn set_airplay_enabled(enabled: bool) -> Result<(), String> {
    AIRPLAY_ENABLED.store(enabled, Ordering::Relaxed);
    drop_disallowed().await;
    Ok(())
}

/// L'appareil en cours appartient à une famille désactivée : retour au lecteur local.
async fn drop_disallowed() {
    found().retain(Device::allowed);
    if current().is_some_and(|d| !d.allowed()) {
        crate::commands::playback::stop_all().await;
        apply(None).await;
        log::info!("📡 Famille de lecteurs désactivée : retour au lecteur local");
    }
}

/// Au lancement : lit le réglage, puis retrouve le lecteur mémorisé. Absent, on reste en local sans l'oublier.
pub async fn init(state: &AppState) {
    let on = |key| async move { SettingsRepository::get(&state.pool, key).await.ok().flatten().as_deref() == Some("true") };
    UPNP_ENABLED.store(on(UPNP_KEY).await, Ordering::Relaxed);
    AIRPLAY_ENABLED.store(on(AIRPLAY_KEY).await, Ordering::Relaxed);
    if upnp_enabled() || airplay_enabled() {
        restore(state).await;
    }
}

async fn restore(state: &AppState) {
    let id = SettingsRepository::get(&state.pool, ID_KEY).await.ok().flatten().unwrap_or_default();
    let location = SettingsRepository::get(&state.pool, LOCATION_KEY).await.ok().flatten().unwrap_or_default();
    if id.is_empty() {
        return;
    }
    let device = if id.starts_with("airplay:") {
        if !airplay_enabled() {
            return;
        }
        airplay_device::discover(SEARCH).await.into_iter().find(|d| d.id == id).map(Device::Airplay)
    } else if location.is_empty() || !upnp_enabled() {
        None
    } else {
        discovery::fetch(&location).await.filter(|r| r.id == id).map(Device::Upnp)
    };
    match device {
        Some(d) => {
            log::info!("📡 Lecteur réseau retrouvé : {}", d.view().name);
            found().push(d.clone());
            apply(Some(d)).await;
        }
        None => log::info!("📡 Lecteur réseau mémorisé injoignable : sortie locale"),
    }
}
