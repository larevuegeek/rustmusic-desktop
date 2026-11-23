//! Paroles synchronisées au format LRC (`[mm:ss.xx] texte`).

use crate::models::player::LyricLine;

/// Lignes triées par instant, décalage `[offset:]` appliqué. Une ligne sans horodatage
/// est rattachée à la ligne horodatée qui la précède, comme traduction.
pub fn parse_lrc(content: &str) -> Vec<LyricLine> {
    let mut offset = 0f64;
    let mut lines: Vec<LyricLine> = Vec::new();
    // Lignes créées par le dernier horodatage lu, qui reçoivent la traduction.
    let mut last = 0..0;
    for raw_value in content.lines() {
        let line = raw_value.trim();
        if line.is_empty() {
            last = 0..0;
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
            // Balise inconnue (`[id:…]`) : ignorée ; texte brut : traduction de la ligne d'avant.
            if !line.starts_with('[') {
                for l in &mut lines[last.clone()] {
                    l.translation.push(line.to_string());
                }
            }
            continue;
        }
        let text = rest.trim().to_string();
        let start = lines.len();
        for t in instants {
            lines.push(LyricLine { at: (t + offset).max(0.), text: text.clone(), translation: Vec::new() });
        }
        last = start..lines.len();
    }
    lines.sort_by(|a, b| a.at.total_cmp(&b.at));
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(lines: &[LyricLine]) -> Vec<(f64, &str, Vec<&str>)> {
        lines.iter().map(|l| (l.at, l.text.as_str(), l.translation.iter().map(String::as_str).collect())).collect()
    }

    #[test]
    fn an_untimed_line_is_the_translation_of_the_line_before() {
        let lrc = "[ti:Titre]\n[00:12.34] Original lyric line\nLigne traduite sans horodatage\n[00:16.80] Next original lyric line\nLigne traduite suivante\n";
        assert_eq!(texts(&parse_lrc(lrc)), vec![
            (12_340., "Original lyric line", vec!["Ligne traduite sans horodatage"]),
            (16_800., "Next original lyric line", vec!["Ligne traduite suivante"]),
        ]);
    }

    #[test]
    fn several_untimed_lines_and_repeated_timestamps_share_the_translation() {
        let lrc = "[00:01.00][00:30.00] Refrain\nChorus\nCoro\n[00:10.00] Couplet\n";
        assert_eq!(texts(&parse_lrc(lrc)), vec![
            (1_000., "Refrain", vec!["Chorus", "Coro"]),
            (10_000., "Couplet", vec![]),
            (30_000., "Refrain", vec!["Chorus", "Coro"]),
        ]);
    }

    #[test]
    fn text_before_any_timestamp_after_a_blank_line_or_in_unknown_tags_is_ignored() {
        let lrc = "Texte d'en-tête\n[id:abc]\n[00:05.00] Un\n\nTexte isolé\n[00:06.00] Deux\n[#commentaire]\n";
        assert_eq!(texts(&parse_lrc(lrc)), vec![(5_000., "Un", vec![]), (6_000., "Deux", vec![])]);
    }

    #[test]
    fn the_offset_shifts_every_line() {
        let lrc = "[offset:500]\n[00:01.00] Un\nOne\n";
        assert_eq!(texts(&parse_lrc(lrc)), vec![(1_500., "Un", vec!["One"])]);
    }
}
