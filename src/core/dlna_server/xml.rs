//! XML helpers shared across the DLNA server (DIDL builder, SOAP envelope,
//! body parsing).

/// Escape `<`, `>`, `&` for XML element text content.
pub fn xml_text(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Escape characters for XML attribute values (also escapes `"`).
pub fn xml_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Extract the text content of `<Tag>...</Tag>` (first occurrence,
/// case-sensitive). Returns `None` if not found.
///
/// This is a deliberately simple matcher : DLNA SOAP bodies are extremely
/// predictable, so we avoid pulling a full XML parser dependency. Not safe
/// for arbitrary XML (e.g. nested same-name tags would confuse it).
pub fn extract_tag(body: &str, tag: &str) -> Option<String> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    let start = body.find(&open)? + open.len();
    let end = body[start..].find(&close)? + start;
    Some(body[start..end].trim().to_string())
}
