//! Runtime configuration of the DLNA server.

use std::net::IpAddr;

#[derive(Debug, Clone)]
pub struct DlnaConfig {
    /// TCP port for the HTTP service.
    pub port: u16,

    /// User-visible name advertised in SSDP and shown in DLNA clients.
    /// e.g. `"RustMusic on DESKTOP-1234"`.
    pub friendly_name: String,

    /// Stable UUID v4 for this server instance (advertised as USN).
    /// Persisted in settings so the same value is used across restarts —
    /// otherwise clients keep re-discovering us as a "new" server.
    pub uuid: String,

    /// Local interface to bind to. `0.0.0.0` (default) listens on all interfaces.
    pub bind_address: IpAddr,
}

impl Default for DlnaConfig {
    fn default() -> Self {
        Self {
            port: 8200,
            friendly_name: "RustMusic Media Server".to_string(),
            uuid: uuid::Uuid::new_v4().to_string(),
            bind_address: IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED),
        }
    }
}
