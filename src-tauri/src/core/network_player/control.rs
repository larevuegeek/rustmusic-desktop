//! Commandes envoyées au lecteur réseau (SOAP) : transport, volume, formats acceptés.

use std::time::Duration;

use super::discovery::Renderer;
use crate::helper::xml;

const AV_TRANSPORT: &str = "urn:schemas-upnp-org:service:AVTransport:1";
const RENDERING_CONTROL: &str = "urn:schemas-upnp-org:service:RenderingControl:1";
const CONNECTION_MANAGER: &str = "urn:schemas-upnp-org:service:ConnectionManager:1";

/// État du transport tel que l'annonce le lecteur.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportState {
    Playing,
    Paused,
    Stopped,
    /// Chargement ou passage d'une piste à l'autre.
    Transitioning,
    NoMedia,
    Unknown,
}

impl TransportState {
    fn parse(s: &str) -> Self {
        match s.trim() {
            "PLAYING" => Self::Playing,
            "PAUSED_PLAYBACK" | "PAUSED_RECORDING" => Self::Paused,
            "STOPPED" => Self::Stopped,
            "TRANSITIONING" => Self::Transitioning,
            "NO_MEDIA_PRESENT" => Self::NoMedia,
            _ => Self::Unknown,
        }
    }
}

/// Position renvoyée par le lecteur.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Position {
    /// Adresse de la piste en cours : révèle un enchaînement fait par le lecteur.
    pub track_uri: String,
    pub elapsed: Option<f64>,
    pub duration: Option<f64>,
}

/// Pilote d'un lecteur réseau.
#[derive(Clone)]
pub struct Controller {
    renderer: Renderer,
    client: reqwest::Client,
}

impl Controller {
    pub fn new(renderer: Renderer) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(4))
            .build()
            .unwrap_or_default();
        Self { renderer, client }
    }

    pub fn renderer(&self) -> &Renderer {
        &self.renderer
    }

    pub async fn set_uri(&self, url: &str, metadata: &str) -> Result<(), String> {
        self.av("SetAVTransportURI", &[("CurrentURI", url), ("CurrentURIMetaData", metadata)]).await.map(drop)
    }

    /// Piste suivante, pour un enchaînement fait par le lecteur lui-même.
    pub async fn set_next_uri(&self, url: &str, metadata: &str) -> Result<(), String> {
        self.av("SetNextAVTransportURI", &[("NextURI", url), ("NextURIMetaData", metadata)]).await.map(drop)
    }

    pub async fn play(&self) -> Result<(), String> {
        self.av("Play", &[("Speed", "1")]).await.map(drop)
    }

    pub async fn pause(&self) -> Result<(), String> {
        self.av("Pause", &[]).await.map(drop)
    }

    pub async fn stop(&self) -> Result<(), String> {
        self.av("Stop", &[]).await.map(drop)
    }

    pub async fn seek(&self, seconds: f64) -> Result<(), String> {
        self.av("Seek", &[("Unit", "REL_TIME"), ("Target", &format_time(seconds))]).await.map(drop)
    }

    pub async fn position(&self) -> Result<Position, String> {
        let body = self.av("GetPositionInfo", &[]).await?;
        Ok(parse_position(&body))
    }

    pub async fn transport_state(&self) -> Result<TransportState, String> {
        let body = self.av("GetTransportInfo", &[]).await?;
        Ok(TransportState::parse(&xml::text(&body, "CurrentTransportState").unwrap_or_default()))
    }

    /// Volume 0–100, `None` si le lecteur ne le pilote pas.
    pub async fn volume(&self) -> Result<Option<u8>, String> {
        let Some(url) = self.renderer.rendering_control.as_deref() else { return Ok(None) };
        let body = call(&self.client, url, RENDERING_CONTROL, "GetVolume", &[("InstanceID", "0"), ("Channel", "Master")]).await?;
        Ok(xml::text(&body, "CurrentVolume").and_then(|v| v.parse::<u32>().ok()).map(|v| v.min(100) as u8))
    }

    pub async fn set_volume(&self, volume: u8) -> Result<(), String> {
        let Some(url) = self.renderer.rendering_control.as_deref() else { return Ok(()) };
        let v = volume.min(100).to_string();
        call(&self.client, url, RENDERING_CONTROL, "SetVolume", &[("InstanceID", "0"), ("Channel", "Master"), ("DesiredVolume", &v)])
            .await
            .map(drop)
    }

    /// Formats acceptés (`http-get:*:audio/flac:*`…), vide si le lecteur ne le dit pas.
    pub async fn sink_protocols(&self) -> Vec<String> {
        let Some(url) = self.renderer.connection_manager.as_deref() else { return Vec::new() };
        match call(&self.client, url, CONNECTION_MANAGER, "GetProtocolInfo", &[]).await {
            Ok(body) => xml::text(&body, "Sink")
                .unwrap_or_default()
                .split(',')
                .map(|p| p.trim().to_string())
                .filter(|p| !p.is_empty())
                .collect(),
            Err(_) => Vec::new(),
        }
    }

    async fn av(&self, action: &str, args: &[(&str, &str)]) -> Result<String, String> {
        let mut all = vec![("InstanceID", "0")];
        all.extend_from_slice(args);
        call(&self.client, &self.renderer.av_transport, AV_TRANSPORT, action, &all).await
    }
}

/// Envoie une action SOAP et rend le corps de la réponse.
pub(crate) async fn call(
    client: &reqwest::Client,
    url: &str,
    service: &str,
    action: &str,
    args: &[(&str, &str)],
) -> Result<String, String> {
    let response = client
        .post(url)
        .header("Content-Type", "text/xml; charset=\"utf-8\"")
        .header("SOAPAction", format!("\"{service}#{action}\""))
        .body(envelope(service, action, args))
        .send()
        .await
        .map_err(|e| format!("{action} : {e}"))?;
    let status = response.status();
    let body = response.text().await.map_err(|e| format!("{action} : {e}"))?;
    if status.is_success() {
        Ok(body)
    } else {
        Err(format!("{action} refusé ({status}) : {}", fault(&body)))
    }
}

fn envelope(service: &str, action: &str, args: &[(&str, &str)]) -> String {
    let mut body = String::new();
    for (name, value) in args {
        body.push_str(&format!("<{name}>{}</{name}>", xml::escape(value)));
    }
    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\
<s:Envelope xmlns:s=\"http://schemas.xmlsoap.org/soap/envelope/\" s:encodingStyle=\"http://schemas.xmlsoap.org/soap/encoding/\">\
<s:Body><u:{action} xmlns:u=\"{service}\">{body}</u:{action}></s:Body></s:Envelope>"
    )
}

/// Message d'une erreur UPnP : « 701 Transition not available ».
fn fault(body: &str) -> String {
    let code = xml::text(body, "errorCode").unwrap_or_default();
    let desc = xml::text(body, "errorDescription").unwrap_or_default();
    format!("{code} {desc}").trim().to_string()
}

fn parse_position(body: &str) -> Position {
    Position {
        track_uri: xml::text(body, "TrackURI").unwrap_or_default(),
        elapsed: xml::text(body, "RelTime").and_then(|t| parse_time(&t)),
        duration: xml::text(body, "TrackDuration").and_then(|t| parse_time(&t)),
    }
}

/// « H:MM:SS » ou « H:MM:SS.mmm » en secondes ; `None` pour `NOT_IMPLEMENTED`.
pub fn parse_time(s: &str) -> Option<f64> {
    let mut parts = s.trim().split(':');
    let h: f64 = parts.next()?.parse().ok()?;
    let m: f64 = parts.next()?.parse().ok()?;
    let sec: f64 = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some(h * 3600.0 + m * 60.0 + sec)
}

/// Secondes en « H:MM:SS », le format attendu par `Seek` et les métadonnées.
pub fn format_time(seconds: f64) -> String {
    let total = seconds.max(0.0).round() as u64;
    format!("{}:{:02}:{:02}", total / 3600, total / 60 % 60, total % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_an_escaped_envelope() {
        let e = envelope(AV_TRANSPORT, "SetAVTransportURI", &[("InstanceID", "0"), ("CurrentURI", "http://h/a?x=1&y=2")]);
        assert!(e.contains("<u:SetAVTransportURI xmlns:u=\"urn:schemas-upnp-org:service:AVTransport:1\">"));
        assert!(e.contains("<CurrentURI>http://h/a?x=1&amp;y=2</CurrentURI>"));
    }

    #[test]
    fn reads_position_and_times() {
        let body = "<s:Envelope><s:Body><u:GetPositionInfoResponse><Track>1</Track>\
            <TrackDuration>0:04:46</TrackDuration><TrackURI>http://10.0.0.121:8201/stream/ab12</TrackURI>\
            <RelTime>0:01:02.500</RelTime></u:GetPositionInfoResponse></s:Body></s:Envelope>";
        let p = parse_position(body);
        assert_eq!(p.track_uri, "http://10.0.0.121:8201/stream/ab12");
        assert_eq!(p.elapsed, Some(62.5));
        assert_eq!(p.duration, Some(286.0));
        assert_eq!(parse_time("NOT_IMPLEMENTED"), None);
        assert_eq!(format_time(3725.4), "1:02:05");
    }

    #[test]
    fn reads_transport_state_and_faults() {
        assert_eq!(TransportState::parse("PAUSED_PLAYBACK"), TransportState::Paused);
        assert_eq!(TransportState::parse("WEIRD"), TransportState::Unknown);
        let f = "<s:Fault><detail><UPnPError><errorCode>701</errorCode><errorDescription>Transition not available</errorDescription></UPnPError></detail></s:Fault>";
        assert_eq!(fault(f), "701 Transition not available");
    }
}
