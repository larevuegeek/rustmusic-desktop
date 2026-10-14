//! SOAP control endpoint : `POST /control/<service>`.
//!
//! This module is a thin dispatcher : it routes the SOAP action to the right
//! ContentDirectory / ConnectionManager handler. Envelope construction and
//! body parsing live in [`crate::core::dlna_server::soap`].

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::response::Response;

use crate::core::dlna_server::content_directory::browse::handle_browse;
use crate::core::dlna_server::http::router::AppState;
use crate::core::dlna_server::library::provider::LibraryProvider;
use crate::core::dlna_server::soap::{
    browse_response, parse_browse_request, soap_fault, soap_response,
};

/// `POST /control/<service>` — SOAP dispatcher.
pub async fn soap_control(
    Path(service): Path<String>,
    State(state): State<AppState>,
    headers: HeaderMap,
    body: String,
) -> Response {
    let action = headers
        .get("SOAPAction")
        .or_else(|| headers.get("soapaction"))
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .trim_matches('"');

    log::debug!(
        "SOAP {} POST /control/{} ({} bytes)",
        action, service, body.len()
    );

    match service.as_str() {
        "ContentDirectory" => {
            let base = state.base_url_for(headers.get("host").and_then(|v| v.to_str().ok()));
            content_directory(&state, &base, action, &body).await
        }
        "ConnectionManager" => connection_manager(action),
        _ => soap_fault(401, "Unknown service"),
    }
}

// ─── ContentDirectory ──────────────────────────────────────────────────

async fn content_directory(state: &AppState, base: &str, action: &str, body: &str) -> Response {
    if action.contains("#Browse") {
        handle_browse_action(state.provider.clone(), base, body).await
    } else if action.contains("#GetSearchCapabilities") {
        soap_response("GetSearchCapabilities", "<SearchCaps></SearchCaps>")
    } else if action.contains("#GetSortCapabilities") {
        soap_response("GetSortCapabilities", "<SortCaps></SortCaps>")
    } else if action.contains("#GetSystemUpdateID") {
        soap_response("GetSystemUpdateID", "<Id>1</Id>")
    } else {
        soap_fault(401, "Invalid Action")
    }
}

async fn handle_browse_action(
    provider: Arc<dyn LibraryProvider>,
    server_base_url: &str,
    body: &str,
) -> Response {
    let req = parse_browse_request(body);

    match handle_browse(provider, server_base_url, &req).await {
        Ok(r) => browse_response(&r.didl_xml, r.number_returned, r.total_matches, r.update_id),
        Err(e) => {
            log::error!("Browse handler error: {}", e);
            soap_fault(501, "Action Failed")
        }
    }
}

// ─── ConnectionManager (minimal stubs — most amps just probe these) ────

fn connection_manager(action: &str) -> Response {
    if action.contains("#GetProtocolInfo") {
        soap_response(
            "GetProtocolInfo",
            "<Source>http-get:*:audio/mpeg:*,http-get:*:audio/flac:*,http-get:*:audio/wav:*,http-get:*:audio/aiff:*,http-get:*:audio/mp4:*,http-get:*:audio/ogg:*</Source><Sink></Sink>",
        )
    } else if action.contains("#GetCurrentConnectionIDs") {
        soap_response("GetCurrentConnectionIDs", "<ConnectionIDs>0</ConnectionIDs>")
    } else if action.contains("#GetCurrentConnectionInfo") {
        soap_response(
            "GetCurrentConnectionInfo",
            "<RcsID>-1</RcsID>\
             <AVTransportID>-1</AVTransportID>\
             <ProtocolInfo></ProtocolInfo>\
             <PeerConnectionManager></PeerConnectionManager>\
             <PeerConnectionID>-1</PeerConnectionID>\
             <Direction>Output</Direction>\
             <Status>OK</Status>",
        )
    } else {
        soap_fault(401, "Invalid Action")
    }
}
