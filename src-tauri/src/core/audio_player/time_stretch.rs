//! Vitesse de lecture sans changer la hauteur (WSOLA) : des fenêtres de son qui se chevauchent,
//! recollées plus serrées (ou plus espacées), chacune calée sur la précédente pour éviter les raccords.
//! Pensé pour la voix (podcasts) ; tout est préalloué, rien n'est alloué dans le rappel audio.

use std::sync::atomic::{AtomicU32, Ordering};

/// Vitesse demandée (bits d'un f32) ; 1.0 = lecture normale, sans traitement.
static SPEED: AtomicU32 = AtomicU32::new(0x3F80_0000);

pub const MIN_SPEED: f32 = 0.5;
pub const MAX_SPEED: f32 = 3.0;

pub fn set_speed(speed: f32) {
    SPEED.store(speed.clamp(MIN_SPEED, MAX_SPEED).to_bits(), Ordering::Relaxed);
}

pub fn speed() -> f32 {
    f32::from_bits(SPEED.load(Ordering::Relaxed))
}

pub fn is_normal(speed: f32) -> bool {
    (speed - 1.0).abs() < 0.01
}

/// Étireur WSOLA, son entrelacé à `channels` canaux.
pub struct TimeStretcher {
    channels: usize,
    /// Fenêtre (trames), pas de synthèse (moitié) et marge de recherche.
    window: usize,
    hop: usize,
    tolerance: usize,
    hann: Vec<f32>,
    /// Son source pas encore consommé (entrelacé), et son mono pour la recherche des raccords.
    input: Vec<f32>,
    mono: Vec<f32>,
    /// Début nominal de la prochaine fenêtre et de la précédente retenue (trames, dans `input`).
    nominal: f64,
    previous: Option<usize>,
    /// Seconde moitié de la fenêtre précédente, déjà pondérée, en attente de chevauchement.
    tail: Vec<f32>,
    /// Son produit pas encore rendu.
    ready: Vec<f32>,
    scratch: Vec<f32>,
}

impl TimeStretcher {
    pub fn new(sample_rate: u32, channels: usize) -> Self {
        // ~42 ms de fenêtre, ±10 ms de recherche : le classique pour la parole.
        let window = ((sample_rate as usize * 42 / 1000) / 2 * 2).max(256);
        let hop = window / 2;
        let tolerance = (sample_rate as usize / 100).max(32);
        let hann = (0..window).map(|i| (0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / window as f64).cos()) as f32).collect();
        let cap = (window * 4 + tolerance * 4) * channels;
        TimeStretcher {
            channels,
            window,
            hop,
            tolerance,
            hann,
            input: Vec::with_capacity(cap * 4),
            mono: Vec::with_capacity(cap * 4 / channels),
            nominal: tolerance as f64,
            previous: None,
            tail: vec![0.0; hop * channels],
            ready: Vec::with_capacity(cap),
            scratch: vec![0.0; cap],
        }
    }

    /// Après un saut ou un changement de piste : tout repart de zéro.
    pub fn reset(&mut self) {
        self.input.clear();
        self.mono.clear();
        self.ready.clear();
        self.tail.fill(0.0);
        self.nominal = self.tolerance as f64;
        self.previous = None;
    }

    fn frames(&self) -> usize {
        self.input.len() / self.channels
    }

    /// Le début de fenêtre, près de `nominal`, qui prolonge le mieux la fenêtre précédente.
    fn best_start(&self, nominal: usize) -> usize {
        let lo = nominal.saturating_sub(self.tolerance);
        let hi = nominal + self.tolerance;
        let Some(prev) = self.previous else { return nominal };
        // Ce qui aurait suivi naturellement la fenêtre précédente.
        let natural = prev + self.hop;
        let mut best = (nominal, f32::MIN);
        // Pas de 2 sur le décalage et sur la corrélation : assez précis pour la voix, 4 fois moins coûteux.
        let reference = &self.mono[natural..natural + self.hop];
        let mut k = lo;
        while k <= hi {
            let candidate = &self.mono[k..k + self.hop];
            let score: f32 = candidate.iter().zip(reference).step_by(2).map(|(a, b)| a * b).sum();
            if score > best.1 {
                best = (k, score);
            }
            k += 2;
        }
        best.0
    }

    /// Remplit `out` à la vitesse donnée ; `pull` fournit le son source (rend le nombre d'échantillons écrits,
    /// 0 quand il n'y a plus rien). Rend le nombre d'échantillons utiles de `out`.
    pub fn process(&mut self, out: &mut [f32], speed: f32, mut pull: impl FnMut(&mut [f32]) -> usize) -> usize {
        let c = self.channels;
        let analysis = (self.hop as f64 * speed as f64).max(1.0);
        while self.ready.len() < out.len() {
            let nominal = self.nominal.round() as usize;
            let needed = nominal + self.tolerance + self.window + 1;
            // Il faut aussi la suite naturelle de la fenêtre précédente.
            let needed = needed.max(self.previous.map_or(0, |p| p + self.hop * 2 + 1));
            if self.frames() < needed {
                let want = ((needed - self.frames()) * c).min(self.scratch.len());
                let got = pull(&mut self.scratch[..want]);
                if got == 0 {
                    break;
                }
                self.input.extend_from_slice(&self.scratch[..got]);
                // Mono des trames désormais complètes (une lecture peut s'arrêter au milieu d'une trame).
                for f in self.mono.len()..self.frames() {
                    let v = self.input[f * c..f * c + c].iter().sum::<f32>() / c as f32;
                    self.mono.push(v);
                }
                continue;
            }
            let start = self.best_start(nominal);
            // Chevauchement-addition : première moitié + reste de la précédente, seconde moitié gardée.
            for f in 0..self.window {
                let w = self.hann[f];
                for ch in 0..c {
                    let v = self.input[(start + f) * c + ch] * w;
                    if f < self.hop {
                        self.ready.push(self.tail[f * c + ch] + v);
                    } else {
                        self.tail[(f - self.hop) * c + ch] = v;
                    }
                }
            }
            self.previous = Some(start);
            self.nominal += analysis;
            // Le son déjà dépassé est retiré (tout reste relatif au début de `input`).
            let keep_from = start.min(self.nominal as usize).saturating_sub(self.tolerance);
            if keep_from > self.window {
                self.input.drain(..keep_from * c);
                self.mono.drain(..keep_from);
                self.nominal -= keep_from as f64;
                self.previous = Some(start - keep_from);
            }
        }
        let n = self.ready.len().min(out.len());
        out[..n].copy_from_slice(&self.ready[..n]);
        self.ready.drain(..n);
        out[n..].fill(0.0);
        n
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Une sinusoïde stéréo à 440 Hz, en continu.
    fn tone(rate: u32) -> impl FnMut(&mut [f32]) -> usize {
        let mut t = 0usize;
        move |buf: &mut [f32]| {
            for frame in buf.chunks_mut(2) {
                let v = (std::f64::consts::TAU * 440.0 * t as f64 / rate as f64).sin() as f32 * 0.5;
                frame.fill(v);
                t += 1;
            }
            buf.len()
        }
    }

    /// Fréquence estimée par les passages à zéro.
    fn frequency(samples: &[f32], rate: u32) -> f64 {
        let mono: Vec<f32> = samples.chunks(2).map(|f| f[0]).collect();
        let crossings = mono.windows(2).filter(|w| w[0] <= 0.0 && w[1] > 0.0).count();
        crossings as f64 * rate as f64 / mono.len() as f64
    }

    #[test]
    fn faster_keeps_the_pitch_and_consumes_more() {
        let rate = 48_000;
        for speed in [1.5f32, 2.0, 0.75] {
            let mut st = TimeStretcher::new(rate, 2);
            let mut consumed = 0usize;
            let mut src = tone(rate);
            let mut out = vec![0.0f32; 4800 * 2];
            let mut all = Vec::new();
            for _ in 0..20 {
                let n = st.process(&mut out, speed, |b| {
                    let n = src(b);
                    consumed += n;
                    n
                });
                assert_eq!(n, out.len());
                all.extend_from_slice(&out);
            }
            // Hauteur gardée (après la mise en route).
            let f = frequency(&all[4800 * 2..], rate);
            assert!((f - 440.0).abs() < 10.0, "vitesse {speed} : {f} Hz");
            // Rythme : la source avance `speed` fois plus vite que la sortie.
            let ratio = consumed as f64 / all.len() as f64;
            assert!((ratio - speed as f64).abs() < 0.08, "vitesse {speed} : rapport {ratio}");
        }
    }

    #[test]
    fn steady_level_and_end_of_source() {
        let rate = 44_100;
        let mut st = TimeStretcher::new(rate, 2);
        let mut src = tone(rate);
        let mut out = vec![0.0f32; 4410 * 2];
        let mut all = Vec::new();
        for _ in 0..10 {
            st.process(&mut out, 1.25, &mut src);
            all.extend_from_slice(&out);
        }
        // Niveau constant (fenêtres de Hann à moitié chevauchées) : pas de creux ni de bosse.
        let peak = all[8820 * 2..].iter().fold(0.0f32, |m, v| m.max(v.abs()));
        assert!((peak - 0.5).abs() < 0.06, "crête {peak}");
        // Source épuisée : rendu partiel puis silence, sans panique.
        let mut left = 3000usize;
        let n = st.process(&mut out, 1.25, |b| {
            let n = b.len().min(left);
            left -= n;
            b[..n].fill(0.1);
            n
        });
        assert!(n < out.len());
        st.reset();
        assert_eq!(st.process(&mut out, 1.25, |_| 0), 0);
    }

    #[test]
    fn speed_is_clamped() {
        set_speed(9.0);
        assert_eq!(speed(), MAX_SPEED);
        set_speed(1.0);
        assert!(is_normal(speed()));
    }
}
