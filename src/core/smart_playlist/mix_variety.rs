//! Variété des mix : un genre trop pauvre n'en fait pas un, un artiste ne le remplit pas.

/// En dessous, un genre ne fait pas un mix : ce serait un album.
pub const MIN_ARTISTS: i64 = 5;
pub const MIN_TRACKS: i64 = 30;

/// Au plus `ceiling` titres par artiste, dans l'ordre reçu (déjà tiré au hasard), puis `n` au total.
pub fn diversify<T>(tracks_list: Vec<T>, n: usize, artist_name: impl Fn(&T) -> Option<String>) -> Vec<T> {
    let distinct = tracks_list.iter().filter_map(&artist_name).collect::<std::collections::HashSet<_>>().len().max(1);
    let ceiling = 3.max(n.div_ceil(distinct));
    let mut by: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut output = Vec::with_capacity(n);
    for p in tracks_list {
        if output.len() >= n {
            break;
        }
        if let Some(a) = artist_name(&p) {
            let c = by.entry(a).or_default();
            if *c >= ceiling {
                continue;
            }
            *c += 1;
        }
        output.push(p);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cap_per_artist() {
        let tracks_list: Vec<(u32, &str)> = (0..30).map(|i| (i, if i < 20 { "A" } else { "B" })).collect();
        let r = diversify(tracks_list, 10, |p| Some(p.1.to_string()));
        assert_eq!(r.len(), 10);
        assert_eq!(r.iter().filter(|p| p.1 == "A").count(), 5);
    }

}
