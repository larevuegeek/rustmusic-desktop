//! Tirages au hasard sans dépendance : graine tirée de l'horloge, xorshift64.

/// `n` éléments tirés au hasard (Fisher-Yates partiel).
pub fn random_sample<T: Clone>(items: &[T], n: usize) -> Vec<T> {
    let mut copy = items.to_vec();
    let how_many = n.min(copy.len());
    let mut seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9E37_79B9_7F4A_7C15)
        | 1;
    for i in 0..how_many {
        // xorshift64
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let j = i + (seed % (copy.len() - i) as u64) as usize;
        copy.swap(i, j);
    }
    copy.truncate(how_many);
    copy
}
