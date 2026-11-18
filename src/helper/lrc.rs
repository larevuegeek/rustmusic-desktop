//! Paroles synchronisées au format LRC (`[mm:ss.xx] texte`).

/// Lignes triées par instant, décalage `[offset:]` appliqué.
pub fn parse_lrc(content: &str) -> Vec<(f64, String)> {
    let mut offset = 0f64;
    let mut lines = Vec::new();
    for raw_value in content.lines() {
        let line = raw_value.trim();
        if line.is_empty() {
            continue;
        }
        let low = line.to_lowercase();
        if let Some(v) = low.strip_prefix("[offset:").and_then(|r| r.strip_suffix(']')) {
            offset = v.trim().parse().unwrap_or(0.);
            continue;
        }
        if ["[ti:", "[ar:", "[al:", "[au:", "[length:", "[by:", "[re:", "[ve:"].iter().any(|m| low.starts_with(m)) {
            continue;
        }
        // Tous les horodatages en tête, puis le texte.
        let mut rest = line;
        let mut instants = Vec::new();
        while let Some(r) = rest.strip_prefix('[') {
            let Some(end) = r.find(']') else { break };
            let mark = &r[..end];
            let Some((m, s)) = mark.split_once(':') else { break };
            let (sec, frac) = s.split_once(['.', ':']).unwrap_or((s, "0"));
            let (Ok(m), Ok(sec)) = (m.parse::<f64>(), sec.parse::<f64>()) else { break };
            let frac_ms = if frac.len() == 3 { frac.parse::<f64>().unwrap_or(0.) } else { format!("{frac:0<2}")[..2].parse::<f64>().unwrap_or(0.) * 10. };
            instants.push(m * 60_000. + sec * 1000. + frac_ms);
            rest = &r[end + 1..];
        }
        if instants.is_empty() {
            continue;
        }
        let text = rest.trim().to_string();
        for t in instants {
            lines.push(((t + offset).max(0.), text.clone()));
        }
    }
    lines.sort_by(|a, b| a.0.total_cmp(&b.0));
    lines
}
