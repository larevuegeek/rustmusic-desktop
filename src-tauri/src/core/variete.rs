//! Variété des mix : un genre trop pauvre n'en fait pas un, un artiste ne le remplit pas.

/// En dessous, un genre ne fait pas un mix : ce serait un album.
pub const MIN_ARTISTES: i64 = 5;
pub const MIN_TITRES: i64 = 30;

/// Au plus `plafond` titres par artiste, dans l'ordre reçu (déjà tiré au hasard), puis `n` au total.
pub fn diversifier<T>(pistes: Vec<T>, n: usize, artiste: impl Fn(&T) -> Option<String>) -> Vec<T> {
    let distincts = pistes.iter().filter_map(&artiste).collect::<std::collections::HashSet<_>>().len().max(1);
    let plafond = 3.max(n.div_ceil(distincts));
    let mut par: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut sortie = Vec::with_capacity(n);
    for p in pistes {
        if sortie.len() >= n {
            break;
        }
        if let Some(a) = artiste(&p) {
            let c = par.entry(a).or_default();
            if *c >= plafond {
                continue;
            }
            *c += 1;
        }
        sortie.push(p);
    }
    sortie
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plafond_par_artiste() {
        let pistes: Vec<(u32, &str)> = (0..30).map(|i| (i, if i < 20 { "A" } else { "B" })).collect();
        let r = diversifier(pistes, 10, |p| Some(p.1.to_string()));
        assert_eq!(r.len(), 10);
        assert_eq!(r.iter().filter(|p| p.1 == "A").count(), 5);
    }

}
