//! SOAP envelope helpers shared by the HTTP control endpoint.
//!
//! Builds well-formed SOAP responses (success + fault) and parses the
//! Browse request body into a `BrowseRequest`. Kept transport-agnostic so
//! the HTTP layer (`http/control.rs`) only handles routing.

use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};

use crate::core::dlna_server::content_directory::browse::{BrowseFlag, BrowseRequest};
use crate::core::dlna_server::xml::{extract_tag, xml_text};

/// Build a `200 OK` SOAP envelope wrapping the given action's response payload.
pub fn soap_response(action: &str, inner_xml: &str) -> Response {
    let body = format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/" s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/">
<s:Body>
<u:{action}Response xmlns:u="urn:schemas-upnp-org:service:ContentDirectory:1">
{inner_xml}
</u:{action}Response>
</s:Body>
</s:Envelope>"#
    );
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/xml; charset=\"utf-8\"")],
        body,
    )
        .into_response()
}

/// Build a SOAP `<Fault>` envelope with a UPnPError code/description.
pub fn soap_fault(code: u16, description: &str) -> Response {
    let body = format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/" s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/">
<s:Body>
<s:Fault>
<faultcode>s:Client</faultcode>
<faultstring>UPnPError</faultstring>
<detail>
<UPnPError xmlns="urn:schemas-upnp-org:control-1-0">
<errorCode>{code}</errorCode>
<errorDescription>{description}</errorDescription>
</UPnPError>
</detail>
</s:Fault>
</s:Body>
</s:Envelope>"#
    );
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        [(header::CONTENT_TYPE, "text/xml; charset=\"utf-8\"")],
        body,
    )
        .into_response()
}

/// Wrap a Browse result (DIDL XML + counts) into the SOAP `<BrowseResponse>`
/// payload. The DIDL must be embedded *escaped* inside `<Result>`.
pub fn browse_response(
    didl_xml: &str,
    number_returned: u32,
    total_matches: u32,
    update_id: u32,
) -> Response {
    let inner = format!(
        "<Result>{}</Result>\
         <NumberReturned>{}</NumberReturned>\
         <TotalMatches>{}</TotalMatches>\
         <UpdateID>{}</UpdateID>",
        xml_text(didl_xml),
        number_returned,
        total_matches,
        update_id
    );
    soap_response("Browse", &inner)
}

/// Best-effort parser for the SOAP Browse body. Defaults are applied only
/// when a tag is missing (the DLNA spec requires all four, but real-world
/// clients omit `RequestedCount` to mean "no limit").
pub fn parse_browse_request(body: &str) -> BrowseRequest {
    let object_id = extract_tag(body, "ObjectID").unwrap_or_else(|| "0".to_string());
    let browse_flag = match extract_tag(body, "BrowseFlag").as_deref() {
        Some("BrowseMetadata") => BrowseFlag::BrowseMetadata,
        _ => BrowseFlag::BrowseDirectChildren,
    };
    let starting_index = extract_tag(body, "StartingIndex")
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(0);
    let requested_count = extract_tag(body, "RequestedCount")
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(0);

    BrowseRequest {
        object_id,
        browse_flag,
        starting_index,
        requested_count,
    }
}
