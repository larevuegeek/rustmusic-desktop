//! Sonos : dans un groupe, seule l'enceinte coordinatrice accepte les commandes de lecture.

use super::control;
use super::discovery::{self, Renderer};
use crate::helper::xml;

const ZONE_GROUP_TOPOLOGY: &str = "urn:schemas-upnp-org:service:ZoneGroupTopology:1";

/// L'enceinte qui pilote le groupe de `renderer`, si ce n'est pas elle-même.
pub async fn coordinator(renderer: &Renderer) -> Option<Renderer> {
    let url = renderer.group_topology.as_deref()?;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .ok()?;
    let body = control::call(&client, url, ZONE_GROUP_TOPOLOGY, "GetZoneGroupState", &[]).await.ok()?;
    let state = xml::text(&body, "ZoneGroupState")?;
    let location = coordinator_location(&state, &speaker_id(&renderer.id))?;
    let found = discovery::fetch(&location).await?;
    log::info!("📡 {} fait partie d'un groupe : lecture confiée à {}", renderer.name, found.name);
    Some(found)
}

/// `uuid:RINCON_xxx_MR` → `RINCON_xxx`, l'identifiant qu'utilisent les groupes.
fn speaker_id(udn: &str) -> String {
    let id = udn.strip_prefix("uuid:").unwrap_or(udn);
    id.strip_suffix("_MR").or_else(|| id.strip_suffix("_MS")).unwrap_or(id).to_string()
}

/// Adresse de description de la coordinatrice du groupe de `speaker`, si c'en est une autre.
fn coordinator_location(state: &str, speaker: &str) -> Option<String> {
    for (group_tag, members) in xml::tagged(state, "ZoneGroup") {
        let coordinator = xml::attr(group_tag, "Coordinator")?;
        let members = xml::tagged(members, "ZoneGroupMember");
        if !members.iter().any(|(tag, _)| xml::attr(tag, "UUID").as_deref() == Some(speaker)) {
            continue;
        }
        if coordinator == speaker {
            return None;
        }
        return members
            .iter()
            .find(|(tag, _)| xml::attr(tag, "UUID").as_deref() == Some(coordinator.as_str()))
            .and_then(|(tag, _)| xml::attr(tag, "Location"));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const STATE: &str = r#"<ZoneGroupState><ZoneGroups>
  <ZoneGroup Coordinator="RINCON_AAA01400" ID="RINCON_AAA01400:12">
    <ZoneGroupMember UUID="RINCON_AAA01400" Location="http://192.168.1.60:1400/xml/device_description.xml" ZoneName="Salon"/>
    <ZoneGroupMember UUID="RINCON_BBB01400" Location="http://192.168.1.61:1400/xml/device_description.xml" ZoneName="Cuisine"/>
  </ZoneGroup>
  <ZoneGroup Coordinator="RINCON_CCC01400" ID="RINCON_CCC01400:3">
    <ZoneGroupMember UUID="RINCON_CCC01400" Location="http://192.168.1.62:1400/xml/device_description.xml" ZoneName="Bureau"/>
  </ZoneGroup>
</ZoneGroups></ZoneGroupState>"#;

    #[test]
    fn sends_a_grouped_speaker_to_its_coordinator() {
        assert_eq!(speaker_id("uuid:RINCON_BBB01400_MR"), "RINCON_BBB01400");
        assert_eq!(
            coordinator_location(STATE, "RINCON_BBB01400").as_deref(),
            Some("http://192.168.1.60:1400/xml/device_description.xml")
        );
        // La coordinatrice elle-même, ou une enceinte seule : rien à rediriger.
        assert_eq!(coordinator_location(STATE, "RINCON_AAA01400"), None);
        assert_eq!(coordinator_location(STATE, "RINCON_CCC01400"), None);
        assert_eq!(coordinator_location(STATE, "RINCON_ZZZ"), None);
    }
}
