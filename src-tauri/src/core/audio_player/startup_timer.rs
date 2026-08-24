//! Où passent les millisecondes entre le clic et le premier échantillon.
//!
//! # Pourquoi mesurer plutôt que supposer
//! Le démarrage traverse une demi-douzaine d'étapes — lecture du fichier,
//! analyse du conteneur, énumération des périphériques, négociation du format,
//! ouverture du flux, pré-remplissage du tampon — et chacune est un suspect
//! plausible. Sans chiffres, on optimise la mauvaise.
//!
//! Le préchargement en RAM des fichiers réseau, par exemple, semblait le
//! coupable évident : mesuré, il rend 26 Mo en 0,24 seconde. Ce n'était pas lui.
//!
//! # Ça ne coûte rien quand tout va bien
//! Un `Instant::now()` par étape, et une seule ligne écrite. Le détail ne passe
//! en avertissement — donc dans le fichier de journal — que si le démarrage a
//! réellement traîné : un démarrage lent **est** une anomalie, et c'est
//! précisément la trace qu'on cherchera après coup.

use std::time::Instant;

/// Seuil au-delà duquel un démarrage mérite d'être signalé.
///
/// Un démarrage sain tient sous les deux cents millisecondes. Un demi-seconde
/// est déjà perceptible : c'est le délai après lequel on se demande si le clic
/// a été pris en compte.
const SLOW_START_MS: u128 = 500;

/// Chronomètre les étapes du démarrage de la lecture.
pub struct StartupTimer {
    origin: Instant,
    last: Instant,
    phases: Vec<(&'static str, u128)>,
}

impl StartupTimer {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            origin: now,
            last: now,
            phases: Vec::with_capacity(8),
        }
    }

    /// Clôt l'étape en cours et la nomme.
    pub fn mark(&mut self, phase: &'static str) {
        let now = Instant::now();
        self.phases
            .push((phase, now.duration_since(self.last).as_millis()));
        self.last = now;
    }

    /// Total écoulé depuis la création.
    pub fn total_ms(&self) -> u128 {
        self.origin.elapsed().as_millis()
    }

    /// Rend le détail sous forme lisible, la plus lente d'abord.
    ///
    /// L'ordre décroissant n'est pas cosmétique : sur six étapes, on veut lire
    /// le coupable en premier, pas le chercher dans une liste chronologique.
    pub fn breakdown(&self) -> String {
        let mut sorted: Vec<&(&str, u128)> = self.phases.iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        sorted
            .iter()
            .map(|(name, ms)| format!("{name} {ms} ms"))
            .collect::<Vec<_>>()
            .join(" · ")
    }

    /// Écrit le bilan, et le signale s'il a traîné.
    ///
    /// Emprunte au lieu de consommer : l'appel se fait parfois depuis une
    /// boucle — au premier tampon écrit, par exemple — où le compilateur ne
    /// peut pas prouver qu'il n'aura lieu qu'une fois.
    pub fn finish(&self, backend: &str) {
        let total = self.total_ms();
        let detail = self.breakdown();

        if total >= SLOW_START_MS {
            // En avertissement : le fichier de journal ne retient pas les
            // informations, et c'est justement cette ligne qu'on voudra relire.
            log::warn!("⏱ Démarrage lent : {total} ms via {backend} — {detail}");
        } else {
            log::debug!("⏱ Démarrage : {total} ms via {backend} — {detail}");
        }
    }
}

impl Default for StartupTimer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_etapes_sont_nommees_et_mesurees() {
        let mut timer = StartupTimer::new();
        std::thread::sleep(std::time::Duration::from_millis(12));
        timer.mark("ouverture");
        timer.mark("analyse");

        assert_eq!(timer.phases.len(), 2);
        assert!(timer.phases[0].1 >= 10, "la première étape doit porter l'attente");
        // La seconde suit immédiatement : elle ne doit pas hériter du temps
        // de la première.
        assert!(timer.phases[1].1 < 10);
    }

    #[test]
    fn le_detail_montre_la_plus_lente_en_premier() {
        let mut timer = StartupTimer::new();
        timer.phases.push(("rapide", 3));
        timer.phases.push(("lente", 900));
        timer.phases.push(("moyenne", 40));

        let detail = timer.breakdown();
        assert!(detail.starts_with("lente 900 ms"), "détail : {detail}");
        assert!(detail.ends_with("rapide 3 ms"), "détail : {detail}");
    }

    #[test]
    fn un_demarrage_sans_etape_ne_casse_rien() {
        let timer = StartupTimer::new();
        assert_eq!(timer.breakdown(), "");
    }

    #[test]
    fn le_total_couvre_toutes_les_etapes() {
        let mut timer = StartupTimer::new();
        std::thread::sleep(std::time::Duration::from_millis(8));
        timer.mark("a");
        std::thread::sleep(std::time::Duration::from_millis(8));
        timer.mark("b");

        let somme: u128 = timer.phases.iter().map(|(_, ms)| ms).sum();
        // Le total mesure depuis l'origine ; il ne peut pas être inférieur à la
        // somme des étapes.
        assert!(timer.total_ms() >= somme);
        assert!(timer.total_ms() >= 16);
    }
}
