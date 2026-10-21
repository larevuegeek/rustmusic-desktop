//! Lecteur XML minimal pour les descriptions UPnP et les réponses SOAP.
//!
//! Ce n'est pas un analyseur général : il retrouve des éléments par leur nom
//! local (préfixe ignoré, `<u:Volume>` comme `<Volume>`), gère l'imbrication
//! d'éléments du même nom, saute commentaires, CDATA et instructions, et décode
//! les entités. C'est tout ce qu'exigent les appareils réseau.

/// Balise trouvée à une position : nom local, ouvrante ou fermante, auto-fermée.
struct Tag<'a> {
    local: &'a str,
    closing: bool,
    self_closing: bool,
    /// Index juste après le `>`.
    end: usize,
}

/// Lit la balise qui commence à `start` (un `<`). `None` pour un commentaire,
/// une CDATA, une instruction ou une déclaration : l'appelant saute `skip`.
fn read_tag(xml: &str, start: usize) -> Result<Tag<'_>, usize> {
    let rest = &xml[start..];
    for (open, close) in [("<!--", "-->"), ("<![CDATA[", "]]>"), ("<?", "?>"), ("<!", ">")] {
        if rest.starts_with(open) {
            let skip = rest.find(close).map_or(xml.len(), |i| start + i + close.len());
            return Err(skip);
        }
    }
    let Some(gt) = rest.find('>') else { return Err(xml.len()) };
    let inner = &rest[1..gt];
    let closing = inner.starts_with('/');
    let self_closing = !closing && inner.ends_with('/');
    let name = inner.trim_start_matches('/').trim_end_matches('/');
    let name = name.split(|c: char| c.is_whitespace()).next().unwrap_or("");
    let local = name.rsplit(':').next().unwrap_or(name);
    Ok(Tag { local, closing, self_closing, end: start + gt + 1 })
}

/// Contenu brut des éléments `name` les plus extérieurs, dans l'ordre.
/// Un élément imbriqué du même nom reste à l'intérieur de son parent.
pub fn elements<'a>(xml: &'a str, name: &str) -> Vec<&'a str> {
    tagged(xml, name).into_iter().map(|(_, inner)| inner).collect()
}

/// Comme [`elements`], avec la balise ouvrante de chacun (pour ses attributs).
pub fn tagged<'a>(xml: &'a str, name: &str) -> Vec<(&'a str, &'a str)> {
    let mut out = Vec::new();
    // Balise ouvrante de l'élément en cours.
    let mut open_tag: &str = "";
    let mut pos = 0;
    // (début du contenu, profondeur) de l'élément en cours.
    let mut open: Option<(usize, usize)> = None;

    while let Some(rel) = xml[pos..].find('<') {
        let start = pos + rel;
        match read_tag(xml, start) {
            Err(skip) => pos = skip,
            Ok(tag) => {
                pos = tag.end;
                if tag.local != name {
                    continue;
                }
                match (&mut open, tag.closing, tag.self_closing) {
                    (None, false, true) => out.push((&xml[start..tag.end], "")),
                    (None, false, false) => {
                        open_tag = &xml[start..tag.end];
                        open = Some((tag.end, 1));
                    }
                    (Some((_, depth)), false, false) => *depth += 1,
                    (Some((content, depth)), true, _) => {
                        *depth -= 1;
                        if *depth == 0 {
                            out.push((open_tag, &xml[*content..start]));
                            open = None;
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    out
}

/// Valeur décodée de l'attribut `name` d'une balise ouvrante (`<a x="1">`).
pub fn attr(tag: &str, name: &str) -> Option<String> {
    let mut rest = tag;
    while let Some(i) = rest.find(name) {
        let before_ok = i == 0 || rest[..i].ends_with(|c: char| c.is_whitespace());
        let after = rest[i + name.len()..].trim_start();
        if before_ok {
            if let Some(after_eq) = after.strip_prefix('=') {
                let after_eq = after_eq.trim_start();
                let quote = after_eq.chars().next()?;
                if quote == '"' || quote == '\'' {
                    let value = &after_eq[1..];
                    let end = value.find(quote)?;
                    return Some(decode(&value[..end]));
                }
            }
        }
        rest = &rest[i + name.len()..];
    }
    None
}

/// Contenu brut du premier élément `name`.
pub fn first<'a>(xml: &'a str, name: &str) -> Option<&'a str> {
    elements(xml, name).into_iter().next()
}

/// Texte du premier élément `name`, décodé et sans espaces autour.
pub fn text(xml: &str, name: &str) -> Option<String> {
    first(xml, name).map(|raw| decode(raw).trim().to_string())
}

/// Retire les éléments `name` (et leur contenu) : isole un appareil de ses sous-appareils.
pub fn without<'a>(xml: &'a str, name: &str) -> String {
    let mut out = String::with_capacity(xml.len());
    let mut rest = xml;
    for inner in elements(xml, name) {
        // `inner` est une tranche de `xml` : on coupe de la balise ouvrante à la fermante.
        let content_start = inner.as_ptr() as usize - xml.as_ptr() as usize;
        let content_end = content_start + inner.len();
        let before = &xml[..content_start];
        let open_start = before.rfind('<').unwrap_or(content_start);
        let close_end = xml[content_end..].find('>').map_or(xml.len(), |i| content_end + i + 1);
        let offset = rest.as_ptr() as usize - xml.as_ptr() as usize;
        if open_start >= offset {
            out.push_str(&xml[offset..open_start]);
            rest = &xml[close_end..];
        }
    }
    out.push_str(rest);
    out
}

/// Décode les entités et déballe les CDATA.
pub fn decode(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix("<![CDATA[") {
            let end = after.find("]]>").unwrap_or(after.len());
            out.push_str(&after[..end]);
            rest = after.get(end + 3..).unwrap_or("");
            continue;
        }
        let c = rest.chars().next().unwrap();
        if c == '&' {
            if let Some(semi) = rest.find(';').filter(|&i| i <= 10) {
                let entity = &rest[1..semi];
                let decoded = match entity {
                    "lt" => Some('<'),
                    "gt" => Some('>'),
                    "amp" => Some('&'),
                    "quot" => Some('"'),
                    "apos" => Some('\''),
                    _ => entity
                        .strip_prefix("#x")
                        .or_else(|| entity.strip_prefix("#X"))
                        .and_then(|h| u32::from_str_radix(h, 16).ok())
                        .or_else(|| entity.strip_prefix('#').and_then(|d| d.parse().ok()))
                        .and_then(char::from_u32),
                };
                if let Some(ch) = decoded {
                    out.push(ch);
                    rest = &rest[semi + 1..];
                    continue;
                }
            }
        }
        out.push(c);
        rest = &rest[c.len_utf8()..];
    }
    out
}

/// Échappe un texte pour l'insérer dans du XML.
pub fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_by_local_name_and_decodes() {
        let xml = r#"<?xml version="1.0"?><s:Body><u:GetVolumeResponse xmlns:u="x">
            <CurrentVolume>23</CurrentVolume><Name>Rock &amp; Roll &#233;t&#xE9;</Name>
            <!-- <CurrentVolume>99</CurrentVolume> --></u:GetVolumeResponse></s:Body>"#;
        assert_eq!(text(xml, "CurrentVolume").as_deref(), Some("23"));
        assert_eq!(text(xml, "Name").as_deref(), Some("Rock & Roll été"));
        assert!(first(xml, "GetVolumeResponse").is_some());
    }

    #[test]
    fn keeps_nested_same_name_inside_parent() {
        let xml = "<root><device><t>A</t><deviceList><device><t>B</t></device></deviceList></device></root>";
        let outer = elements(xml, "device");
        assert_eq!(outer.len(), 1);
        assert_eq!(text(&without(outer[0], "deviceList"), "t").as_deref(), Some("A"));
        let inner = elements(first(outer[0], "deviceList").unwrap(), "device");
        assert_eq!(text(inner[0], "t").as_deref(), Some("B"));
    }

    #[test]
    fn unwraps_cdata_and_self_closing() {
        let xml = "<a><b/><c><![CDATA[<x> & y]]></c><b>2</b></a>";
        assert_eq!(elements(xml, "b"), vec!["", "2"]);
        assert_eq!(text(xml, "c").as_deref(), Some("<x> & y"));
    }

    #[test]
    fn reads_attributes_of_tagged_elements() {
        let xml = r#"<ZoneGroups><ZoneGroup Coordinator="RINCON_A" ID="g1"><ZoneGroupMember UUID="RINCON_A" ZoneName="Salon &amp; Cuisine"/></ZoneGroup></ZoneGroups>"#;
        let groups = tagged(xml, "ZoneGroup");
        assert_eq!(attr(groups[0].0, "Coordinator").as_deref(), Some("RINCON_A"));
        let members = tagged(groups[0].1, "ZoneGroupMember");
        assert_eq!(attr(members[0].0, "ZoneName").as_deref(), Some("Salon & Cuisine"));
        assert_eq!(attr(members[0].0, "UUID").as_deref(), Some("RINCON_A"));
        assert_eq!(attr(members[0].0, "Missing"), None);
    }

    #[test]
    fn escape_round_trips() {
        let s = r#"AC/DC "Back" <in> & 'Black'"#;
        assert_eq!(decode(&escape(s)), s);
    }
}
